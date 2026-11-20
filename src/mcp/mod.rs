//! Dev-server MCP surface — the agent-facing half of `lagrange dev --mcp`.
//!
//! Scope and shape (P2 of the knowledge-base repositioning):
//!
//! - **Lifetime**: the MCP surface exists only inside `lagrange dev`. There
//!   is no standalone MCP service and no production write path; tools read
//!   the build artifacts (`knowledge.json`, `build_status.json`) and the
//!   docs tree, and `build.trigger` reuses the very build the watch loop
//!   runs — one index, never two.
//! - **Transport**: stdio, line-delimited JSON-RPC 2.0 (see [`protocol`]).
//!   The static HTTP server and the watch loop keep running unchanged
//!   alongside; stdout belongs to the protocol, all logging goes to stderr.
//! - **Citations**: a citation is the five-tuple
//!   `{document_id, lang, revision, span, anchor}` — exactly the fields
//!   every chunk in `knowledge.json` already carries. `docs.fetch` validates
//!   the tuple against BOTH the built index and the current file bytes and
//!   refuses stale revisions explicitly (never silently serving newer
//!   content).
//!
//! Concurrency posture: one MCP session per `--mcp` process; requests are
//! handled strictly in order on the session thread. The only cross-thread
//! shared resource is the build, serialized by [`McpState::build_lock`] so
//! a `build.trigger` cannot interleave with a watch-loop rebuild (both
//! clear and repopulate the output directory).

mod ops;
mod protocol;
mod query;
mod tools;
mod write;

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::site::BuildOptions;
use protocol::RpcError;

/// Latest MCP protocol revision this server speaks. Older revisions from
/// the official set are accepted by echoing the client's request back.
const PROTOCOL_VERSION: &str = "2025-06-18";
const KNOWN_PROTOCOL_VERSIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-06-18"];

/// Shared context for the MCP session, built once by `lagrange dev --mcp`.
pub struct McpState {
    /// Docs tree root (`BuildOptions::src`).
    pub src: PathBuf,
    /// Output directory (`BuildOptions::out`) — `knowledge.json` and
    /// `build_status.json` are read from here.
    pub out: PathBuf,
    /// Full options for `build.trigger` (same ones the dev command uses).
    pub build_opts: BuildOptions,
    /// `[mcp] write_paths` from lagrange.toml — empty means the whole docs
    /// root is writable (single-agent local-dev posture).
    pub write_paths: Vec<String>,
    /// Build mutex shared with the watch loop: the build clears and
    /// repopulates the output directory, so two concurrent builds would
    /// corrupt it. Poisoning is tolerated (dev server resilience).
    pub build_lock: Arc<Mutex<()>>,
}

impl McpState {
    /// Load `knowledge.json` from the output directory.
    fn knowledge(&self) -> anyhow::Result<crate::knowledge::KnowledgeBase> {
        crate::knowledge::read_json(&self.out)
    }
}

/// Names of the served tools, for startup logging.
pub fn tool_names() -> &'static [&'static str] {
    &tools::TOOL_NAMES
}

/// Run the stdio JSON-RPC loop until stdin closes.
///
/// Returns `Ok` on EOF (client disconnected — the dev server keeps
/// running); returns `Err` on I/O failure.
pub fn run_stdio(state: Arc<McpState>) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let state_for_cb = state.clone();
        if let Some(response) = protocol::process_line(&line, |method, params| {
            dispatch(&state_for_cb, method, params)
        }) {
            out.write_all(response.as_bytes())?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Method-level dispatch (one level above tools): the MCP handshake core.
fn dispatch(state: &McpState, method: &str, params: &Value) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(initialize_result(params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools::descriptors() })),
        "tools/call" => tools::call(state, params),
        // Notifications (initialized, cancelled, …) are observed here and
        // intentionally produce no output.
        _ if method.starts_with("notifications/") => Ok(Value::Null),
        _ => Err(RpcError::new(
            protocol::error_code::METHOD_NOT_FOUND,
            format!("unknown method `{method}`"),
        )),
    }
}

