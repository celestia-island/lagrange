//! Process-level end-to-end test: `lagrange dev --mcp` over a tempdir docs
//! tree, driven through the real stdio JSON-RPC surface.
//!
//! This is the "dev 起停 + MCP 全链" acceptance slice: a REAL child process,
//! REAL stdio framing (one JSON object per line), REAL builds — including
//! the stdout-purity invariant, asserted by parsing every line the child
//! ever writes to stdout as JSON (a single stray log line would fail the
//! parse and this test with it).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{json, Value};

/// Spawn the dev server with `--mcp` over a fresh tree.
fn spawn_dev(tree: &std::path::Path) -> Child {
    let bin = env!("CARGO_BIN_EXE_lagrange");
    Command::new(bin)
        .args([
            "dev",
            "--src",
            tree.join("docs").to_str().unwrap(),
            "--out",
            tree.join("dist").to_str().unwrap(),
            "--port",
            "0",
            "--mcp",
        ])
        .env("LAGRANGE_SKIP_LIVE", "1")
        .env("RUST_LOG", "lagrange_library=info")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn lagrange dev")
}

/// One JSON-RPC line reader on a channel, so reads can time out.
struct LineReader {
    rx: mpsc::Receiver<String>,
}

impl LineReader {
    fn spawn(child: &mut Child) -> Self {
        let stdout = child.stdout.take().expect("piped stdout");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        });
        Self { rx }
    }

    fn next_line(&self, what: &str) -> String {
        self.rx
            .recv_timeout(Duration::from_secs(60))
            .unwrap_or_else(|e| panic!("timed out waiting for {what}: {e}"))
    }
}

struct DevProcess {
    child: Child,
    stdin: std::process::ChildStdin,
    reader: LineReader,
    next_id: i64,
}

impl DevProcess {
    /// Send a request; return the matching response (id-correlated).
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let frame = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let line = serde_json::to_string(&frame).unwrap();
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
        loop {
            let raw = self.reader.next_line(method);
            // Stdout purity, two layers: EVERY line must parse as JSON
            // (a stray tracing line fails here) AND be a well-formed
            // JSON-RPC response (`jsonrpc: "2.0"` + a `result` or `error`
            // member) — arbitrary JSON objects like `{"pollution":true}`
            // must not slip through either (review R2, S3).
            let v: Value = serde_json::from_str(&raw)
                .unwrap_or_else(|e| panic!("non-JSON on stdout: {raw:?} ({e})"));
            let is_response = v.is_object()
                && v.get("jsonrpc").and_then(|j| j.as_str()) == Some("2.0")
                && (v.get("result").is_some() || v.get("error").is_some());
            assert!(
                is_response,
                "non-response JSON on stdout: {raw:?} — stdout must carry only JSON-RPC responses"
            );
            if v.get("id").and_then(|i| i.as_i64()) == Some(id) {
                return v;
            }
            // Not ours (should not happen — one client, serial requests);
            // keep draining until the id matches.
        }
    }

    fn notify(&mut self, method: &str) {
        let frame = json!({"jsonrpc": "2.0", "method": method});
        let line = serde_json::to_string(&frame).unwrap();
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    fn call_tool(&mut self, name: &str, args: Value) -> Value {
        self.request("tools/call", json!({"name": name, "arguments": args}))
    }
}

impl Drop for DevProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn fixture_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("docs");
    std::fs::create_dir_all(src.join("en/guides")).unwrap();
    std::fs::write(
        src.join("en/SUMMARY.md"),
        "- [Quickstart](guides/quickstart.md)\n",
    )
    .unwrap();
    std::fs::write(
        src.join("en/index.md"),
        "---\ntitle: Home\n---\n# Home\n\nWelcome.\n",
    )
    .unwrap();
    std::fs::write(
        src.join("en/guides/quickstart.md"),
        "---\ntitle: Quickstart\n---\n# Quickstart\n\nInstall the tool from the registry.\n\n## Configure\n\nEdit the config file.\n",
    )
    .unwrap();
    dir
}

