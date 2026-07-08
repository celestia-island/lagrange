//! Comment mount-point generation.
//!
//! The site builder calls [`mount_html`] once per page, after the body HTML is
//! assembled. Depending on the configured `mode` (and the page's frontmatter
//! opt-out), this returns either:
//!
//! - **empty string** — `mode = "none"`, `enabled = false`, or `comments: false`
//!   in frontmatter. The page HTML stays exactly as it was before comments
//!   existed. This is the load-bearing "pure static" guarantee.
//! - **a `<lagrange-comments>` custom element** — for `faas` / `self-host` /
//!   `static-json`. The element's `data-*` attributes carry the wiring; the
//!   actual fetch/render happens client-side in `assets/lagrange-comments.js`
//!   (a separate, framework-free Web Component).
//! - **a third-party embed** — for `disqus` / `giscus` / `github-issue`. These
//!   inject the vendor's own script; no lagrange runtime is shipped.
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
        CommentMode::Faas | CommentMode::SelfHost => {
            faas_or_self_host_mount(input, &node_id)
        }
        CommentMode::StaticJson => static_json_mount(input, &node_id),
        CommentMode::Disqus => disqus_embed(input, &node_id),
        CommentMode::Giscus => giscus_embed(input, &node_id),
        CommentMode::GithubIssue => github_issue_embed(input, &node_id),
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