/// Build the `initialize` result, negotiating the protocol revision.
fn initialize_result(params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or(PROTOCOL_VERSION);
    let negotiated = if KNOWN_PROTOCOL_VERSIONS.contains(&requested) {
        requested
    } else {
        PROTOCOL_VERSION
    };
    json!({
        "protocolVersion": negotiated,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "lagrange", "version": env!("CARGO_PKG_VERSION") },
        "instructions": "lagrange dev docs surface. Search with docs.search, then fetch exact source text with docs.fetch by echoing the citation five-tuple (document_id, lang, revision, span, anchor). docs.fetch refuses mismatched revisions with a STALE error instead of returning different content. docs.write edits markdown under the docs root (scope may be narrowed by [mcp] write_paths in lagrange.toml); trigger build.trigger to rebuild, then docs.status reports the build outcome, freshness, missing translations and broken links."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_fixture() -> McpState {
        McpState {
            src: PathBuf::from("/nonexistent"),
            out: PathBuf::from("/nonexistent"),
            build_opts: BuildOptions {
                src: PathBuf::from("/nonexistent"),
                out: PathBuf::from("/nonexistent"),
                site_url: None,
                default_lang: None,
            },
            write_paths: Vec::new(),
            build_lock: Arc::new(Mutex::new(())),
        }
    }

    #[test]
    fn initialize_negotiates_known_versions_and_defaults_on_unknown() {
        for known in KNOWN_PROTOCOL_VERSIONS {
            let r = initialize_result(&json!({"protocolVersion": known}));
            assert_eq!(r["protocolVersion"], known);
        }
        let r = initialize_result(&json!({"protocolVersion": "1999-01-01"}));
        assert_eq!(r["protocolVersion"], PROTOCOL_VERSION);
        // No version at all → ours.
        let r = initialize_result(&json!({}));
        assert_eq!(r["protocolVersion"], PROTOCOL_VERSION);
    }

    #[test]
    fn initialize_result_shape_is_the_mcp_contract() {
        let r = initialize_result(&json!({"protocolVersion": PROTOCOL_VERSION}));
        assert_eq!(r["serverInfo"]["name"], "lagrange");
        assert_eq!(r["capabilities"]["tools"]["listChanged"], false);
        assert!(r["instructions"].as_str().unwrap().contains("STALE"));
    }

    #[test]
    fn dispatch_covers_the_handshake_core() {
        let st = state_fixture();
        assert_eq!(dispatch(&st, "ping", &json!({})).unwrap(), json!({}));
        let list = dispatch(&st, "tools/list", &json!({})).unwrap();
        let names: Vec<&str> = list["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "docs.search",
                "docs.fetch",
                "docs.tree",
                "docs.write",
                "docs.status",
                "build.trigger"
            ]
        );
        // notifications are absorbed without a response payload
        assert_eq!(
            dispatch(&st, "notifications/initialized", &json!({})).unwrap(),
            Value::Null
        );
        let err = dispatch(&st, "no/such/method", &json!({})).unwrap_err();
        assert_eq!(err.code, protocol::error_code::METHOD_NOT_FOUND);
    }

    #[test]
    fn tools_call_with_unknown_tool_is_invalid_params() {
        let st = state_fixture();
        let err = tools::call(&st, &json!({"name": "docs.nope", "arguments": {}})).unwrap_err();
        assert_eq!(err.code, protocol::error_code::INVALID_PARAMS);
    }
}

/// End-to-end tool tests against a REAL tempdir docs tree and a REAL
/// build: these exercise the whole read path (knowledge.json + source
/// slicing), not mocks.
#[cfg(test)]
mod tool_tests {
    use super::*;
    use serde_json::json;

    /// A tree with: one fully-translated document, one English-only
    /// document (missing zh-Hans), a working SUMMARY link and a broken one.
    fn fixture() -> (tempfile::TempDir, McpState) {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("docs");
        let out = dir.path().join("dist");
        std::fs::create_dir_all(src.join("en/guides")).unwrap();
        std::fs::create_dir_all(src.join("zh-Hans/guides")).unwrap();
        std::fs::write(
            src.join("en/SUMMARY.md"),
            "- [Quickstart](guides/quickstart.md)\n- [Nowhere](guides/missing.md)\n",
        )
        .unwrap();
        std::fs::write(
            src.join("en/index.md"),
            "---\ntitle: Home\n---\n# Home\n\nWelcome to the docs.\n",
        )
        .unwrap();
        std::fs::write(
            src.join("en/guides/quickstart.md"),
            "---\ntitle: Quickstart\nid: guides/quickstart\n---\n# Quickstart\n\nInstall the tool in your home directory.\n\n## Next steps\n\nConfigure it.\n",
        )
        .unwrap();
        std::fs::write(
            src.join("zh-Hans/guides/quickstart.md"),
            "---\ntitle: 快速开始\n---\n# 快速开始\n\n安装工具。\n\n## 下一步\n\n配置它。\n",
        )
        .unwrap();
        let opts = BuildOptions {
            src: src.clone(),
            out: out.clone(),
            site_url: None,
            default_lang: Some("en".to_string()),
        };
        crate::site::build_with_status(&opts).expect("fixture build");
        let state = McpState {
            src,
            out,
            build_opts: opts,
            write_paths: Vec::new(),
            build_lock: Arc::new(Mutex::new(())),
        };
        (dir, state)
    }

