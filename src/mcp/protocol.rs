//! Minimal MCP-over-stdio protocol layer — JSON-RPC 2.0, line-delimited.
//!
//! Hand-written on purpose (the survey that led here is recorded in the P2
//! PR description): the surface `lagrange dev --mcp` needs is the stable
//! core of the protocol — `initialize`, `tools/list`, `tools/call`, `ping`
//! — which is a few hundred lines on top of `serde_json` (already a
//! dependency). The published-crate candidates were rejected because they
//! would drag a second JSON stack or a young framework into a crate that
//! ships to crates.io consumers.
//!
//! Design constraints pinned here:
//!
//! - **stdout is protocol-only.** Every response is one line of JSON. All
//!   logging goes to stderr (see `main.rs`); a single stray stdout line
//!   would corrupt the client's stream.
//! - **Batching is rejected.** JSON-RPC 2.0 permits arrays, but MCP removed
//!   batching in the 2025-03-26 revision; a batch here is a client bug we
//!   answer with one error, not something we silently expand.
//! - **Notifications never get a response** — not even an error. Replying
//!   to a notification is the classic infinite-loop shape.
//! - Requests are processed strictly in arrival order (one line in, at most
//!   one line out); no interleaving is possible by construction.

use serde_json::{json, Value};

/// JSON-RPC / MCP error codes used by this server.
pub mod error_code {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
}

/// A protocol-level error to be answered with a JSON-RPC error response.
#[derive(Debug, Clone)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(error_code::INVALID_PARAMS, message)
    }
}

/// One incoming message after envelope parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    /// A request carrying an `id` — MUST be answered exactly once.
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    /// A notification (no `id`) — MUST NOT be answered.
    Notification { method: String, params: Value },
    /// A client-side RESPONSE (has `result`/`error`, no `method`). A server
    /// never sends requests, so receiving one means the stream is odd —
    /// ignoring it is the only reply that cannot create a loop.
    Response,
}

/// Serialize a success response line for the given request id.
pub fn response_result(id: Value, result: Value) -> String {
    let mut line = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }))
    .expect("responses are always serializable");
    line.push('\n');
    line
}

/// Serialize an error response line for the given request id.
pub fn response_error(id: Value, code: i64, message: impl Into<String>) -> String {
    let mut line = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message.into() },
    }))
    .expect("responses are always serializable");
    line.push('\n');
    line
}

/// Parse one input line into an incoming message, or a protocol error to
/// answer (the error carries the id to answer — `null` when unknown).
pub fn parse_message(line: &str) -> Result<Incoming, (Value, RpcError)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        // Blank lines are transport noise, not messages: nothing to parse
        // and no id to answer.
        return Ok(Incoming::Notification {
            method: String::new(),
            params: Value::Null,
        });
    }
    let value: Value = serde_json::from_str(trimmed).map_err(|e| {
        (
            Value::Null,
            RpcError::new(error_code::PARSE_ERROR, e.to_string()),
        )
    })?;
    let obj = match value.as_object() {
        Some(o) => o,
        None => {
            // Arrays are JSON-RPC batches (removed by MCP) or garbage.
            return Err((
                Value::Null,
                RpcError::new(
                    error_code::INVALID_REQUEST,
                    "expected a single JSON-RPC object (batching is not supported)",
                ),
            ));
        }
    };
    let method = obj
        .get("method")
        .and_then(|m| m.as_str())
        .map(str::to_string);
    let params = obj.get("params").cloned().unwrap_or(Value::Null);
    match (obj.get("id").cloned(), method) {
        (Some(id), Some(method)) => Ok(Incoming::Request { id, method, params }),
        (None, Some(method)) => Ok(Incoming::Notification { method, params }),
        // No method: either a client-side response (ignore — answering an
        // unknown id is how loops start) or a malformed frame with an id.
        // A response is recognisable by its result/error member; only a
        // true malformed frame gets the invalid-request error.
        (id, None) => {
            if obj.get("result").is_some() || obj.get("error").is_some() {
                Ok(Incoming::Response)
            } else {
                Err((
                    id.unwrap_or(Value::Null),
                    RpcError::new(error_code::INVALID_REQUEST, "message has no method"),
                ))
            }
        }
    }
}

