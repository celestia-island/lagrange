//! Tool registry — names, input schemas, dispatch.
//!
//! Descriptions are the agent-facing contract: an MCP client never sees
//! this crate's docs, only these strings. They deliberately spell out the
//! stale-revision rule and the write scope, because those are the two ways
//! an agent can silently do the wrong thing.

use serde_json::{json, Value};

use super::protocol::RpcError;
use super::McpState;

/// The ordered tool list. Order is part of the presentation (agents read
/// top-down); keep read tools before write tools.
pub const TOOL_NAMES: [&str; 6] = [
    "docs.search",
    "docs.fetch",
    "docs.tree",
    "docs.write",
    "docs.status",
    "build.trigger",
];

/// `tools/list` payload: one descriptor per tool with a JSON Schema for its
/// arguments.
pub fn descriptors() -> Value {
    json!([
        {
            "name": "docs.search",
            "description": "Full-text search over the built knowledge base. Returns chunk-level hits, each carrying the citation five-tuple (document_id, lang, revision, span, anchor), a snippet cut from the exact source span, and the source path. Tokenisation matches the static site search: lowercase ASCII words and CJK bigrams. Filter with `lang` (language code) and `path_prefix` (language-relative path prefix, e.g. `guides/`). Results are ranked (heading matches weigh 3x body matches) and deterministic.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search text (words and/or CJK; minimum one indexable token)." },
                    "lang": { "type": "string", "description": "Restrict to one language code (e.g. `en`, `zh-Hans`)." },
                    "path_prefix": { "type": "string", "description": "Restrict to translations whose language-relative path starts with this prefix (e.g. `guides/`)." },
                    "limit": { "type": "integer", "description": "Max hits (default 10, capped at 50)." }
                },
                "required": ["query"]
            }
        },
        {
            "name": "docs.fetch",
            "description": "Fetch the exact source text of one chunk by echoing a citation: document_id + lang + revision + (anchor | span). The requested revision is validated against BOTH the built index and the current file bytes; on any mismatch the call fails with a STALE error naming the current revision instead of silently returning different content. Re-search and re-cite after a stale error.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "document_id": { "type": "string" },
                    "lang": { "type": "string" },
                    "revision": { "type": "string", "description": "Content hash (sha256 hex) cited from a previous search/tree result." },
                    "anchor": { "type": "string", "description": "Chunk anchor, e.g. `quickstart` or `prologue`. Exactly one of anchor/span." },
                    "span": {
                        "type": "object",
                        "description": "Chunk span with start_line/end_line (1-based inclusive) and start_byte/end_byte (0-based, end exclusive) over the full source file. Exactly one of anchor/span.",
                        "properties": {
                            "start_line": { "type": "integer" },
                            "end_line": { "type": "integer" },
                            "start_byte": { "type": "integer" },
                            "end_byte": { "type": "integer" }
                        },
                        "required": ["start_line", "end_line", "start_byte", "end_byte"]
                    }
                },
                "required": ["document_id", "lang", "revision"]
            }
        },
        {
            "name": "docs.tree",
            "description": "The document tree with per-language translations and freshness. Freshness compares each translation's recorded revision against the current file bytes: fresh / stale (file changed since the build) / missing (file gone). Also lists configured languages and, per document, the languages it has no translation for.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "docs.write",
            "description": "Write a full markdown file into the docs tree. `path` is tree-relative INCLUDING the language segment (e.g. `en/guides/quickstart.md`); the first segment must be a configured language directory. Scope: the docs root, narrowed by `[mcp] write_paths` prefixes in lagrange.toml when configured — paths outside the scope are rejected, as are path escapes and symlink targets. Identity guards (on overwrite): refuses to DROP or CHANGE an explicit frontmatter `id:`, and refuses to ADD an `id:` that differs from the file's path-derived id (that would silently re-key the document and strand citations; adding the idempotent derived value is allowed). A file without `id:` keeps its path-derived identity because the path is unchanged. The write lands as a plain file edit (visible in git diff); rebuild via build.trigger.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Tree-relative path with language segment, e.g. `en/guides/quickstart.md`." },
                    "content": { "type": "string", "description": "Full file content (frontmatter included) in UTF-8." }
                },
                "required": ["path", "content"]
            }
        },
        {
            "name": "docs.status",
            "description": "Last build outcome and corpus health: build time/duration/ok/error from build_status.json, knowledge counts, per-corpus freshness tallies, missing translations and broken internal links. Fails with instructions to run build.trigger when no build has been recorded yet.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "build.trigger",
            "description": "Run a full rebuild (same build the dev server's watch loop runs, serialized with it). Returns the new build summary, or fails with the full error chain — including file paths — when the build fails; docs.status then reflects the failure.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

/// `tools/call` dispatch. Protocol-level problems (missing/unknown tool
/// name, malformed arguments envelope) surface as JSON-RPC errors;
/// domain-level outcomes (stale fetch, scope rejection, unknown document)
/// return a successful call with `isError: true` and a text message — per
/// the MCP spec that is how a tool reports "ran, but the answer is no".
pub fn call(state: &McpState, params: &Value) -> Result<Value, RpcError> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("tools/call requires a string `name`"))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    if !args.is_object() {
        return Err(RpcError::invalid_params("`arguments` must be an object"));
    }
    let outcome: Result<Value, String> = match name {
        "docs.search" => super::query::search(state, &args),
        "docs.fetch" => super::query::fetch(state, &args),
        "docs.tree" => super::query::tree(state, &args),
        "docs.write" => super::write::write(state, &args),
        "docs.status" => super::ops::status(state, &args),
        "build.trigger" => super::ops::trigger(state, &args),
        other => {
            return Err(RpcError::invalid_params(format!(
                "unknown tool `{other}`; available: {}",
                TOOL_NAMES.join(", ")
            )));
        }
    };
    Ok(match outcome {
        Ok(payload) => json!({
            "content": [{ "type": "text", "text": pretty(&payload) }],
            "structuredContent": payload,
            "isError": false
        }),
        Err(message) => json!({
            "content": [{ "type": "text", "text": message }],
            "isError": true
        }),
    })
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The schema contract is pinned: every descriptor has the three
    /// required MCP fields and a `type: object` inputSchema whose
    /// `required` entries all exist in `properties` (tools without
    /// `required` — tree/status/trigger — are legal: all args optional).
    #[test]
    fn descriptors_are_well_formed() {
        let tools = descriptors();
        let arr = tools.as_array().unwrap();
        assert_eq!(arr.len(), TOOL_NAMES.len(), "registry and list agree");
        for t in arr {
            assert!(t["name"].is_string());
            assert!(t["description"].as_str().unwrap().len() > 40);
            assert_eq!(t["inputSchema"]["type"], "object");
            let props = t["inputSchema"]["properties"].as_object().unwrap();
            for req in t["inputSchema"]["required"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
            {
                let req = req.as_str().unwrap();
                assert!(
                    props.contains_key(req),
                    "required `{req}` missing from properties"
                );
            }
        }
    }

    /// The fetch description must state the stale rule — it is the
    /// acceptance line for this surface, and descriptions are the only
    /// contract agents see.
    #[test]
    fn fetch_description_pins_the_stale_rule() {
        let tools = descriptors();
        let fetch = tools
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "docs.fetch")
            .unwrap();
        assert!(fetch["description"].as_str().unwrap().contains("STALE"));
    }
}
