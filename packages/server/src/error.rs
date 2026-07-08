//! Error → HTTP response mapping.
//!
//! [`ApiError`] wraps a [`lagrange_protocol::ProtocolError`] and knows the HTTP
//! status it maps to. The axum layer turns any `Result<_, ApiError>` into a
//! JSON `ProtocolError` body with the right status code.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use lagrange_protocol::ProtocolError;

/// The server-side error type. Carries the protocol error + the HTTP status.
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub inner: ProtocolError,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status, self.inner.message)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            inner: ProtocolError::new(code, message),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", msg)
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", msg)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", msg)
    }

    pub fn bad_request(code: impl Into<String>, msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, msg)
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", msg)
    }

    /// Map a protocol-layer [`ProtocolError`] to an HTTP status by its code.
    /// This is the single place that decides status codes, so adding a new
    /// error code only touches here.
    pub fn from_protocol(err: ProtocolError) -> Self {
        let status = match err.code.as_str() {
            "validation" => StatusCode::BAD_REQUEST,
            "unauthorized" => StatusCode::UNAUTHORIZED,
            "forbidden" => StatusCode::FORBIDDEN,
            "not_found" => StatusCode::NOT_FOUND,
            "thread_locked" => StatusCode::CONFLICT,
            "rate_limited" => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self { status, inner: err }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.inner)).into_response()
    }
}
