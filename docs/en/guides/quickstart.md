# Quick Start

## Build a site

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

Open `target/site/index.html` — it redirects to the English book.

## Directory layout

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zhs/
│   └── ...
```

Each subdirectory of `docs/` is one language. `SUMMARY.md` drives the sidebar.

## Markdown support

Headings, paragraphs, fenced code, lists, blockquotes, tables, thematic breaks,
images, and inline `code`, **bold**, *italic*, [links](./architecture.md).

> Blockquotes are supported too.
