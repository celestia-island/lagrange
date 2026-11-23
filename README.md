<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/celestia-island.github.io/main/res/logos/lagrange.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>An agent-ready documentation knowledge base — Markdown in, citations out, MCP-native</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![crates.io](https://img.shields.io/crates/v/lagrange-library)](https://crates.io/crates/lagrange-library)
[![Docs](https://img.shields.io/badge/docs-lagrange.docs.celestia.world-blue)](https://lagrange.docs.celestia.world)

</div>

<div align="center">

**English** ·
[简体中文](./docs/zh-Hans/README.md) ·
[繁體中文](./docs/zh-Hant/README.md) ·
[日本語](./docs/ja/README.md) ·
[한국어](./docs/ko/README.md) ·
[Français](./docs/fr/README.md) ·
[Español](./docs/es/README.md) ·
[Русский](./docs/ru/README.md) ·
[العربية](./docs/ar/README.md)

</div>

Lagrange turns a folder of markdown into a static, multilingual documentation
site — WASI-rendered, with a built-in language switcher and root redirect to
English. No JavaScript framework, no mdBook, no Node toolchain.

Lagrange renders **its own documentation**: the `docs/` tree next to this README
is built by Lagrange itself (`just docs`). If you are reading the published
site, it was produced by Lagrange.

## Quick start

Build the site for this very repo:

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

Open `target/site/index.html` — it redirects into the English book.

Point Lagrange at any docs tree with the same shape:

```
docs/
├── logo.webp          # shared assets live at the docs root
├── en/
│   ├── README.md      # becomes <site>/en/index.html
│   ├── SUMMARY.md     # drives the sidebar
│   └── guides/*.md
└── zh-Hans/ …             # one directory per language
```

```bash
lagrange build --src docs --out _site
```

`README.md` and `index.md` both map to `index.html`; a `docs/en/README.md`
symlink to the root `README.md` is the recommended way to keep the GitHub
landing page and the docs index in sync.

## Agent knowledge base

Lagrange is being repositioned as an **agent-ready documentation knowledge base**: the
authoritative source is the git/noa Markdown tree, and the build pipeline produces both
a static site and a `knowledge.json` file (documents, translations, revisions, chunks
with source-coordinate spans) that downstream agents consume via the `lagrange dev --mcp`
tool surface.

### MCP tools (via `lagrange dev --mcp`)

| Tool | Purpose |
|---|---|
| `docs.search` | Chunk-level search with citations (five-tuple: document_id / lang / revision / span / anchor) |
| `docs.fetch` | Fetch a chunk by citation; fails with `STALE:` if the revision has changed |
| `docs.tree` | Document tree with translations and freshness |
| `docs.write` | Write back Markdown (scope-limited, identity-guarded) |
| `docs.status` | Build diagnostics (errors, broken links, missing translations) |
| `build.trigger` | Trigger a rebuild |

### Webui

The `packages/webui/` directory contains a Vue 3 + Vite app that renders the knowledge
base with hikari components (HkFaceShell / HkMenu / HkMarkdownRenderer) for human
consumption, plus a console tab with build diagnostics.


## Deploying the site

Lagrange emits a plain static directory, so it deploys anywhere. The build is
just `lagrange build --src docs --out <dir>` (or `cargo run --release -- build …`
when consumed from source).

### GitHub Actions → GitHub Pages

A ready-made composite action checks Lagrange out alongside your repo and runs
the build. This is what Lagrange itself uses
([`.github/workflows/docs.yml`](./.github/workflows/docs.yml)):

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: celestia-island/lagrange/.github/actions/build@dev
        with:
          src: docs
          out: _site
      - run: echo "your-project.docs.celestia.world" > _site/CNAME
      - uses: actions/upload-pages-artifact@v3
        with:
          path: _site
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/deploy-pages@v4
```

### Cloudflare Pages

Build command `cargo run --release -- build --src docs --out _site`, build output
directory `_site`. For a prebuilt Lagrange, mirror
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange) and run
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`.
`--site_url` is optional and only affects absolute links in the language
switcher.

### Vercel

Framework preset **Other**, build command
`cargo run --release -- build --src docs --out public`, output directory
`public`. (Vercel needs the Rust toolchain — use the
[`vercel-rust`](https://github.com/vercel-community/rust) runtime builder, or
build in a prior CI step and deploy the static output.)

## Releases

### npm

The lagrange scaffolder is published to npm as `@celestia-island/lagrange` and invoked with:

```bash
npx @celestia-island/lagrange
```

It downloads the prebuilt `lagrange` binary for your platform from the latest
GitHub Release and runs `lagrange init`. Publishing to npm is triggered by
pushing an `npm-v*` tag (e.g. `npm-v0.1.1`); plain `v*` tags drive the binary
archives (`release.yml`) and the crates.io crate (`publish.yml`).

## Features

> ⚠️ The comment system is **experimental** — the `proxied` delivery mode has a known
> gap (the runtime does not recognise it) and the GitHub Discussions adapter is a skeleton.
> Only the `static-json` archive mode is functional. See the ledger for details.


- **pest-based markdown parser** — block + inline, modelled on
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown), plus
  raw-HTML-block pass-through so centred README markup survives.
- **tairitsu VDom rendering** — documents become `VNode` trees serialised via
  `render_to_html`.
- **Multilingual** — one directory per language, a built-in language switcher,
  and a root redirect to English.
- **Self-hosting** — Lagrange's own documentation is built by Lagrange.

## License

SySL-1.0 (Synthetic Source License). See [LICENSE](./LICENSE) or the [SySL website](https://sysl.celestia.world).