fn faas_or_self_host_mount(input: &MountInput<'_>, node_id: &str) -> String {
    let endpoint = input.config.endpoint.as_deref().unwrap_or("");
    let auth = input.config.auth_attr();
    let canonical = input.canonical.unwrap_or("");

    format!(
        r#"
<section class="lg-comments-section" aria-label="Comments">
<script src="/assets/lagrange-comments.js" defer></script>
<lagrange-comments
  data-mode="{}"
  data-endpoint="{}"
  data-node-id="{}"
  data-canonical="{}"
  data-auth="{}"
  data-archive="{}">
</lagrange-comments>
</section>
"#,
        mode_attr(input.config.mode),
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

fn disqus_embed(_input: &MountInput<'_>, node_id: &str) -> String {
    let shortname = _input.config.disqus_shortname.as_deref().unwrap_or("");
    format!(
        r#"
<section class="lg-comments-section" aria-label="Comments">
<div id="disqus_thread" data-disqus-identifier="{nid}"></div>
<script>
var disqus_config = function () {{
  this.page.identifier = "{nid}";
  this.page.url = window.location.href;
}};
(function() {{
  var d = document, s = d.createElement("script");
  s.src = "https://{short}.disqus.com/embed.js";
  s.setAttribute("data-timestamp", +new Date());
  (d.head || d.body).appendChild(s);
}})();
</script>
<noscript>Please enable JavaScript to view the comments.</noscript>
</section>
"#,
        nid = escape_attr(node_id),
        short = escape_attr(shortname),
    )
}

fn giscus_embed(input: &MountInput<'_>, node_id: &str) -> String {
    let repo = input.config.giscus_repo.as_deref().unwrap_or("");
    let category = input.config.giscus_category.as_deref().unwrap_or("");
    format!(
        r#"
<section class="lg-comments-section" aria-label="Comments">
<script src="https://giscus.app/client.js"
  data-repo="{repo}"
  data-repo-id=""
  data-category="{cat}"
  data-category-id=""
  data-mapping="specific"
  data-term="{nid}"
  data-strict="0"
  data-reactions-enabled="1"
  data-emit-metadata="0"
  data-input-position="top"
  data-theme="preferred_color_scheme"
  data-lang="en"
  crossorigin="anonymous"
  async>
</script>
</section>
"#,
        repo = escape_attr(repo),
        cat = escape_attr(category),
        nid = escape_attr(node_id),
    )
}

fn github_issue_embed(_input: &MountInput<'_>, node_id: &str) -> String {
    // utterances-style: a GitHub Issue per node id. Repository is taken from
    // giscus_repo as a fallback when only GitHub-Issue mode is configured.
    let repo = _input.config.giscus_repo.as_deref().unwrap_or("");
    format!(
        r#"
<section class="lg-comments-section" aria-label="Comments">
<script src="https://utteranc.es/client.js"
  repo="{repo}"
  issue-term="og:nic"
  label="comments"
  theme="preferred-color-scheme"
  crossorigin="anonymous"
  async>
</script>
<script>
// utterances does not support a custom term via data-* alone; rewrite the
// issue-term to our node id once the script is in place.
document.currentScript.addEventListener('load', function () {{
  var f = document.querySelector('iframe.utterances-frame');
  if (f) f.contentWindow.postMessage({{ type: 'set-config', config: {{ term: '{nid}' }} }}, '*');
}});
</script>
</section>
"#,
        repo = escape_attr(repo),
        nid = escape_attr(node_id),
    )
}

fn mode_attr(mode: CommentMode) -> &'static str {
    match mode {
        CommentMode::Faas => "faas",
        CommentMode::SelfHost => "self-host",
        CommentMode::StaticJson => "static-json",
        CommentMode::Disqus => "disqus",
        CommentMode::Giscus => "giscus",
        CommentMode::GithubIssue => "github-issue",
        CommentMode::None => "none",
    }
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
    use crate::config::CommentMode;

    fn cfg(mode: CommentMode) -> CommentsConfig {
        CommentsConfig {
            enabled: true,
            mode,
            endpoint: Some("https://c.example.workers.dev".into()),
            auth: vec!["anonymous".into(), "github".into()],
            archive_dir: "comments".into(),
            disqus_shortname: Some("mysite".into()),
            giscus_repo: Some("owner/repo".into()),
            giscus_category: Some("Announcements".into()),
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

    fn mount(mode: CommentMode, frontmatter: &FrontMatter) -> String {
        let config = cfg(mode);
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
        assert_eq!(mount(CommentMode::None, &fm), "");
    }

    #[test]
    fn disabled_is_zero_injection() {
        let mut config = cfg(CommentMode::Faas);
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
        // Even with a fully-wired faas config, the page is silent.
        assert_eq!(mount(CommentMode::Faas, &fm), "");
    }

    #[test]
    fn faas_mount_emits_custom_element() {
        let html = mount(CommentMode::Faas, &fm());
        assert!(html.contains("<lagrange-comments"), "got: {html}");
        assert!(html.contains("data-mode=\"faas\""));
        assert!(html.contains("data-endpoint=\"https://c.example.workers.dev\""));
        assert!(html.contains("data-node-id=\"2026/launch\""));
        assert!(html.contains("data-auth=\"anonymous,github\""));
        assert!(html.contains("/assets/lagrange-comments.js"));
    }

    #[test]
    fn self_host_mount_emits_custom_element() {
        let html = mount(CommentMode::SelfHost, &fm());
        assert!(html.contains("data-mode=\"self-host\""));
        assert!(html.contains("data-endpoint=\"https://c.example.workers.dev\""));
    }

    #[test]
    fn static_json_mount_points_at_archive() {
        let html = mount(CommentMode::StaticJson, &fm());
        assert!(html.contains("data-mode=\"static-json\""));
        assert!(html.contains("data-archive=\"/comments/2026/launch.json\""));
        // No endpoint attribute for the read-only mode.
        assert!(!html.contains("data-endpoint"));
    }

    #[test]
    fn disqus_mount_emits_disqus_thread() {
        let html = mount(CommentMode::Disqus, &fm());
        assert!(html.contains("id=\"disqus_thread\""));
        assert!(html.contains("mysite.disqus.com/embed.js"));
        assert!(html.contains("this.page.identifier = \"2026/launch\""));
    }

    #[test]
    fn giscus_mount_emits_giscus_client() {
        let html = mount(CommentMode::Giscus, &fm());
        assert!(html.contains("giscus.app/client.js"));
        assert!(html.contains("data-repo=\"owner/repo\""));
        assert!(html.contains("data-term=\"2026/launch\""));
    }

    #[test]
    fn github_issue_mount_emits_utterances() {
        let html = mount(CommentMode::GithubIssue, &fm());
        assert!(html.contains("utteranc.es/client.js"));
        assert!(html.contains("repo=\"owner/repo\""));
    }

    #[test]
    fn node_id_falls_back_to_slug_then_page_path() {
        // No explicit node_id, but a slug.
        let mut fm = fm();
        fm.node_id = None;
        fm.slug = Some("my-slug".into());
        let html = mount(CommentMode::Faas, &fm);
        assert!(html.contains("data-node-id=\"my-slug\""));

        // No node_id and no slug → page path derived.
        fm.slug = None;
        let html = mount(CommentMode::Faas, &fm);
        assert!(html.contains("data-node-id=\"posts/2026/launch\""));
    }

    #[test]
    fn attr_values_are_escaped() {
        let mut fm = fm();
        fm.node_id = Some("a<b>&\"x".into());
        let html = mount(CommentMode::Faas, &fm);
        assert!(html.contains("a&lt;b&gt;&amp;&quot;x"));
    }
}