#[test]
fn dev_mcp_full_chain_search_fetch_write_trigger_status() {
    let tree = fixture_tree();
    let mut child = spawn_dev(tree.path());
    let reader = LineReader::spawn(&mut child);
    let stdin = child.stdin.take().expect("piped stdin");
    let mut dev = DevProcess {
        child,
        stdin,
        reader,
        next_id: 1,
    };

    // 1. Handshake.
    let init = dev.request(
        "initialize",
        json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "e2e-test"}}),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "lagrange");
    dev.notify("notifications/initialized");

    // 2. Tool list.
    let list = dev.request("tools/list", json!({}));
    let names: Vec<&str> = list["result"]["tools"]
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

    // 3. Search → citation five-tuple.
    let search = dev.call_tool("docs.search", json!({"query": "registry", "lang": "en"}));
    assert_eq!(search["result"]["isError"], false, "raw: {search}");
    let hits = search["result"]["structuredContent"]["results"]
        .as_array()
        .unwrap();
    assert!(!hits.is_empty(), "raw: {search}");
    let hit = &hits[0];
    assert_eq!(hit["document_id"], "guides/quickstart");
    for f in ["document_id", "lang", "revision", "span", "anchor"] {
        assert!(hit.get(f).is_some_and(|v| !v.is_null()), "field {f}");
    }

    // 4. Fetch the exact cited bytes.
    let fetch = dev.call_tool(
        "docs.fetch",
        json!({
            "document_id": hit["document_id"],
            "lang": hit["lang"],
            "revision": hit["revision"],
            "anchor": hit["anchor"],
        }),
    );
    assert_eq!(fetch["result"]["isError"], false, "raw: {fetch}");
    let content = fetch["result"]["structuredContent"]["content"]
        .as_str()
        .unwrap();
    let source = std::fs::read_to_string(tree.path().join("docs/en/guides/quickstart.md")).unwrap();
    let span = &hit["span"];
    assert_eq!(
        content,
        &source[span["start_byte"].as_u64().unwrap() as usize
            ..span["end_byte"].as_u64().unwrap() as usize],
        "fetched bytes must equal the cited span"
    );

    // 5. Stale fetch is explicit, not silent: edit the file behind the
    //    server's back, then fetch the OLD revision.
    {
        let p = tree.path().join("docs/en/guides/quickstart.md");
        let mut text = std::fs::read_to_string(&p).unwrap();
        text.push_str("\nEdited out-of-band.\n");
        std::fs::write(&p, text).unwrap();
    }
    let stale = dev.call_tool(
        "docs.fetch",
        json!({
            "document_id": hit["document_id"],
            "lang": hit["lang"],
            "revision": hit["revision"],
            "anchor": hit["anchor"],
        }),
    );
    assert_eq!(stale["result"]["isError"], true, "raw: {stale}");
    assert!(
        stale["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("STALE:"),
        "raw: {stale}"
    );
    assert!(stale["result"].get("structuredContent").is_none());

    // 6. Write back (in scope: whole docs root by default).
    let write = dev.call_tool(
        "docs.write",
        json!({
            "path": "en/guides/agent-notes.md",
            "content": "# Agent notes\n\nAdded through MCP.\n"
        }),
    );
    assert_eq!(write["result"]["isError"], false, "raw: {write}");
    let on_disk = std::fs::read_to_string(tree.path().join("docs/en/guides/agent-notes.md"))
        .expect("write must land on disk");
    assert!(on_disk.contains("Added through MCP."));
    // Out-of-tree write is rejected.
    let bad = dev.call_tool(
        "docs.write",
        json!({"path": "../escape.md", "content": "# x\n"}),
    );
    assert_eq!(bad["result"]["isError"], true);

    // 7. Rebuild; the new document is in the knowledge base afterwards.
    let trigger = dev.call_tool("build.trigger", json!({}));
    assert_eq!(trigger["result"]["isError"], false, "raw: {trigger}");
    assert_eq!(trigger["result"]["structuredContent"]["ok"], true);
    assert_eq!(
        trigger["result"]["structuredContent"]["documents"]
            .as_u64()
            .unwrap(),
        3,
        "index + quickstart + agent-notes"
    );

    // 8. Status reflects the rebuilt world.
    let status = dev.call_tool("docs.status", json!({}));
    assert_eq!(status["result"]["isError"], false, "raw: {status}");
    assert_eq!(status["result"]["structuredContent"]["build"]["ok"], true);
    // The lane must be OBSERVABLE through MCP too (round 10): the
    // silent-fallback signal one layer up from build_status.json.
    let lane = status["result"]["structuredContent"]["build"]["lane"]
        .as_str()
        .unwrap_or("");
    assert!(
        lane == "webui" || lane == "legacy",
        "lane must be reported, got {lane:?}"
    );
    // Per-language URL scheme: the flat root becomes a language-negotiation
    // stub BY DESIGN, and each language's landing page is the real page —
    // pin both so the collision can never silently flip (round 10 S2).
    // MUT: M54 (inverting the root-stub pin turns this assertion RED).
    {
        let root = std::fs::read_to_string(tree.path().join("dist").join("index.html"))
            .expect("root index");
        assert!(
            root.contains("location.replace"),
            "the flat root is a negotiation stub under the per-language scheme"
        );
        let en = std::fs::read_to_string(tree.path().join("dist").join("en").join("index.html"))
            .expect("en landing page");
        assert!(
            en.contains("lagrange-page-data"),
            "the real landing content lives at /en/ (and per-language)"
        );
    }

    // 9. Ping still answers after everything (long-lived session).
    let ping = dev.request("ping", json!({}));
    assert_eq!(ping["result"], json!({}));
    // Drop kills the child (dev 起停).
}
