//! `docs.status` and `build.trigger` — the build-facing tools.
//!
//! `docs.status` never invents state: everything it reports is read back
//! from the artifacts (`build_status.json`, `knowledge.json`) and the
//! current file bytes. `build.trigger` runs the exact build the dev
//! watch loop runs, serialized against it by the shared build lock.

use serde_json::{json, Value};

use super::McpState;
use crate::site::BuildStatus;

/// Handle `docs.status`.
pub fn status(state: &McpState, _args: &Value) -> Result<Value, String> {
    let status: BuildStatus = read_build_status(state)?;

    // Knowledge is optional here: right after a FAILED build there may be
    // no knowledge.json at all, and status must still answer.
    let knowledge = state.knowledge().ok();
    let (counts, freshness) = match &knowledge {
        Some(kb) => {
            let mut fresh = 0usize;
            let mut stale = 0usize;
            let mut missing = 0usize;
            for doc in &kb.documents {
                for (lang, tr) in &doc.translations {
                    match super::query::freshness_of(state, lang, tr) {
                        "fresh" => fresh += 1,
                        "stale" => stale += 1,
                        _ => missing += 1,
                    }
                }
            }
            (
                json!({
                    "documents": kb.documents.len(),
                    "chunks": kb.chunks.len(),
                    "langs": kb.source.langs,
                }),
                json!({ "fresh": fresh, "stale": stale, "missing": missing }),
            )
        }
        None => (Value::Null, Value::Null),
    };

    Ok(json!({
        "build": {
            "built_at": status.built_at,
            "duration_ms": status.duration_ms,
            "ok": status.ok,
            "error": status.error,
            "pages": status.pages,
            "langs": status.langs,
            // Which chrome lane produced the pages — the silent-fallback
            // signal (round 9/10 of PR #119).
            "lane": status.lane,
        },
        "knowledge": counts,
        "freshness": freshness,
        "missing_translations": status.missing_translations,
        "broken_links": status.broken_links,
    }))
}

/// Handle `build.trigger`.
pub fn trigger(state: &McpState, _args: &Value) -> Result<Value, String> {
    // Poisoned lock = a previous build panicked; recovery is "build again",
    // not "refuse forever" (dev server posture).
    let _guard = state
        .build_lock
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    match crate::site::build_with_status(&state.build_opts) {
        Ok(()) => {
            let status: BuildStatus = read_build_status(state)?;
            Ok(json!({
                "ok": true,
                "built_at": status.built_at,
                "duration_ms": status.duration_ms,
                "lane": status.lane,
                "pages": status.pages,
                "documents": status.documents,
                "chunks": status.chunks,
                "broken_links": status.broken_links.len(),
                "missing_translations": status.missing_translations.len(),
            }))
        }
        Err(e) => Err(format!(
            "build failed: {e:#}\n(failed build recorded — docs.status reflects it)"
        )),
    }
}

fn read_build_status(state: &McpState) -> Result<BuildStatus, String> {
    let path = state.out.join("build_status.json");
    let raw = std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "no build record at {} — run build.trigger first",
            path.display()
        )
    })?;
    serde_json::from_str(&raw).map_err(|e| format!("corrupt build_status.json: {e}"))
}
