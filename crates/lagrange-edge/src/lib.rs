//! `lagrange-edge` — runtime-neutral HTTP handler for lagrange comments.
//!
//! Every serverless platform (Cloudflare Workers, Vercel Edge Functions,
//! Firebase Functions, …) exposes a different request/response API, but they
//! all model an HTTP exchange. This crate reduces that exchange to a single
//! pure function:
//!
//! ```ignore
//! handle(method, path, query, headers, body, &store, &auth_verifier)
//!     -> EdgeResponse { status, headers, body }
//! ```
//!
//! A platform binding (see `examples/` and the per-platform notes below) is
//! then a ~30-line shim that translates its native request into the inputs and
//! the [`EdgeResponse`] back into its native response. The handler owns all the
//! protocol logic, error mapping, and JSON (de)serialisation — identical on
//! every platform.
//!
//! ## Storage on the edge
//!
//! The handler is generic over any [`CommentService`], so each platform plugs
//! in its native store:
//!
//! | Platform | Store adapter | Notes |
//! |----------|---------------|-------|
//! | Cloudflare Workers | D1 (SQL) or KV | reuses `lagrange-adapter` SQL dialect for D1 |
//! | Vercel Edge | KV / Postgres | |
//! | Firebase | Firestore | document model |
//!
//! For dev/test, [`lagrange_adapter::MemoryStore`] works out of the box.

use std::collections::HashMap;

use lagrange_protocol::{
    api::{Caller, CommentService, ListComments, ThreadLookup},
    types::*,
    PROTOCOL_VERSION,
};

/// A minimal HTTP response. Platform bindings translate this into their
/// native response object.
#[derive(Debug, Clone)]
pub struct EdgeResponse {
    pub status: u16,
    pub headers: HashMap<&'static str, String>,
    pub body: String,
}

impl EdgeResponse {
    pub fn json(status: u16, json: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type", "application/json; charset=utf-8".into());
        headers.insert(
            "access-control-allow-origin",
            "*".into(),
        );
        Self {
            status,
            headers,
            body: json.to_string(),
        }
    }

    pub fn no_content() -> Self {
        let mut headers = HashMap::new();
        headers.insert("access-control-allow-origin", "*".into());
        Self {
            status: 204,
            headers,
            body: String::new(),
        }
    }

    pub fn error(status: u16, err: ProtocolError) -> Self {
        Self::json(status, &serde_json::to_string(&err).unwrap_or_else(|_| "{}".into()))
    }
}

/// A caller verifier: given the request headers, decide who is calling.
/// On the edge, auth is usually a JWT check (shared secret) or an OAuth
/// callback state. Implementations are tiny; the handler stays generic.
pub trait CallerResolver {
    fn resolve(&self, headers: &HashMap<String, String>) -> Caller;
}

/// An anonymous-only resolver — the simplest valid edge deploy (no accounts,
/// comments held for moderation). Production deploys plug in a JWT resolver.
pub struct AnonymousResolver;

impl CallerResolver for AnonymousResolver {
    fn resolve(&self, headers: &HashMap<String, String>) -> Caller {
        let fingerprint = headers
            .get("x-client-fingerprint")
            .cloned()
            .or_else(|| headers.get("cf-connecting-ip").cloned());
        Caller::Anonymous { fingerprint }
    }
}

