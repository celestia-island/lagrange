//! Cloudflare Worker binding for lagrange-edge.
//!
//! This is a *reference* binding, not a workspace member. Copy it into a
//! standalone worker crate (`cargo generate cloudflare/worker-template`) and
//! add `lagrange-edge` + a D1-backed adapter as dependencies.
//!
//! The whole job of this file is to translate the Worker `Request` into the
//! `(method, path, query, headers, body)` tuple that `lagrange_edge::handle`
//! expects, and translate its `EdgeResponse` back into a Worker `Response`.

use std::collections::HashMap;

use lagrange_adapter::MemoryStore; // replace with a D1 adapter in production
use lagrange_edge::{handle, AnonymousResolver};
use worker::*;

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let method = req.method().to_string();
    let url = req.url()?;
    let path = url.path().to_string();
    let query = url.query().unwrap_or("").to_string();

    let mut headers = HashMap::new();
    for (k, v) in req.headers() {
        headers.insert(k.to_lowercase(), v.to_string());
    }
    let body = req.text().await.unwrap_or_default();

    // For a real deploy: build a D1-backed store from `env.d1("...")` here
    // and a JWT resolver instead of AnonymousResolver.
    let store = MemoryStore::new();
    let resp = handle(&method, &path, &query, &headers, &body, &store, &AnonymousResolver);

    let mut builder = ResponseBuilder::new(resp.status).with_headers(&resp.headers);
    // The worker crate's ResponseBuilder builds from these pieces.
    let _ = builder; // (illustrative — exact API follows your worker-template version)
    Response::ok(resp.body).map(|r| {
        let _ = resp.status;
        r.with_headers(headers_from_map(&resp.headers))
    })
}

fn headers_from_map(map: &HashMap<&'static str, String>) -> Headers {
    let mut h = Headers::new();
    for (k, v) in map {
        let _ = h.insert(k, v);
    }
    h
}

// NOTE: this file is illustrative and intentionally does not compile in the
// workspace (it depends on the `worker` crate, which needs the wasm32 target).
// See the README in this directory for the full deploy walkthrough.
