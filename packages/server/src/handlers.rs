//! HTTP handlers — thin JSON ↔ [`CommentService`] adapters.
//!
//! Each handler: (1) resolves the [`Caller`] from the `Authorization` header,
//! (2) parses the JSON body / query, (3) delegates to the protocol trait,
//! (4) returns the protocol types as JSON. Error mapping lives in
//! [`crate::error`], so handlers only deal with `Result<_, ApiError>`.

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Json,
};
use lagrange_protocol::{
    api::{Caller, CommentService, ListComments, ThreadLookup},
    types::*,
    CommentList, Thread,
};
use serde::Deserialize;

use crate::error::ApiError;
use crate::store::AppState;

/// Resolve a [`Caller`] from the request. `Authorization: Bearer <jwt>`
/// authenticates; absence → anonymous. Anonymous callers get a fingerprint
/// derived from a header so rate-limiting can still work.
pub fn caller_from_headers(
    headers: &HeaderMap,
    auth: &crate::auth::AuthState,
) -> Result<Caller, ApiError> {
    if let Some(value) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                let claims = auth.verify(token)?;
                return Ok(crate::auth::claims_to_caller(&claims));
            }
        }
    }
    let fingerprint = headers
        .get("x-client-fingerprint")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    Ok(Caller::Anonymous { fingerprint })
}

// ── query params ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NodeQuery {
    pub node: String,
}

#[derive(Deserialize)]
pub struct CommentsQuery {
    pub thread: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct ModQuery {
    #[serde(default = "default_mod_filter")]
    pub filter: String,
}

fn default_mod_filter() -> String {
    "pending".to_string()
}

fn parse_mod_filter(s: &str) -> Result<ModerationFilter, ApiError> {
    match s {
        "pending" => Ok(ModerationFilter::Pending),
        "spam" => Ok(ModerationFilter::Spam),
        "deleted" => Ok(ModerationFilter::Deleted),
        "all" => Ok(ModerationFilter::All),
        other => Err(ApiError::bad_request(
            "validation",
            format!("unknown filter '{other}'"),
        )),
    }
}

// ── thread + comment routes ───────────────────────────────────────────────

pub async fn get_thread(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<NodeQuery>,
) -> Result<axum::response::Response, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    match state.store.get_thread(&q.node, &caller) {
        Ok(ThreadLookup::Found(t)) => Ok(Json(ThreadResponse::Found(t)).into_response()),
        Ok(ThreadLookup::Missing { node_id }) => {
            Ok(Json(ThreadResponse::Missing { node_id }).into_response())
        }
        Err(e) => Err(ApiError::from_protocol(e)),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
enum ThreadResponse {
    Found(Thread),
    Missing { node_id: String },
}

pub async fn list_comments(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CommentsQuery>,
) -> Result<Json<CommentList>, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    let req = ListComments {
        thread_id: q.thread,
        cursor: q.cursor,
        limit: q.limit,
    };
    state
        .store
        .list_comments(&req, &caller)
        .map(Json)
        .map_err(ApiError::from_protocol)
}

pub async fn create_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateComment>,
) -> Result<(axum::http::StatusCode, Json<Comment>), ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    state
        .store
        .create_comment(&body, &caller)
        .map(|c| (axum::http::StatusCode::CREATED, Json(c)))
        .map_err(ApiError::from_protocol)
}

pub async fn edit_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<EditComment>,
) -> Result<Json<Comment>, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    state
        .store
        .edit_comment(&id, &body, &caller)
        .map(Json)
        .map_err(ApiError::from_protocol)
}

pub async fn delete_comment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    state
        .store
        .delete_comment(&id, &caller)
        .map(|_| axum::http::StatusCode::NO_CONTENT)
        .map_err(ApiError::from_protocol)
}

#[derive(Deserialize)]
pub struct VoteBody {
    pub dir: VoteDir,
}

pub async fn vote(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<VoteBody>,
) -> Result<Json<VoteSummary>, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    state
        .store
        .vote(&id, body.dir, &caller)
        .map(Json)
        .map_err(ApiError::from_protocol)
}

// ── moderation routes ─────────────────────────────────────────────────────

pub async fn list_moderation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ModQuery>,
) -> Result<Json<CommentList>, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    let filter = parse_mod_filter(&q.filter)?;
    state
        .store
        .list_moderation(filter, &caller)
        .map(Json)
        .map_err(ApiError::from_protocol)
}

#[derive(Deserialize)]
pub struct ModActionBody {
    pub action: ModerationAction,
}

pub async fn moderate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ModActionBody>,
) -> Result<Json<Comment>, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    state
        .store
        .moderate(&id, body.action, &caller)
        .map(Json)
        .map_err(ApiError::from_protocol)
}

// ── auth routes ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LoginBody {
    pub name: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub author: Author,
    pub moderator: bool,
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Json<LoginResponse>, ApiError> {
    let account = state.verify_login(&body.name, &body.password)?;
    let token = state.auth.issue(&account)?;
    Ok(Json(LoginResponse {
        token,
        author: Author {
            id: Some(account.id.clone()),
            name: account.name.clone(),
            avatar: None,
            identity_kind: IdentityKind::Local,
            external_id: None,
        },
        moderator: account.moderator,
    }))
}

pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = caller_from_headers(&headers, &state.auth)?;
    match caller {
        Caller::Anonymous { .. } => Ok(Json(serde_json::json!({
            "authenticated": false
        }))),
        Caller::Authenticated(a) => Ok(Json(serde_json::json!({
            "authenticated": true,
            "moderator": false,
            "author": a,
        }))),
        Caller::Moderator(a) => Ok(Json(serde_json::json!({
            "authenticated": true,
            "moderator": true,
            "author": a,
        }))),
    }
}

// ── health ────────────────────────────────────────────────────────────────

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "protocol": lagrange_protocol::PROTOCOL_VERSION,
    }))
}
