//! CLI scaffolding: `lagrange init`, `lagrange comments link`, and the
//! `--comments` dev-server helper.
//!
//! These subcommands make it fast to stand up a working lagrange site with
//! comments wired in, without hand-editing `lagrange.toml` or looking up
//! GitHub GraphQL node ids.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tracing::info;

/// `lagrange init` — scaffold a lagrange.toml + docs/ skeleton.
pub fn init_site(dir: &Path, title: Option<&str>, lang: &str, comments: &str) -> Result<()> {
    let title = title.unwrap_or("My Docs");
    std::fs::create_dir_all(dir.join(lang))?;

    // lagrange.toml
    let toml = render_init_toml(title, lang, comments);
    let toml_path = dir.join("lagrange.toml");
    if toml_path.exists() {
        anyhow::bail!("{} already exists — refusing to overwrite", toml_path.display());
    }
    std::fs::write(&toml_path, toml)
        .with_context(|| format!("write {}", toml_path.display()))?;

    // docs/<lang>/README.md (the landing page)
    let readme_path = dir.join(lang).join("README.md");
    if !readme_path.exists() {
        std::fs::write(&readme_path, format!("# {title}\n\nWelcome to your new lagrange site.\n"))?;
    }
    // docs/<lang>/SUMMARY.md (sidebar)
    let summary_path = dir.join(lang).join("SUMMARY.md");
    if !summary_path.exists() {
        std::fs::write(&summary_path, "- [Home](./README.md)\n")?;
    }

    info!("✓ scaffolded lagrange site at {}", dir.display());
    println!("  created: {}", toml_path.display());
    println!("  created: {}", readme_path.display());
    println!("  created: {}", summary_path.display());
    println!("\n  Next: lagrange build --src {} --out _site", dir.display());
    if comments != "none" {
        println!("  Then: lagrange comments link --src {}", dir.display());
    }
    Ok(())
}

fn render_init_toml(title: &str, lang: &str, comments: &str) -> String {
    let mut s = format!(
        "[site]\ntitle = \"{title}\"\n\n[languages]\ndefault = \"{lang}\"\norder = [\"{lang}\"]\n"
    );
    match comments {
        "native" => s.push_str(
            "\n[comments]\nenabled = true\nmode = \"proxied\"\nsource = \"native\"\nendpoint = \"http://localhost:18099\"\nauth = [\"anonymous\", \"local\"]\n",
        ),
        "github-discussions" => s.push_str(
            "\n[comments]\nenabled = true\nmode = \"proxied\"\nsource = \"github-discussions\"\n# Run `lagrange comments link --repo owner/name` to fill endpoint + IDs\nauth = [\"github\", \"anonymous\"]\n",
        ),
        "disqus" => s.push_str(
            "\n[comments]\nenabled = true\nmode = \"proxied\"\nsource = \"disqus\"\nauth = [\"anonymous\"]\n",
        ),
        _ => {} // none — no comments block
    }
    s
}

/// `lagrange comments link` — fetch GitHub/Discus IDs and update lagrange.toml.
///
/// For GitHub: queries the GraphQL API (via `gh` CLI if available, else
/// instructs the user) for the repo node id + discussion category ids, then
/// writes a proxy config block.
pub fn comments_link(src: &Path, repo: Option<&str>, disqus: Option<&str>) -> Result<()> {
    if let Some(repo) = repo {
        link_github(src, repo)?;
    } else if let Some(shortname) = disqus {
        link_disqus(src, shortname)?;
    } else {
        anyhow::bail!("specify --repo owner/name (GitHub) or --disqus shortname");
    }
    Ok(())
}

