//! `lagrange-migrate` — import content from foreign engines into lagrange.
//!
//! Pipeline: `Reader → ExchangeDoc → Writer(s)`. One run can write multiple
//! sinks (e.g. markdown for articles + archive-json for comments) from a
//! single source. Comments and articles are emitted to separate trees and
//! never share a file.
//!
//! Examples:
//!
//! ```text
//! # WordPress WXR → markdown articles + comment archive
//! lagrange-migrate import --from wordpress --wxr dump.xml \
//!     --write markdown --out docs \
//!     --write archive-json --out comments
//!
//! # Hexo posts → markdown only (no comments)
//! lagrange-migrate import --from hexo --src ~/blog/source/_posts \
//!     --write markdown --out docs
//!
//! # Preview without writing
//! lagrange-migrate import --from zola --src ./content --dry-run
//! ```

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use tracing::info;

use lagrange_exchange::{
    markdown_dir,
    reader::Reader,
    wordpress::WordpressReader,
    writer::{
        archive_json::ArchiveJsonWriter,
        markdown::{Layout, MarkdownWriter},
        Writer,
    },
};

#[derive(Parser)]
#[command(
    name = "lagrange-migrate",
    version,
    about = "Migrate content from WordPress / Hexo / Zola / Hugo / Jekyll into lagrange"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Import from a foreign source and write to one or more sinks.
    Import {
        /// Source engine.
        #[arg(long, value_enum)]
        from: Source,

        /// WordPress: path to the WXR `.xml` export. Required when `--from wordpress`.
        #[arg(long)]
        wxr: Option<PathBuf>,

        /// Markdown engines: source directory of posts.
        #[arg(long)]
        src: Option<PathBuf>,

        /// Repeatable writer specification. Each `--write` takes a sink name
        /// and is immediately followed by `--out` for that sink.
        #[arg(long = "write", value_enum, num_args = 1)]
        writes: Vec<Sink>,

        /// Output directory for the next `--write` on the command line. The
        /// parser pairs each `--write` with the most recent `--out`.
        #[arg(long = "out", value_name = "DIR")]
        outs: Vec<PathBuf>,

        /// Language subdirectory the markdown writer targets (default "en").
        #[arg(long, default_value = "en")]
        lang: String,

        /// Use nested layout (posts/ pages/ boards/) for the markdown writer.
        #[arg(long)]
        nested: bool,

        /// Dry-run: parse and report counts, but write nothing.
        #[arg(long)]
        dry_run: bool,
    },

    /// List the supported source engines.
    List,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Source {
    #[value(name = "wordpress")]
    WordPress,
    #[value(name = "hexo")]
    Hexo,
    #[value(name = "hugo")]
    Hugo,
    #[value(name = "zola")]
    Zola,
    #[value(name = "jekyll")]
    Jekyll,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Sink {
    /// Article bodies → markdown files.
    #[value(name = "markdown")]
    Markdown,
    /// Detached comments → JSON archive files.
    #[value(name = "archive-json")]
    ArchiveJson,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("lagrange_migrate=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::List => {
            println!("Supported sources:");
            for s in [
                "wordpress  — WXR XML export (--wxr FILE)",
                "hexo       — _posts/ markdown, YAML frontmatter (--src DIR)",
                "hugo       — content/posts/ markdown, YAML frontmatter (--src DIR)",
                "zola       — content/ markdown, TOML frontmatter (+++) (--src DIR)",
                "jekyll     — _posts/ markdown, YAML frontmatter (--src DIR)",
            ] {
                println!("  {s}");
            }
            println!("\nSupported sinks (--write):");
            println!("  markdown      — article bodies → <out>/<lang>/*.md");
            println!("  archive-json  — detached comments → <out>/<node_id>.json");
            Ok(())
        }
        Command::Import {
            from,
            wxr,
            src,
            writes,
            outs,
            lang,
            nested,
            dry_run,
        } => run_import(ImportArgs {
            from,
            wxr,
            src,
            writes,
            outs,
            lang,
            nested,
            dry_run,
        }),
    }
}

