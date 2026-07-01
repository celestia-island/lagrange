# Quick Start

## Build a site

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

Open `target/site/index.html` — it redirects to the English book.

## Directory layout

```
docs/
├── logo.webp            # shared assets live at the docs root
├── en/
│   ├── README.md        # -> ../../README.md (the repo landing page)
│   ├── SUMMARY.md       # drives the sidebar
│   └── guides/*.md
├── zhs/
│   ├── README.md        # one translated landing page per language
│   └── ...
```

Each subdirectory of `docs/` is one language. `SUMMARY.md` drives the sidebar.
`README.md` and `index.md` both map to `index.html`; symlinking
`docs/en/README.md` to the root `README.md` keeps GitHub and the docs site in
sync.

## Markdown support

Headings, paragraphs, fenced code, lists, blockquotes, tables, thematic breaks,
images, inline `code`, **bold**, *italic*, [links](./architecture.md), and raw
HTML blocks (so a centred `<p align="center">…</p>` in a README passes through
untouched).

> Blockquotes are supported too.

## Deploying

The build emits a static directory — see the root
[README](./README.md#deploying-the-site) for GitHub Pages, Cloudflare Pages and
Vercel recipes.
