//! Proxy adapters — implement [`CommentService`] against third-party APIs.
//!
//! Each adapter (GitHub Discussions, GitHub Issues, Disqus) translates the
//! lagrange-comment/v1 protocol to the vendor's API. The edge/server layer
//! instantiates one of these as the `store` parameter; the handler, routing,
//! and JSON serialisation are unchanged from the native backend.
//!
//! Because [`CommentService`] is a **sync** trait but every vendor API is
//! async, adapters bridge via [`block_on`] (a current-thread tokio runtime
//! per call). This is fine for an edge worker where each request already
//! runs on a tokio thread.

pub mod capabilities;
pub mod github;

pub use capabilities::{Capabilities, Source};

/// Run an async future on a fresh current-thread runtime and block for the
/// result. Each call creates + drops a runtime (cheap relative to a network
/// round-trip). Avoids "runtime within runtime" panics when the adapter is
/// called from inside an existing tokio context by always using a bare
/// current-thread executor.
pub fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build block_on runtime")
        .block_on(future)
}