fn link_github(src: &Path, repo: &str) -> Result<()> {
    // Try `gh` CLI first — it handles auth transparently.
    let (repo_id, categories) = match fetch_github_ids_via_gh(repo) {
        Ok(data) => data,
        Err(e) => {
            println!("Could not query GitHub via `gh` ({e}).");
            println!("Ensure `gh auth login` has been run, or fill the IDs manually.");
            println!("\nTo find them manually:");
            println!("  repo-id:     gh api repos/{repo} --jq .node_id");
            println!("  category-id: gh api graphql -f query='...discussionCategories...'");
            anyhow::bail!("could not auto-fetch GitHub IDs");
        }
    };

    // Find or suggest a "Comments" category.
    let comments_cat = categories
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case("comments"))
        .or_else(|| categories.first());

    let (cat_name, cat_id) = if let Some(c) = comments_cat {
        (&c.name, &c.id)
    } else {
        println!("No discussion categories found. Create a 'Comments' category first:");
        println!("  gh api repos/{repo}/discussions-categories ...");
        anyhow::bail!("no discussion categories");
    };

    // Write a proxy config file next to lagrange.toml.
    let proxy_cfg = src.join("lagrange-proxy.toml");
    let config = format!(
        "# lagrange proxy config — used by the edge/worker that fronts GitHub Discussions.
# Deploy this proxy (see packages/edge) and point lagrange.toml [comments].endpoint at it.
source = \"github-discussions\"
repo = \"{repo}\"
repo_id = \"{repo_id}\"
category = \"{cat_name}\"
category_id = \"{cat_id}\"
# A read token (PAT with public_repo scope) — required even for public-repo reads.
read_token = \"ghp_YOUR_TOKEN_HERE\"
");
    std::fs::write(&proxy_cfg, config)?;
    info!("✓ wrote proxy config: {}", proxy_cfg.display());
    println!("  repo:        {repo}");
    println!("  repo-id:     {repo_id}");
    println!("  category:    {cat_name} ({cat_id})");
    println!("\n  Next: deploy the proxy (packages/edge), then set endpoint in lagrange.toml.");
    Ok(())
}

struct GhCategory {
    name: String,
    id: String,
}

fn fetch_github_ids_via_gh(repo: &str) -> Result<(String, Vec<GhCategory>)> {
    let (owner, name) = repo
        .split_once('/')
        .context("repo must be owner/name")?;

    // repo node id
    let repo_id = std::process::Command::new("gh")
        .args(["api", &format!("repos/{repo}"), "--jq", ".node_id"])
        .output()
        .context("run `gh` CLI")?;
    if !repo_id.status.success() {
        anyhow::bail!(
            "`gh api` failed: {}",
            String::from_utf8_lossy(&repo_id.stderr)
        );
    }
    let repo_id = String::from_utf8(repo_id.stdout)?.trim().to_string();

    // discussion categories via GraphQL
    let query = format!(
        "query {{ repository(owner:\"{owner}\",name:\"{name}\") {{ discussionCategories(first:20) {{ nodes {{ id name }} }} }} }}"
    );
    let cat_out = std::process::Command::new("gh")
        .args(["api", "graphql", "-f", &format!("query={query}")])
        .output()
        .context("run `gh api graphql`")?;
    if !cat_out.status.success() {
        anyhow::bail!(
            "`gh api graphql` failed: {}",
            String::from_utf8_lossy(&cat_out.stderr)
        );
    }
    let json: serde_json::Value = serde_json::from_slice(&cat_out.stdout)?;
    let nodes = json["data"]["repository"]["discussionCategories"]["nodes"]
        .as_array()
        .context("no category nodes")?;
    let categories = nodes
        .iter()
        .map(|n| GhCategory {
            id: n["id"].as_str().unwrap_or("").to_string(),
            name: n["name"].as_str().unwrap_or("").to_string(),
        })
        .collect();
    Ok((repo_id, categories))
}

