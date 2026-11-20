//! `docs.write` — scoped markdown write-back into the docs tree.
//!
//! The write model is deliberately blunt: full-file content at a
//! tree-relative path, landing as a plain edit the human reviews in
//! `git diff`. No patches, no renames, no partial updates — the scope and
//! identity guards below are the whole safety story, so they are ordered
//! cheapest-first and each rejection names the exact rule it enforced.

use std::path::Path;

use serde_json::{json, Value};

use super::McpState;

/// Handle `docs.write`.
pub fn write(state: &McpState, args: &Value) -> Result<Value, String> {
    let path = str_arg(args, "path")?;
    let content = str_arg(args, "content")?;

    // 1. Path shape: tree-relative, no escapes, markdown only.
    let normalized = normalize_write_path(&path)?;

    // 2. Language segment must be a configured/known language directory —
    //    a file landing anywhere else would never be built.
    let langs = known_langs(state);
    let first = normalized.split('/').next().unwrap_or_default();
    if !langs.iter().any(|l| l == first) {
        return Err(format!(
            "first path segment `{first}` is not a language directory (known: {}) — \
             tree paths include the language segment, e.g. `en/guides/quickstart.md`",
            langs.join(", ")
        ));
    }

    // 3. Scope: the docs root by default, narrowed to `[mcp] write_paths`
    //    prefixes when configured.
    if !state.write_paths.is_empty() {
        let allowed = state.write_paths.iter().any(|p| {
            let p = normalize_prefix(p);
            normalized == p || normalized.starts_with(&format!("{p}/"))
        });
        if !allowed {
            return Err(format!(
                "path `{normalized}` is outside the configured write scope \
                 ([mcp] write_paths = [{}]) — only those prefixes are writable",
                state
                    .write_paths
                    .iter()
                    .map(|p| format!("`{p}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    // 4. Containment on the real filesystem (symlink defense): the target's
    //    parent must resolve inside the docs root.
    let target = state.src.join(&normalized);
    // A symlink FILE would be followed by the write below (landing outside
    // the tree even when the parent check passes) — refuse it outright.
    match std::fs::symlink_metadata(&target) {
        Ok(md) if md.file_type().is_symlink() => {
            return Err(format!(
                "path `{normalized}` is a symbolic link — docs.write does not \
                 write through symlinks; write the real file instead"
            ));
        }
        _ => {}
    }
    let existed = target.is_file();
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        let root = state
            .src
            .canonicalize()
            .map_err(|e| format!("cannot resolve docs root: {e}"))?;
        let resolved = parent
            .canonicalize()
            .map_err(|e| format!("cannot resolve {}: {e}", parent.display()))?;
        if !resolved.starts_with(&root) {
            return Err(format!(
                "path `{normalized}` resolves to `{}`, outside the docs root `{}`",
                resolved.display(),
                root.display()
            ));
        }
    }

    // 5. Identity guards (only when overwriting an existing file): an
    //    explicit frontmatter `id:` is the document's stable identity —
    //    dropping or changing it on rewrite would silently re-key every
    //    existing citation. A file without `id:` keeps its path-derived
    //    identity precisely because the path does not change (that is the
    //    "derived semantics survive unchanged paths" rule).
    if existed {
        let existing = std::fs::read_to_string(&target)
            .map_err(|e| format!("cannot read current {}: {e}", target.display()))?;
        let (_, existing_fm, _) = crate::frontmatter::strip(&existing);
        let (_, new_fm, _) = crate::frontmatter::strip(&content);
        match (&existing_fm.id, &new_fm.id) {
            (Some(old), None) => {
                return Err(format!(
                    "refusing to write: this would DROP the explicit document id `{old}` — \
                     keep `id: {old}` in the frontmatter so the document keeps its identity \
                     (or write to a new path)"
                ));
            }
            (Some(old), Some(new)) if old != new => {
                return Err(format!(
                    "refusing to write: this would CHANGE the document id `{old}` → `{new}` — \
                     ids are stable identities; restore `id: {old}` or move the content to a \
                     new path"
                ));
            }
            _ => {}
        }
    }

    // 6. Write.
    std::fs::write(&target, content.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", target.display()))?;
    Ok(json!({
        "path": normalized,
        "bytes": content.len(),
        "created": !existed,
    }))
}

/// Validate and normalize a requested write path: backslashes fold to `/`,
/// absolute paths, `..` escapes, empty segments and non-markdown suffixes
/// are rejected. Returns the normalized tree-relative path.
fn normalize_write_path(path: &str) -> Result<String, String> {
    let p = path.replace('\\', "/");
    if p.starts_with('/') || Path::new(&p).is_absolute() {
        return Err(format!(
            "path `{p}` must be tree-RELATIVE (start with the language segment), not absolute"
        ));
    }
    let mut parts: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                return Err(format!(
                    "path `{p}` must not contain `..` — stay inside the docs tree"
                ));
            }
            s => parts.push(s),
        }
    }
    if parts.is_empty() {
        return Err(format!("path `{path}` does not name a file"));
    }
    let normalized = parts.join("/");
    if !normalized.to_ascii_lowercase().ends_with(".md") {
        return Err(format!(
            "path `{normalized}` must be a markdown file (`.md`) — docs.write only writes pages"
        ));
    }
    Ok(normalized)
}

fn normalize_prefix(prefix: &str) -> String {
    prefix
        .replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// The language directories to accept as first path segments: the built
/// knowledge's list when available, else a direct directory listing of the
/// docs root (a build must have run for the dev server to get this far,
/// but the listing keeps `write` usable the instant it hasn't).
fn known_langs(state: &McpState) -> Vec<String> {
    if let Ok(kb) = state.knowledge() {
        if !kb.source.langs.is_empty() {
            return kb.source.langs;
        }
    }
    let mut langs: Vec<String> = std::fs::read_dir(&state.src)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    langs.sort();
    langs
}

fn str_arg(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| format!("missing required string argument `{key}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_accepts_tree_relative_markdown() {
        assert_eq!(
            normalize_write_path("en/guides/quickstart.md").unwrap(),
            "en/guides/quickstart.md"
        );
        assert_eq!(
            normalize_write_path("./en/a.md").unwrap(),
            "en/a.md",
            "leading ./ is harmless"
        );
        assert_eq!(
            normalize_write_path("en\\a.md").unwrap(),
            "en/a.md",
            "backslashes fold"
        );
    }

    #[test]
    fn normalize_rejects_escapes_and_non_markdown() {
        assert!(normalize_write_path("../outside.md").is_err());
        assert!(normalize_write_path("en/../../etc/passwd.md").is_err());
        assert!(normalize_write_path("/abs/path.md").is_err());
        assert!(normalize_write_path("en/image.png").is_err());
        assert!(normalize_write_path("").is_err());
        assert!(normalize_write_path("en/").is_err());
    }

    #[test]
    fn prefix_matching_is_component_wise() {
        // The exact rule used by the scope check, pinned from both sides.
        assert!(normalize_write_path("en/agent-notes/a.md")
            .map(|p| p == "en/agent-notes" || p.starts_with("en/agent-notes/"))
            .unwrap());
        // `en/agent-notes-x/a.md` must NOT match prefix `en/agent-notes`.
        let p = normalize_write_path("en/agent-notes-x/a.md").unwrap();
        assert!(!(p == "en/agent-notes" || p.starts_with("en/agent-notes/")));
    }
}