    fn call(state: &McpState, name: &str, args: Value) -> Value {
        tools::call(state, &json!({"name": name, "arguments": args})).unwrap()
    }

    #[test]
    fn search_ranks_heading_matches_above_body_matches() {
        let (_dir, st) = fixture();
        // Two candidates for the token "home": the index document's H1
        // HEADING is "Home" (heading match), and quickstart's body says
        // "…in your home directory." (body match). The heading match must
        // rank first — this is the pin that makes the 3x/1x weighting
        // mutation-detectable (swapping them flips the order).
        let r = call(&st, "docs.search", json!({"query": "home", "lang": "en"}));
        let hits = r["structuredContent"]["results"].as_array().unwrap();
        let order: Vec<&str> = hits.iter().map(|h| h["anchor"].as_str().unwrap()).collect();
        assert_eq!(
            order.first(),
            Some(&"home"),
            "heading match must outrank body match; got order: {order:?}"
        );
        assert!(
            order.contains(&"quickstart"),
            "the body match must also be a hit; got order: {order:?}"
        );
    }

    #[test]
    fn search_returns_chunk_citations_with_source_true_snippets() {
        let (_dir, st) = fixture();
        let r = call(
            &st,
            "docs.search",
            json!({"query": "quickstart", "lang": "en"}),
        );
        assert_eq!(r["isError"], false, "raw: {r}");
        let hits = r["structuredContent"]["results"].as_array().unwrap();
        assert!(!hits.is_empty());
        let top = &hits[0];
        assert_eq!(top["document_id"], "guides/quickstart");
        assert_eq!(top["lang"], "en");
        assert_eq!(top["anchor"], "quickstart");
        // Snippet truth: the reported span slices the REAL file bytes, and
        // the snippet is the projection of exactly that slice (integration
        // pin — an off-by-one in the span usage shows up here as a
        // mismatch, not only in docs.fetch).
        let span = &top["span"];
        let source = std::fs::read_to_string(st.src.join("en/guides/quickstart.md")).unwrap();
        let sliced = &source[span["start_byte"].as_u64().unwrap() as usize
            ..span["end_byte"].as_u64().unwrap() as usize];
        assert_eq!(
            top["snippet"].as_str().unwrap(),
            crate::search::truncate_chars(
                &crate::search::plain_text(sliced),
                crate::search::SNIPPET_CHARS
            ),
            "snippet must be the projection of the span's own bytes"
        );
        // …and the citation five-tuple is complete.
        for f in ["document_id", "lang", "revision", "span", "anchor"] {
            assert!(top.get(f).is_some_and(|v| !v.is_null()), "field {f}");
        }
    }