/// The entry point. `path` is the URL path (e.g. `/comments`), `query` is the
/// raw query string (e.g. `thread=t1&cursor=…`), `headers` is a lowercased-key
/// map, `body` is the raw request body.
///
/// The handler routes by `method + path`, parses inputs, calls the protocol
/// trait on `store`, and maps the result to an [`EdgeResponse`].
pub fn handle<S: CommentService, R: CallerResolver>(
    method: &str,
    path: &str,
    query: &str,
    headers: &HashMap<String, String>,
    body: &str,
    store: &S,
    resolver: &R,
) -> EdgeResponse {
    // CORS preflight short-circuit.
    if method == "OPTIONS" {
        let mut h = HashMap::new();
        h.insert("access-control-allow-origin", "*".into());
        h.insert(
            "access-control-allow-methods",
            "GET,POST,PATCH,DELETE,OPTIONS".into(),
        );
        h.insert("access-control-allow-headers", "authorization,content-type".into());
        return EdgeResponse {
            status: 204,
            headers: h,
            body: String::new(),
        };
    }

    let caller = resolver.resolve(headers);
    let params = parse_query(query);
    let route = (method, path);

    macro_rules! protocol_err {
        ($e:expr) => {
            EdgeResponse::error(status_for_code(&$e.code), $e)
        };
    }

    match route {
        ("GET", "/health") => EdgeResponse::json(
            200,
            &serde_json::to_string(&serde_json::json!({
                "status": "ok",
                "protocol": PROTOCOL_VERSION,
            }))
            .unwrap(),
        ),

        ("GET", "/threads") => {
            let Some(node) = params.get("node") else {
                return EdgeResponse::error(400, ProtocolError::new("validation", "missing ?node="));
            };
            match store.get_thread(node, &caller) {
                Ok(ThreadLookup::Found(t)) => EdgeResponse::json(
                    200,
                    &serde_json::to_string(&serde_json::json!({"status":"found","id":t.id,"node_id":t.node_id,"comment_count":t.comment_count,"locked":t.locked})).unwrap(),
                ),
                Ok(ThreadLookup::Missing { node_id }) => EdgeResponse::json(
                    200,
                    &serde_json::to_string(&serde_json::json!({"status":"missing","node_id":node_id})).unwrap(),
                ),
                Err(e) => protocol_err!(e),
            }
        }

        ("GET", "/comments") => {
            let Some(thread) = params.get("thread") else {
                return EdgeResponse::error(400, ProtocolError::new("validation", "missing ?thread="));
            };
            let req = ListComments {
                thread_id: thread.clone(),
                cursor: params.get("cursor").cloned(),
                limit: params.get("limit").and_then(|s| s.parse().ok()),
            };
            match store.list_comments(&req, &caller) {
                Ok(list) => EdgeResponse::json(200, &serde_json::to_string(&list).unwrap()),
                Err(e) => protocol_err!(e),
            }
        }

        ("POST", "/comments") => {
            let parsed: Result<CreateComment, _> = serde_json::from_str(body);
            let Ok(req) = parsed else {
                return EdgeResponse::error(400, ProtocolError::new("validation", "invalid JSON body"));
            };
            match store.create_comment(&req, &caller) {
                Ok(c) => EdgeResponse::json(201, &serde_json::to_string(&c).unwrap()),
                Err(e) => protocol_err!(e),
            }
        }

        ("PATCH", p) if p.starts_with("/comments/") => {
            let id = p.trim_start_matches("/comments/");
            let parsed: Result<EditComment, _> = serde_json::from_str(body);
            let Ok(req) = parsed else {
                return EdgeResponse::error(400, ProtocolError::new("validation", "invalid JSON body"));
            };
            match store.edit_comment(id, &req, &caller) {
                Ok(c) => EdgeResponse::json(200, &serde_json::to_string(&c).unwrap()),
                Err(e) => protocol_err!(e),
            }
        }

        ("DELETE", p) if p.starts_with("/comments/") => {
            let id = p.trim_start_matches("/comments/");
            match store.delete_comment(id, &caller) {
                Ok(()) => EdgeResponse::no_content(),
                Err(e) => protocol_err!(e),
            }
        }

        ("POST", p) if p.starts_with("/comments/") && p.ends_with("/vote") => {
            let id = p.trim_start_matches("/comments/");
            let id = id.trim_end_matches("/vote");
            let parsed: Result<VoteBody, _> = serde_json::from_str(body);
            let Ok(vb) = parsed else {
                return EdgeResponse::error(400, ProtocolError::new("validation", "invalid JSON body"));
            };
            match store.vote(id, vb.dir, &caller) {
                Ok(s) => EdgeResponse::json(200, &serde_json::to_string(&s).unwrap()),
                Err(e) => protocol_err!(e),
            }
        }

        _ => EdgeResponse::error(404, ProtocolError::new("not_found", "unknown route")),
    }
}

#[derive(serde::Deserialize)]
struct VoteBody {
    dir: VoteDir,
}

fn parse_query(q: &str) -> HashMap<String, String> {
    // serde_urlencoded parses a+b as space and decodes %xx; for our keys
    // (node ids, cursors) that's the right behaviour.
    serde_urlencoded::from_str::<HashMap<String, String>>(q).unwrap_or_default()
}

