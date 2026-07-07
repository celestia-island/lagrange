<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/lagrange/master/docs/logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>A WASI-rendered Markdown static site facility — multilingual out of the box</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![crates.io](https://img.shields.io/crates/v/lagrange-library)](https://crates.io/crates/lagrange-library)
[![Docs](https://img.shields.io/badge/docs-lagrange.docs.celestia.world-blue)](https://lagrange.docs.celestia.world)

</div>

<div align="center">

**English** ·
[简体中文](./docs/zhs/README.md) ·
[繁體中文](./docs/zht/README.md) ·
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
└── zhs/ …             # one directory per language
```

```bash
lagrange build --src docs --out _site
```

`README.md` and `index.md` both map to `index.html`; a `docs/en/README.md`
symlink to the root `README.md` is the recommended way to keep the GitHub
landing page and the docs index in sync.

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

## Features

- **pest-based markdown parser** — block + inline, modelled on
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown), plus
  raw-HTML-block pass-through so centred README markup survives.
- **tairitsu VDom rendering** — documents become `VNode` trees serialised via
  `render_to_html`.
- **Multilingual** — one directory per language, a built-in language switcher,
  and a root redirect to English.
- **Self-hosting** — Lagrange's own documentation is built by Lagrange.

## License

SySL-1.0 (Synthetic Source License). See [LICENSE](https://sysl.celestia.world).
