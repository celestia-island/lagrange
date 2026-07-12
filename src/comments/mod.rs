//! Comment mount-point generation.
//!
//! The site builder calls [`mount_html`] once per page, after the body HTML is
//! assembled. Depending on the configured `mode` (and the page's frontmatter
//! opt-out), this returns either:
//!
//! - **empty string** — `mode = "none"`, `enabled = false`, or `comments: false`
//!   in frontmatter. The page HTML stays exactly as it was before comments
//!   existed. This is the load-bearing "pure static" guarantee.
//! - **a `<lagrange-comments>` custom element** — for `proxied` and
//!   `static-json`. The element's `data-*` attributes carry the wiring; the
//!   actual fetch/render happens client-side in `runtime.js` (a separate,
//!   framework-free Web Component embedded into the binary at compile time).
//!   Every data source — native backend, GitHub Discussions/Issues proxy,
//!   Disqus proxy — renders the *same* component. The UI is always ours;
//!   only the data source behind `data-endpoint` varies.
//!
//! Comments are **never** embedded into the article body. The mount point is a
//! sibling element appended after `</main>`. The comment ↔ article link is
//! carried only by `data-node-id` / `data-canonical` attributes.

use crate::config::{CommentMode, CommentsConfig};
use crate::frontmatter::FrontMatter;

/// Inputs needed to render a single page's comment mount point.
pub struct MountInput<'a> {
    pub config: &'a CommentsConfig,
    pub frontmatter: &'a FrontMatter,
    /// Page path relative to site root, e.g. `posts/2026/launch.html`.
    pub page_path: &'a str,
    /// Canonical URL, if known (frontmatter `canonical` or config-derived).
    pub canonical: Option<&'a str>,
}

/// Render the comment mount-point HTML for one page.
///
/// Returns an empty `String` whenever comments are inactive for this page,
/// which is the no-op / pure-static path.
pub fn mount_html(input: &MountInput<'_>) -> String {
    // Hard opt-out via frontmatter.
    if input.frontmatter.comments == Some(false) {
        return String::new();
    }
    // Site-level inactive.
    if !input.config.is_active() {
        return String::new();
    }

    let node_id = input
        .frontmatter
        .effective_node_id()
        .map(str::to_string)
        .unwrap_or_else(|| derive_node_id(input.page_path));

    match input.config.mode {
        CommentMode::None => String::new(),
        CommentMode::Proxied => proxied_mount(input, &node_id),
        CommentMode::StaticJson => static_json_mount(input, &node_id),
    }
}

// ── private helpers ───────────────────────────────────────────────────────

fn derive_node_id(page_path: &str) -> String {
    // Fall back to the page path (sans extension) as the stable node id.
    page_path
        .trim_end_matches(".html")
        .trim_end_matches("/index")
        .trim_start_matches('/')
        .to_string()
}

fn archive_url(archive_dir: &str, node_id: &str) -> String {
    format!("/{archive_dir}/{node_id}.json")
}

/// The single live-backend mount point. Covers native (`lagrange-server` /
/// edge), and every proxied third-party source (GitHub Discussions/Issues,
/// Disqus) — the component talks the lagrange-comment/v1 protocol at
/// `data-endpoint` and never knows which source is behind it. `data-source`
/// tags the source for the proxy to route on and for UI hints; the runtime
/// component does not branch on it.
fn proxied_mount(input: &MountInput<'_>, node_id: &str) -> String {
    let endpoint = input.config.endpoint.as_deref().unwrap_or("");
    let auth = input.config.auth_attr();
    let canonical = input.canonical.unwrap_or("");
    let source = input.config.source_attr();

    format!(
        r#"
<section class="lg-comments-section" aria-label="Comments">
<script src="/assets/lagrange-comments.js" defer></script>
<lagrange-comments
  data-mode="proxied"
  data-source="{}"
  data-endpoint="{}"
  data-node-id="{}"
  data-canonical="{}"
  data-auth="{}"
  data-archive="{}">
</lagrange-comments>
</section>
"#,
        source,
        escape_attr(endpoint),
        escape_attr(node_id),
        escape_attr(canonical),
        escape_attr(&auth),
        escape_attr(&archive_url(&input.config.archive_dir, node_id)),
    )
}

fn static_json_mount(input: &MountInput<'_>, node_id: &str) -> String {
    // Read-only: the component only reads the archive JSON, never hits a
    // backend. Still ships the component so the UX is consistent.
    let archive = archive_url(&input.config.archive_dir, node_id);
    format!(
        r#"
<section class="lg-comments-section" aria-label="Comments">
<script src="/assets/lagrange-comments.js" defer></script>
<lagrange-comments
  data-mode="static-json"
  data-node-id="{}"
  data-archive="{}">
</lagrange-comments>
</section>
"#,
        escape_attr(node_id),
        escape_attr(&archive),
    )
}

fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CommentMode, CommentSource};

    fn cfg(mode: CommentMode, source: CommentSource) -> CommentsConfig {
        CommentsConfig {
            enabled: true,
            mode,
            source,
            endpoint: Some("https://c.example.workers.dev".into()),
            auth: vec!["anonymous".into(), "github".into()],
            archive_dir: "comments".into(),
            // Legacy vendor fields still parse but are dead in the new model.
            disqus_shortname: None,
            giscus_repo: None,
            giscus_repo_id: None,
            giscus_category: None,
            giscus_category_id: None,
        }
    }

    fn fm() -> FrontMatter {
        FrontMatter {
            title: Some("T".into()),
            node_id: Some("2026/launch".into()),
            canonical: Some("https://blog.example.com/2026/launch".into()),
            ..Default::default()
        }
    }

    fn mount(mode: CommentMode, source: CommentSource, frontmatter: &FrontMatter) -> String {
        let config = cfg(mode, source);
        let input = MountInput {
            config: &config,
            frontmatter,
            page_path: "posts/2026/launch.html",
            canonical: frontmatter.canonical.as_deref(),
        };
        mount_html(&input)
    }

    #[test]
    fn none_mode_is_zero_injection() {
        let fm = fm();
        assert_eq!(mount(CommentMode::None, CommentSource::Native, &fm), "");
    }

    #[test]
    fn disabled_is_zero_injection() {
        let mut config = cfg(CommentMode::Proxied, CommentSource::Native);
        config.enabled = false;
        let fm = fm();
        let input = MountInput {
            config: &config,
            frontmatter: &fm,
            page_path: "p.html",
            canonical: None,
        };
        assert_eq!(mount_html(&input), "");
    }

    #[test]
    fn frontmatter_opt_out_is_zero_injection() {
        let mut fm = fm();
        fm.comments = Some(false);
        // Even with a fully-wired proxied config, the page is silent.
        assert_eq!(mount(CommentMode::Proxied, CommentSource::Native, &fm), "");
    }

    #[test]
    fn proxied_native_mount_emits_custom_element() {
        let html = mount(CommentMode::Proxied, CommentSource::Native, &fm());
        assert!(html.contains("<lagrange-comments"), "got: {html}");
        assert!(html.contains("data-mode=\"proxied\""));
        assert!(html.contains("data-source=\"native\""));
        assert!(html.contains("data-endpoint=\"https://c.example.workers.dev\""));
        assert!(html.contains("data-node-id=\"2026/launch\""));
        assert!(html.contains("data-auth=\"anonymous,github\""));
        assert!(html.contains("/assets/lagrange-comments.js"));
        // No vendor scripts leak through.
        assert!(!html.contains("giscus.app"));
        assert!(!html.contains("utteranc.es"));
        assert!(!html.contains("disqus.com"));
    }

    #[test]
    fn proxied_github_discussions_tags_source() {
        let html = mount(
            CommentMode::Proxied,
            CommentSource::GitHubDiscussions,
            &fm(),
        );
        assert!(html.contains("data-source=\"github-discussions\""));
        assert!(html.contains("data-mode=\"proxied\""));
        // Still the same component, same protocol — no giscus widget.
        assert!(!html.contains("giscus.app/client.js"));
    }

    #[test]
    fn proxied_disqus_tags_source() {
        let html = mount(CommentMode::Proxied, CommentSource::Disqus, &fm());
        assert!(html.contains("data-source=\"disqus\""));
        assert!(!html.contains("disqus.com/embed.js"));
    }

    #[test]
    fn static_json_mount_points_at_archive() {
        let html = mount(CommentMode::StaticJson, CommentSource::Native, &fm());
        assert!(html.contains("data-mode=\"static-json\""));
        assert!(html.contains("data-archive=\"/comments/2026/launch.json\""));
        // No endpoint attribute for the read-only mode.
        assert!(!html.contains("data-endpoint"));
    }

    #[test]
    fn node_id_falls_back_to_slug_then_page_path() {
        // No explicit node_id, but a slug.
        let mut fm = fm();
        fm.node_id = None;
        fm.slug = Some("my-slug".into());
        let html = mount(CommentMode::Proxied, CommentSource::Native, &fm);
        assert!(html.contains("data-node-id=\"my-slug\""));

        // No node_id and no slug → page path derived.
        fm.slug = None;
        let html = mount(CommentMode::Proxied, CommentSource::Native, &fm);
        assert!(html.contains("data-node-id=\"posts/2026/launch\""));
    }

    #[test]
    fn attr_values_are_escaped() {
        let mut fm = fm();
        fm.node_id = Some("a<b>&\"x".into());
        let html = mount(CommentMode::Proxied, CommentSource::Native, &fm);
        assert!(html.contains("a&lt;b&gt;&amp;&quot;x"));
    }
}