fn status_for_code(code: &str) -> u16 {
    match code {
        "validation" => 400,
        "unauthorized" => 401,
        "forbidden" => 403,
        "not_found" => 404,
        "thread_locked" => 409,
        "rate_limited" => 429,
        _ => 500,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lagrange_adapter::MemoryStore;

    fn store() -> MemoryStore {
        MemoryStore::new()
    }
    fn headers() -> HashMap<String, String> {
        HashMap::new()
    }

    #[test]
    fn health_reports_protocol() {
        let r = handle("GET", "/health", "", &headers(), "", &store(), &AnonymousResolver);
        assert_eq!(r.status, 200);
        assert!(r.body.contains(PROTOCOL_VERSION));
    }

    #[test]
    fn unknown_route_is_404() {
        let r = handle("GET", "/nope", "", &headers(), "", &store(), &AnonymousResolver);
        assert_eq!(r.status, 404);
    }

    #[test]
    fn cors_preflight_short_circuits() {
        let r = handle("OPTIONS", "/comments", "", &headers(), "", &store(), &AnonymousResolver);
        assert_eq!(r.status, 204);
        assert_eq!(r.headers.get("access-control-allow-origin"), Some(&"*".to_string()));
    }

    #[test]
    fn anonymous_create_then_list() {
        let s = store();
        // Create as anonymous.
        let body = r#"{"node_id":"n1","body_markdown":"hello","author_name":"guest"}"#;
        let r = handle("POST", "/comments", "", &headers(), body, &s, &AnonymousResolver);
        assert_eq!(r.status, 201);
        let c: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(c["status"], "pending"); // anonymous → pending
        let tid = c["thread_id"].as_str().unwrap().to_string();

        // Public list is empty (pending not shown).
        let q = format!("thread={tid}");
        let r = handle("GET", "/comments", &q, &headers(), "", &s, &AnonymousResolver);
        let list: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert!(list["comments"].as_array().unwrap().is_empty());
    }

    #[test]
    fn thread_lookup_round_trip() {
        let s = store();
        // Missing before any comment.
        let r = handle("GET", "/threads", "node=zzz", &headers(), "", &s, &AnonymousResolver);
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["status"], "missing");

        // Create one, then it's found.
        let body = r#"{"node_id":"zzz","body_markdown":"hi"}"#;
        // Use an authenticated caller so the comment is visible immediately.
        let author = Author {
            id: Some("u".into()),
            name: "U".into(),
            avatar: None,
            identity_kind: IdentityKind::Local,
            external_id: None,
        };
        struct Authed;
        impl CallerResolver for Authed {
            fn resolve(&self, _: &HashMap<String, String>) -> Caller {
                Caller::Authenticated(Author {
                    id: Some("u".into()),
                    name: "U".into(),
                    avatar: None,
                    identity_kind: IdentityKind::Local,
                    external_id: None,
                })
            }
        }
        let _ = author; // silence unused binding
        let r = handle("POST", "/comments", "", &headers(), body, &s, &Authed);
        assert_eq!(r.status, 201);

        let r = handle("GET", "/threads", "node=zzz", &headers(), "", &s, &Authed);
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["status"], "found");
        assert_eq!(v["comment_count"], 1);
    }

    #[test]
    fn vote_requires_identity() {
        let s = store();
        // Seed a visible comment via an authed caller.
        struct Authed;
        impl CallerResolver for Authed {
            fn resolve(&self, _: &HashMap<String, String>) -> Caller {
                Caller::Authenticated(Author {
                    id: Some("u".into()),
                    name: "U".into(),
                    avatar: None,
                    identity_kind: IdentityKind::Local,
                    external_id: None,
                })
            }
        }
        let body = r#"{"node_id":"v","body_markdown":"x"}"#;
        let r = handle("POST", "/comments", "", &headers(), body, &s, &Authed);
        let cid = serde_json::from_str::<serde_json::Value>(&r.body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Anonymous cannot vote.
        let r = handle(
            "POST",
            &format!("/comments/{cid}/vote"),
            "",
            &headers(),
            r#"{"dir":"up"}"#,
            &s,
            &AnonymousResolver,
        );
        assert_eq!(r.status, 403);

        // Authed can.
        let r = handle(
            "POST",
            &format!("/comments/{cid}/vote"),
            "",
            &headers(),
            r#"{"dir":"up"}"#,
            &s,
            &Authed,
        );
        assert_eq!(r.status, 200);
    }

    #[test]
    fn validation_error_maps_to_400() {
        let r = handle(
            "POST",
            "/comments",
            "",
            &headers(),
            r#"{"node_id":"n","body_markdown":"   "}"#,
            &store(),
            &AnonymousResolver,
        );
        assert_eq!(r.status, 400);
        let e: ProtocolError = serde_json::from_str(&r.body).unwrap();
        assert_eq!(e.code, "validation");
    }

    #[test]
    fn invalid_json_is_400() {
        let r = handle(
            "POST",
            "/comments",
            "",
            &headers(),
            "not json",
            &store(),
            &AnonymousResolver,
        );
        assert_eq!(r.status, 400);
    }
}