/// Bundled arguments for `import`, kept off the signature so clippy's
/// `too_many_arguments` lint stays happy and the call site reads cleanly.
struct ImportArgs {
    from: Source,
    wxr: Option<PathBuf>,
    src: Option<PathBuf>,
    writes: Vec<Sink>,
    outs: Vec<PathBuf>,
    lang: String,
    nested: bool,
    dry_run: bool,
}

fn run_import(args: ImportArgs) -> Result<()> {
    let ImportArgs {
        from,
        wxr,
        src,
        writes,
        outs,
        lang,
        nested,
        dry_run,
    } = args;

    if !dry_run && writes.is_empty() {
        bail!("at least one --write sink is required (try --write markdown --out docs)");
    }
    // In dry-run, writers may be omitted; if given, still validate pairing.
    if !writes.is_empty() && writes.len() != outs.len() {
        bail!(
            "mismatched --write ({}) and --out ({}) counts: each writer needs its own --out",
            writes.len(),
            outs.len()
        );
    }

    let reader = build_reader(from, wxr, src)?;
    let docs = reader.read().context("read source")?;

    let article_count = docs.len();
    let comment_count: usize = docs.iter().map(|d| d.comments.len()).sum();
    info!(
        "importing from {}: {} document(s), {} comment(s)",
        reader.name(),
        article_count,
        comment_count
    );

    if dry_run {
        println!("dry-run: parsed {} document(s), {} comment(s) — nothing written", article_count, comment_count);
        for d in docs.iter().take(10) {
            println!(
                "  • {} [{}] ({} comment{})",
                d.frontmatter.title.as_deref().unwrap_or(&d.node_id),
                d.node_id,
                d.comments.len(),
                if d.comments.len() == 1 { "" } else { "s" }
            );
        }
        if docs.len() > 10 {
            println!("  … and {} more", docs.len() - 10);
        }
        return Ok(());
    }

    // Build writers from the (sink, out) pairs.
    let writers = build_writers(&writes, &outs, &lang, nested)?;
    for doc in &docs {
        for w in &writers {
            w.write(doc).with_context(|| {
                format!("write doc {} via {}", doc.node_id, w.name())
            })?;
        }
    }

    info!(
        "done: wrote {} document(s) to {} sink(s)",
        article_count,
        writers.len()
    );
    Ok(())
}

fn build_reader(
    from: Source,
    wxr: Option<PathBuf>,
    src: Option<PathBuf>,
) -> Result<Box<dyn Reader>> {
    match from {
        Source::WordPress => {
            let path = wxr.context("wordpress requires --wxr <file.xml>")?;
            Ok(Box::new(WordpressReader::new(path)))
        }
        Source::Hexo => {
            let dir = src.context("hexo requires --src <dir>")?;
            Ok(Box::new(markdown_dir::hexo(dir)))
        }
        Source::Hugo => {
            let dir = src.context("hugo requires --src <dir>")?;
            Ok(Box::new(markdown_dir::hugo(dir)))
        }
        Source::Zola => {
            let dir = src.context("zola requires --src <dir>")?;
            Ok(Box::new(markdown_dir::zola(dir)))
        }
        Source::Jekyll => {
            let dir = src.context("jekyll requires --src <dir>")?;
            Ok(Box::new(markdown_dir::jekyll(dir)))
        }
    }
}

fn build_writers(
    writes: &[Sink],
    outs: &[PathBuf],
    lang: &str,
    nested: bool,
) -> Result<Vec<Box<dyn Writer>>> {
    let mut writers = Vec::new();
    for (sink, out) in writes.iter().zip(outs.iter()) {
        match sink {
            Sink::Markdown => {
                let w = MarkdownWriter {
                    out: out.clone(),
                    layout: if nested { Layout::Nested } else { Layout::Flat },
                    default_lang: lang.to_string(),
                };
                writers.push(Box::new(w) as Box<dyn Writer>);
            }
            Sink::ArchiveJson => {
                writers.push(Box::new(ArchiveJsonWriter::new(out.clone())) as Box<dyn Writer>);
            }
        }
    }
    Ok(writers)
}

// Allow `--from wordpress` to require `--wxr`. clap's `requires` needs a
// named group; we approximate with a value-enum + manual context() check.
#[allow(dead_code)]
fn _from_wordpress_marker() {}