    #[test]
    fn search_supports_cjk_and_path_prefix_filters() {
        let (_dir, st) = fixture();
        let r = call(
            &st,
            "docs.search",
            json!({"query": "安装", "lang": "zh-Hans"}),
        );
        let hits = r["structuredContent"]["results"].as_array().unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0]["lang"], "zh-Hans");
        // path_prefix excludes the guides subtree → nothing matches.
        let r = call(
            &st,
            "docs.search",
            json!({"query": "quickstart", "path_prefix": "nope/"}),
        );
        assert!(r["structuredContent"]["results"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn fetch_by_anchor_returns_the_exact_cited_bytes() {
        let (_dir, st) = fixture();
        let s = call(&st, "docs.search", json!({"query": "Configure"}));
        let hit = &s["structuredContent"]["results"][0];
        let f = call(
            &st,
            "docs.fetch",
            json!({
                "document_id": hit["document_id"],
                "lang": hit["lang"],
                "revision": hit["revision"],
                "anchor": hit["anchor"],
            }),
        );
        assert_eq!(f["isError"], false, "raw: {f}");
        let cited = f["structuredContent"]["content"].as_str().unwrap();
        let source = std::fs::read_to_string(st.src.join("en/guides/quickstart.md")).unwrap();
        let span = &hit["span"];
        assert_eq!(
            cited,
            &source[span["start_byte"].as_u64().unwrap() as usize
                ..span["end_byte"].as_u64().unwrap() as usize],
            "fetched content must be the exact cited span"
        );
    }

    #[test]
    fn fetch_by_span_equivalence() {
        let (_dir, st) = fixture();
        let s = call(&st, "docs.search", json!({"query": "Install"}));
        let hit = &s["structuredContent"]["results"][0];
        let f = call(
            &st,
            "docs.fetch",
            json!({
                "document_id": hit["document_id"],
                "lang": hit["lang"],
                "revision": hit["revision"],
                "span": hit["span"],
            }),
        );
        assert_eq!(f["isError"], false, "raw: {f}");
    }

    /// THE acceptance line: a stale revision must be an explicit error,
    /// never silently-newer content. Gate 2: the index still holds the
    /// requested revision, the FILE moved on.
    #[test]
    fn fetch_reports_stale_when_the_file_changed_after_the_build() {
        let (_dir, st) = fixture();
        let s = call(&st, "docs.search", json!({"query": "Install"}));
        let hit = &s["structuredContent"]["results"][0];
        // Edit the file on disk WITHOUT rebuilding.
        let path = st.src.join("en/guides/quickstart.md");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("\nAppended after the build.\n");
        std::fs::write(&path, text).unwrap();
        let f = call(
            &st,
            "docs.fetch",
            json!({
                "document_id": hit["document_id"],
                "lang": hit["lang"],
                "revision": hit["revision"],
                "anchor": hit["anchor"],
            }),
        );
        assert_eq!(f["isError"], true, "stale must be isError");
        let msg = f["content"][0]["text"].as_str().unwrap();
        assert!(msg.starts_with("STALE:"), "message: {msg}");
        assert!(
            msg.contains("re-cite"),
            "message must say what to do: {msg}"
        );
        // And crucially: no content rode along.
        assert!(f.get("structuredContent").is_none());
    }

    /// Gate 1: the index side drifted (citation older or newer than the
    /// built knowledge) — requested revision != chunk revision.
    #[test]
    fn fetch_reports_stale_when_the_index_revision_differs() {
        let (_dir, st) = fixture();
        let s = call(&st, "docs.search", json!({"query": "Install"}));
        let hit = &s["structuredContent"]["results"][0];
        let bogus = "0".repeat(64);
        let f = call(
            &st,
            "docs.fetch",
            json!({
                "document_id": hit["document_id"],
                "lang": hit["lang"],
                "revision": bogus,
                "anchor": hit["anchor"],
            }),
        );
        assert_eq!(f["isError"], true);
        let msg = f["content"][0]["text"].as_str().unwrap();
        assert!(msg.starts_with("STALE:"), "message: {msg}");
        assert!(msg.contains("index"), "must name the index side: {msg}");
    }

    #[test]
    fn fetch_rejects_bad_references() {
        let (_dir, st) = fixture();
        let r = call(
            &st,
            "docs.fetch",
            json!({
                "document_id": "nope", "lang": "en", "revision": "0".repeat(64), "anchor": "x"
            }),
        );
        assert_eq!(r["isError"], true);
        assert!(r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("unknown document_id"));

        // both selector args → error; neither → error
        let s = call(&st, "docs.search", json!({"query": "Install"}));
        let hit = &s["structuredContent"]["results"][0];
        let r = call(
            &st,
            "docs.fetch",
            json!({
                "document_id": hit["document_id"], "lang": "en",
                "revision": hit["revision"], "anchor": "quickstart", "span": hit["span"]
            }),
        );
        assert!(r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not both"));
        let r = call(
            &st,
            "docs.fetch",
            json!({
                "document_id": hit["document_id"], "lang": "en", "revision": hit["revision"]
            }),
        );
        assert!(r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("required"));
    }

    #[test]
    fn tree_reports_freshness_and_missing_translations() {
        let (_dir, st) = fixture();
        let t = call(&st, "docs.tree", json!({}));
        let docs = t["structuredContent"]["documents"].as_array().unwrap();
        let index_doc = docs
            .iter()
            .find(|d| d["document_id"] == "index")
            .expect("index doc");
        assert_eq!(
            index_doc["missing_langs"],
            json!(["zh-Hans"]),
            "en-only document must flag the missing translation"
        );
        let qs = docs
            .iter()
            .find(|d| d["document_id"] == "guides/quickstart")
            .unwrap();
        assert_eq!(qs["translations"]["en"]["freshness"], "fresh");
        // Now edit the en translation without rebuilding → stale.
        let p = st.src.join("en/guides/quickstart.md");
        let mut text = std::fs::read_to_string(&p).unwrap();
        text.push_str("\nchanged\n");
        std::fs::write(&p, text).unwrap();
        let t = call(&st, "docs.tree", json!({}));
        let docs = t["structuredContent"]["documents"].as_array().unwrap();
        let qs = docs
            .iter()
            .find(|d| d["document_id"] == "guides/quickstart")
            .unwrap();
        assert_eq!(qs["translations"]["en"]["freshness"], "stale");
    }

    #[test]
    fn write_creates_files_and_respects_scope() {
        let (_dir, st) = fixture();
        // In-scope create works.
        let w = call(
            &st,
            "docs.write",
            json!({
                "path": "en/agent-notes.md",
                "content": "# Agent notes\n\nWritten by the tool.\n"
            }),
        );
        assert_eq!(w["isError"], false, "raw: {w}");
        assert_eq!(w["structuredContent"]["created"], true);
        assert!(st.src.join("en/agent-notes.md").is_file());

        // Narrowed scope rejects out-of-prefix writes with a clear message.
        let narrowed = McpState {
            src: st.src.clone(),
            out: st.out.clone(),
            build_opts: st.build_opts.clone(),
            write_paths: vec!["zh-Hans/agent".to_string()],
            build_lock: st.build_lock.clone(),
        };
        let w = call(
            &narrowed,
            "docs.write",
            json!({
                "path": "en/agent-notes-2.md",
                "content": "# x\n"
            }),
        );
        assert_eq!(w["isError"], true);
        let msg = w["content"][0]["text"].as_str().unwrap();
        assert!(
            msg.contains("outside the configured write scope"),
            "msg: {msg}"
        );
        assert!(
            msg.contains("write_paths"),
            "must name the config knob: {msg}"
        );
        // The matching prefix still works.
        let w = call(
            &narrowed,
            "docs.write",
            json!({
                "path": "zh-Hans/agent/notes.md",
                "content": "# x\n"
            }),
        );
        assert_eq!(w["isError"], false, "raw: {w}");
    }

    #[test]
    fn write_rejects_escapes_wrong_lang_and_non_markdown() {
        let (_dir, st) = fixture();
        for (path, needle) in [
            ("../escape.md", ".."),
            ("/abs.md", "tree-RELATIVE"),
            ("fr/nouveau.md", "not a language directory"),
            ("en/logo.png", "markdown file"),
        ] {
            let w = call(&st, "docs.write", json!({"path": path, "content": "# x\n"}));
            assert_eq!(w["isError"], true, "{path} must be rejected");
            let msg = w["content"][0]["text"].as_str().unwrap();
            assert!(msg.contains(needle), "path {path}: message was {msg}");
        }
    }

    /// A symlink FILE inside the tree must not be written through — the
    /// write would land wherever the link points (possibly outside the
    /// docs root), which the parent-directory containment cannot see.
    #[cfg(unix)]
    #[test]
    fn write_refuses_symlink_files() {
        let (dir, st) = fixture();
        let outside = dir.path().join("outside.md");
        std::fs::write(&outside, "# outside\n").unwrap();
        let link = st.src.join("en/evil.md");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        let w = call(
            &st,
            "docs.write",
            json!({
                "path": "en/evil.md",
                "content": "# injected\n"
            }),
        );
        assert_eq!(w["isError"], true, "symlink write must be refused");
        assert!(w["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("symbolic link"));
        // And the outside file was NOT touched.
        assert_eq!(
            std::fs::read_to_string(&outside).unwrap(),
            "# outside\n",
            "the write must not have landed through the link"
        );
    }

    #[test]
    fn write_guards_the_document_identity() {
        let (_dir, st) = fixture();
        let p = st.src.join("en/guides/quickstart.md");
        let original = std::fs::read_to_string(&p).unwrap();

        // Dropping the id → refused.
        let w = call(
            &st,
            "docs.write",
            json!({
                "path": "en/guides/quickstart.md",
                "content": "---\ntitle: Quickstart\n---\n# Quickstart\n\nNew body.\n"
            }),
        );
        assert_eq!(w["isError"], true);
        assert!(w["content"][0]["text"].as_str().unwrap().contains("DROP"));

        // Changing the id → refused.
        let w = call(
            &st,
            "docs.write",
            json!({
                "path": "en/guides/quickstart.md",
                "content": "---\ntitle: Quickstart\nid: other/id\n---\n# Quickstart\n\nNew body.\n"
            }),
        );
        assert_eq!(w["isError"], true);
        assert!(w["content"][0]["text"].as_str().unwrap().contains("CHANGE"));

        // Keeping the id → allowed; content lands.
        let w = call(
            &st,
            "docs.write",
            json!({
                "path": "en/guides/quickstart.md",
                "content": "---\ntitle: Quickstart\nid: guides/quickstart\n---\n# Quickstart\n\nNew body.\n"
            }),
        );
        assert_eq!(w["isError"], false, "raw: {w}");
        assert!(std::fs::read_to_string(&p).unwrap().contains("New body."));
        assert_ne!(std::fs::read_to_string(&p).unwrap(), original);
    }

    #[test]
    fn status_reports_build_diagnostics() {
        let (_dir, st) = fixture();
        let s = call(&st, "docs.status", json!({}));
        assert_eq!(s["isError"], false, "raw: {s}");
        let sc = &s["structuredContent"];
        assert_eq!(sc["build"]["ok"], true);
        assert!(sc["build"]["pages"].as_u64().unwrap() >= 2);
        // The fixture's SUMMARY references guides/missing.md → broken.
        let broken = sc["broken_links"].as_array().unwrap();
        assert!(
            broken
                .iter()
                .any(|b| b["target"].as_str().unwrap() == "guides/missing.html"),
            "broken: {broken:?}"
        );
        // index.md exists only in en → missing translation.
        let missing = sc["missing_translations"].as_array().unwrap();
        assert!(missing.iter().any(|m| m["document_id"] == "index"
            && m["missing"].as_array().unwrap().contains(&json!("zh-Hans"))));
        // Freshness tallies: 3 translations (en+zh quickstart, en index),
        // all fresh right after the build.
        assert_eq!(sc["freshness"]["fresh"].as_u64().unwrap(), 3);
    }

    #[test]
    fn status_without_any_build_tells_you_to_trigger_one() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("en")).unwrap();
        std::fs::write(dir.path().join("en/a.md"), "# A\n").unwrap();
        let st = McpState {
            src: dir.path().join("docs"),
            out: dir.path().join("dist"),
            build_opts: BuildOptions {
                src: dir.path().join("docs"),
                out: dir.path().join("dist"),
                site_url: None,
                default_lang: None,
            },
            write_paths: Vec::new(),
            build_lock: Arc::new(Mutex::new(())),
        };
        let s = call(&st, "docs.status", json!({}));
        assert_eq!(s["isError"], true);
        assert!(s["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("build.trigger"));
    }

    #[test]
    fn trigger_rebuilds_and_failure_is_recorded_and_visible() {
        let (_dir, st) = fixture();
        // Happy path: the trigger rebuilds and reports counts.
        let t = call(&st, "build.trigger", json!({}));
        assert_eq!(t["isError"], false, "raw: {t}");
        assert_eq!(t["structuredContent"]["ok"], true);
        assert_eq!(t["structuredContent"]["documents"].as_u64().unwrap(), 2);

        // Failure path: empty the docs root so the build fails ("no
        // language directories"); the failure must be recorded and surface
        // through docs.status.
        std::fs::remove_dir_all(st.src.join("zh-Hans")).unwrap();
        std::fs::remove_dir_all(st.src.join("en")).unwrap();
        let t = call(&st, "build.trigger", json!({}));
        assert_eq!(t["isError"], true);
        assert!(t["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("build failed"));
        let s = call(&st, "docs.status", json!({}));
        assert_eq!(s["isError"], false, "status itself must answer");
        assert_eq!(s["structuredContent"]["build"]["ok"], false);
        assert!(s["structuredContent"]["build"]["error"]
            .as_str()
            .unwrap()
            .contains("language"));
    }
}
