//! `lagrange-server` — self-hostable lagrange comment backend.
//!
//! Exposes the [`lagrange_protocol::CommentService`] surface over HTTP (axum),
//! backed by [`lagrange_adapter::SqliteStore`] and a small local account store
//! (argon2 + JWT). The binary (`src/main.rs`) adds a `serve` subcommand and a
//! `create-admin` bootstrap; the library exposes [`app`] so integration tests
//! can spin the router up against an in-memory store.

pub mod auth;
pub mod error;
pub mod handlers;
pub mod store;

use axum::{routing, Router};
use tower_http::cors::CorsLayer;

use store::AppState;

/// Build the full HTTP router around `state`. Stateless — callers (the binary
/// or tests) own the `AppState`.
pub fn app(state: AppState) -> Router {
    Router::new()
        // Protocol routes (mirror lagrange_protocol::api::routes).
        .route("/threads", routing::get(handlers::get_thread))
        .route("/comments", routing::get(handlers::list_comments).post(handlers::create_comment))
        .route(
            "/comments/{id}",
            routing::patch(handlers::edit_comment).delete(handlers::delete_comment),
        )
        .route("/comments/{id}/vote", routing::post(handlers::vote))
        // Auth.
        .route("/auth/login", routing::post(handlers::login))
        .route("/auth/me", routing::get(handlers::me))
        // Moderation.
        .route("/admin/comments", routing::get(handlers::list_moderation))
        .route("/admin/comments/{id}", routing::post(handlers::moderate))
        // Health.
        .route("/health", routing::get(handlers::health))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
