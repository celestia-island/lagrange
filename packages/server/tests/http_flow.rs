//! End-to-end HTTP flow tests against the full axum router.
//!
//! These spin up the real `app()` with an in-memory sqlite store, create a
//! moderator account, log in, post a comment as that moderator, list it,
//! moderate an anonymous comment, and vote — exercising the protocol surface
//! over actual HTTP. They are the authoritative proof that the server is
//! wired correctly; the protocol/adapter unit tests cover the domain logic.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use lagrange_server::{app, store::AppState};
use tower::ServiceExt;

/// Helper: build an app with one moderator account pre-seeded.
async fn seeded_app() -> (axum::Router, String) {
    let state = AppState::open_in_memory(b"test-secret").unwrap();
    let hash = lagrange_server::auth::hash_password("admin-pass").unwrap();
    let _account = state.create_account("admin", &hash, true).unwrap();
    let token = state
        .auth
        .issue(&lagrange_server::auth::Account {
            id: _account.id.clone(),
            name: _account.name.clone(),
            moderator: _account.moderator,
        })
        .unwrap();
    (app(state), token)
}

fn auth_request(token: &str, method: &str, uri: &str, body: String) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap()
}

async fn body_string(resp: axum::response::Response) -> String {
    use http_body_util::BodyExt;
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn health_reports_protocol_version() {
    let app = app(AppState::open_in_memory(b"s").unwrap());
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    assert!(body.contains("lagrange-comment/v1"));
    assert!(body.contains("\"status\":\"ok\""));
}

#[tokio::test]
async fn login_returns_jwt() {
    // Seed an account directly on the shared state... but app() consumes state.
    // Re-build with a dedicated helper instead.
    let state = AppState::open_in_memory(b"s").unwrap();
    let hash = lagrange_server::auth::hash_password("pw1234567").unwrap();
    state.create_account("alice", &hash, true).unwrap();
    let router = app(state);

    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"name": "alice", "password": "pw1234567"}).to_string(),
        ))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v["token"].as_str().is_some());
    assert_eq!(v["moderator"], true);
}

#[tokio::test]
async fn login_rejects_bad_credentials() {
    let app = app(AppState::open_in_memory(b"s").unwrap());
    let req = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"name": "ghost", "password": "x"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn full_comment_flow_as_moderator() {
    let (app, token) = seeded_app().await;

    // 1. Create a comment as the moderator.
    let create = serde_json::json!({
        "node_id": "2026/test-post",
        "body_markdown": "Hello **world**",
    });
    let resp = app
        .clone()
        .oneshot(auth_request(
            &token,
            "POST",
            "/comments",
            create.to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let comment: serde_json::Value = serde_json::from_str(&body).unwrap();
    let comment_id = comment["id"].as_str().unwrap().to_string();
    let thread_id = comment["thread_id"].as_str().unwrap().to_string();
    // Moderator comments are immediately visible.
    assert_eq!(comment["status"], "visible");
    assert!(comment["body_html"].as_str().unwrap().contains("<strong>"));

    // 2. Look up the thread.
    let resp = app
        .clone()
        .oneshot(auth_request(
            &token,
            "GET",
            "/threads?node=2026/test-post",
            String::new(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. List comments in the thread.
    let resp = app
        .clone()
        .oneshot(auth_request(
            &token,
            "GET",
            &format!("/comments?thread={thread_id}"),
            String::new(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let list: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(list["comments"].as_array().unwrap().len(), 1);

    // 4. Vote on the comment (moderator is authenticated).
    let vote = serde_json::json!({"dir": "up"});
    let resp = app
        .clone()
        .oneshot(auth_request(
            &token,
            "POST",
            &format!("/comments/{comment_id}/vote"),
            vote.to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 5. Edit the comment.
    let edit = serde_json::json!({"body_markdown": "edited body"});
    let resp = app
        .clone()
        .oneshot(auth_request(
            &token,
            "PATCH",
            &format!("/comments/{comment_id}"),
            edit.to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 6. Delete it.
    let resp = app
        .oneshot(auth_request(
            &token,
            "DELETE",
            &format!("/comments/{comment_id}"),
            String::new(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn anonymous_comment_is_pending_then_moderated() {
    let (app, token) = seeded_app().await;

    // Anonymous create (no auth header).
    let create = serde_json::json!({
        "node_id": "anon-test",
        "body_markdown": "from a guest",
        "author_name": "guest",
    });
    let req = Request::builder()
        .method("POST")
        .uri("/comments")
        .header("content-type", "application/json")
        .body(Body::from(create.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_string(resp).await;
    let comment: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(comment["status"], "pending");
    let comment_id = comment["id"].as_str().unwrap().to_string();

    // It is NOT visible to the public list.
    let thread_id = comment["thread_id"].as_str().unwrap().to_string();
    let resp = app
        .clone()
        .oneshot(auth_request(
            &token,
            "GET",
            &format!("/comments?thread={thread_id}"),
            String::new(),
        ))
        .await
        .unwrap();
    let list: serde_json::Value = serde_json::from_str(&body_string(resp).await).unwrap();
    assert!(list["comments"].as_array().unwrap().is_empty());

    // Moderator sees it in the pending queue.
    let resp = app
        .clone()
        .oneshot(auth_request(
            &token,
            "GET",
            "/admin/comments?filter=pending",
            String::new(),
        ))
        .await
        .unwrap();
    let list: serde_json::Value = serde_json::from_str(&body_string(resp).await).unwrap();
    assert_eq!(list["comments"].as_array().unwrap().len(), 1);

    // Approve it.
    let action = serde_json::json!({"action": "approve"});
    let resp = app
        .oneshot(auth_request(
            &token,
            "POST",
            &format!("/admin/comments/{comment_id}"),
            action.to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let approved: serde_json::Value = serde_json::from_str(&body_string(resp).await).unwrap();
    assert_eq!(approved["status"], "visible");
}

#[tokio::test]
async fn non_moderator_cannot_access_admin() {
    // Seed a non-moderator account.
    let state = AppState::open_in_memory(b"s").unwrap();
    let hash = lagrange_server::auth::hash_password("pw1234567").unwrap();
    let account = state.create_account("regular", &hash, false).unwrap();
    let token = state.auth.issue(&account).unwrap();
    let app = app(state);

    let resp = app
        .oneshot(auth_request(
            &token,
            "GET",
            "/admin/comments?filter=pending",
            String::new(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn validation_error_on_empty_body() {
    let (app, token) = seeded_app().await;
    let create = serde_json::json!({"node_id": "x", "body_markdown": "   "});
    let resp = app
        .oneshot(auth_request(
            &token,
            "POST",
            "/comments",
            create.to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_string(resp).await;
    let err: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(err["code"], "validation");
}

#[tokio::test]
async fn me_endpoint_reports_identity() {
    let (app, token) = seeded_app().await;
    let resp = app
        .oneshot(auth_request(&token, "GET", "/auth/me", String::new()))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_string(resp).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["authenticated"], true);
    assert_eq!(v["moderator"], true);

    // Anonymous.
    let anon_router = lagrange_server::app(AppState::open_in_memory(b"s2").unwrap());
    let resp = anon_router
        .oneshot(
            Request::builder()
                .uri("/auth/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&body_string(resp).await).unwrap();
    assert_eq!(v["authenticated"], false);
}