/// Process one input line against `handle`, producing the response line to
/// write back (if any). `handle` receives every request AND notification
/// (notifications' results are discarded) so higher layers can observe
/// lifecycle events like `notifications/initialized`.
pub fn process_line<F>(line: &str, mut handle: F) -> Option<String>
where
    F: FnMut(&str, &Value) -> Result<Value, RpcError>,
{
    match parse_message(line) {
        Err((id, e)) => Some(response_error(id, e.code, e.message)),
        Ok(Incoming::Notification { method, params }) => {
            let _ = handle(&method, &params); // observed, never answered
            None
        }
        Ok(Incoming::Response) => None, // never answered — loop prevention
        Ok(Incoming::Request { id, method, params }) => match handle(&method, &params) {
            Ok(result) => Some(response_result(id, result)),
            Err(e) => Some(response_error(id, e.code, e.message)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_handler() -> impl FnMut(&str, &Value) -> Result<Value, RpcError> {
        |method: &str, params: &Value| {
            if method == "echo" {
                Ok(params.clone())
            } else if method == "fail" {
                Err(RpcError::invalid_params("boom"))
            } else if method == "notifications/anything" {
                // Real servers do not answer notifications; returning a
                // value here proves the layer discards it.
                Ok(json!({"sneaky": true}))
            } else {
                Err(RpcError::new(
                    error_code::METHOD_NOT_FOUND,
                    format!("no {method}"),
                ))
            }
        }
    }

    #[test]
    fn request_answered_with_result() {
        let out = process_line(
            r#"{"jsonrpc":"2.0","id":7,"method":"echo","params":{"a":1}}"#,
            ok_handler(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 7);
        assert_eq!(v["result"]["a"], 1);
        assert!(out.ends_with('\n'), "responses are newline-terminated");
    }

    #[test]
    fn string_ids_round_trip() {
        let out = process_line(
            r#"{"jsonrpc":"2.0","id":"call-1","method":"echo"}"#,
            ok_handler(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["id"], "call-1");
    }

    #[test]
    fn handler_error_becomes_jsonrpc_error() {
        let out =
            process_line(r#"{"jsonrpc":"2.0","id":2,"method":"fail"}"#, ok_handler()).unwrap();
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["error"]["code"], -32602);
        assert_eq!(v["error"]["message"], "boom");
    }

    #[test]
    fn unknown_method_is_method_not_found() {
        let out =
            process_line(r#"{"jsonrpc":"2.0","id":3,"method":"nope"}"#, ok_handler()).unwrap();
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["error"]["code"], -32601);
    }

    #[test]
    fn notifications_produce_no_response_even_if_handler_returns_one() {
        assert!(process_line(
            r#"{"jsonrpc":"2.0","method":"notifications/anything"}"#,
            ok_handler()
        )
        .is_none());
    }

    #[test]
    fn garbage_line_is_a_parse_error_with_null_id() {
        let out = process_line("not json at all", ok_handler()).unwrap();
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["id"], Value::Null);
        assert_eq!(v["error"]["code"], -32700);
    }

    #[test]
    fn batch_arrays_are_rejected_not_expanded() {
        let out = process_line(
            r#"[{"jsonrpc":"2.0","id":1,"method":"echo"}]"#,
            ok_handler(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["error"]["code"], -32600);
        assert!(v["error"]["message"].to_string().contains("batch"));
    }

    #[test]
    fn id_without_method_is_ignored_without_response() {
        // Answering an unknown id is the loop shape; the layer must swallow
        // it instead.
        assert!(
            process_line(r#"{"jsonrpc":"2.0","id":9,"result":{"x":1}}"#, ok_handler()).is_none()
        );
    }

    #[test]
    fn blank_line_is_skipped() {
        assert!(process_line("   ", ok_handler()).is_none());
    }

    #[test]
    fn params_default_to_null() {
        let out =
            process_line(r#"{"jsonrpc":"2.0","id":4,"method":"echo"}"#, ok_handler()).unwrap();
        let v: Value = serde_json::from_str(out.trim()).unwrap();
        assert_eq!(v["result"], Value::Null);
    }
}