fn link_disqus(src: &Path, shortname: &str) -> Result<()> {
    let proxy_cfg = src.join("lagrange-proxy.toml");
    let config = format!(
        "# lagrange proxy config — fronts Disqus.
source = \"disqus\"
shortname = \"{shortname}\"
# Public API key (read-only). Get yours from disqus.com/api/applications/.
api_key = \"YOUR_DISQUS_API_KEY\"
# Secret key (server-side only, for guest writes). Optional for read-only.
api_secret = \"\"
");
    std::fs::write(&proxy_cfg, config)?;
    info!("✓ wrote proxy config: {}", proxy_cfg.display());
    println!("  shortname: {shortname}");
    Ok(())
}

/// Start the dev comment backend on the given port (in-memory store, anonymous
/// + local auth). Used by `lagrange dev --comments`.
pub fn run_dev_comments(port: u16) -> Result<()> {
    // Build a minimal axum server speaking the lagrange-comment protocol.
    // We reuse lagrange-server's app if available; otherwise a stub.
    let bind = format!("127.0.0.1:{port}");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};

        // Tiny in-process store for dev.
        let store = Arc::new(Mutex::new(DevStore::default()));

        let app = axum::Router::new()
            .route(
                "/health",
                axum::routing::get(|| async {
                    axum::Json(serde_json::json!({"status":"ok","protocol":"lagrange-comment/v1"}))
                }),
            )
            .route(
                "/meta",
                axum::routing::get(|| async {
                    axum::Json(serde_json::json!({
                        "source": "native",
                        "capabilities": {
                            "read": true, "login_write": true, "guest_write": true,
                            "edit": true, "delete": true, "vote": true, "moderate": true
                        }
                    }))
                }),
            )
            .route(
                "/threads",
                axum::routing::get({
                    let store = store.clone();
                    move |q: axum::extract::Query<HashMap<String, String>>| {
                        let store = store.clone();
                        async move { dev_get_thread(store, q) }
                    }
                }),
            )
            .route(
                "/comments",
                axum::routing::get({
                    let store = store.clone();
                    move |q: axum::extract::Query<HashMap<String, String>>| {
                        let store = store.clone();
                        async move { dev_list_comments(store, q) }
                    }
                })
                .post({
                    let store = store.clone();
                    move |body: axum::Json<serde_json::Value>| {
                        let store = store.clone();
                        async move { dev_create_comment(store, body) }
                    }
                }),
            );

        let listener = tokio::net::TcpListener::bind(&bind).await?;
        tracing::info!("dev comments backend on http://{bind}");
        axum::serve(listener, app).await
    }).map_err(|e| anyhow::anyhow!("dev comments server: {e}"))
}

// ── minimal dev store (not production-grade; just enough for local testing) ─

#[derive(Default)]
struct DevStore {
    threads: HashMap<String, String>,      // node_id → thread_id
    comments: Vec<serde_json::Value>,
}

fn dev_get_thread(
    store: Arc<Mutex<DevStore>>,
    q: axum::extract::Query<HashMap<String, String>>,
) -> axum::Json<serde_json::Value> {
    let node = q.get("node").cloned().unwrap_or_default();
    let s = store.lock().unwrap();
    if let Some(tid) = s.threads.get(&node) {
        axum::Json(serde_json::json!({"status":"found","id":tid,"node_id":node,"comment_count":s.comments.iter().filter(|c|c["node_id"]==node).count(),"locked":false}))
    } else {
        axum::Json(serde_json::json!({"status":"missing","node_id":node}))
    }
}

fn dev_list_comments(
    store: Arc<Mutex<DevStore>>,
    q: axum::extract::Query<HashMap<String, String>>,
) -> axum::Json<serde_json::Value> {
    let thread = q.get("thread").cloned().unwrap_or_default();
    let s = store.lock().unwrap();
    let comments: Vec<_> = s
        .comments
        .iter()
        .filter(|c| c["thread_id"].as_str() == Some(&thread))
        .cloned()
        .collect();
    axum::Json(serde_json::json!({"comments":comments,"next_cursor":null}))
}

fn dev_create_comment(
    store: Arc<Mutex<DevStore>>,
    body: axum::Json<serde_json::Value>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let mut s = store.lock().unwrap();
    let node = body["node_id"].as_str().unwrap_or("n").to_string();
    // Lazy thread creation.
    let tid = s
        .threads
        .entry(node.clone())
        .or_insert_with(|| format!("t_{}", ulid_lite()))
        .clone();
    let id = format!("c_{}", ulid_lite());
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let comment = serde_json::json!({
        "id": id, "thread_id": tid, "parent_id": body.get("parent_id"),
        "node_id": node, "canonical_url": body.get("canonical_url"),
        "author": {"name": body.get("author_name").and_then(|v|v.as_str()).unwrap_or("guest"), "identity_kind": "anonymous"},
        "body_markdown": body["body_markdown"], "body_html": format!("<p>{}</p>", body["body_markdown"].as_str().unwrap_or("")),
        "created_at": now, "status": "visible",
        "votes": {"up": 0, "down": 0}
    });
    s.comments.push(comment.clone());
    (axum::http::StatusCode::CREATED, axum::Json(comment))
}

fn ulid_lite() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{ts:x}")
}
