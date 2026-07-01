<h1 align="center">lagrange</h1>

<p align="center"><strong>A pest-based markdown documentation rendering facility.</strong></p>

<p align="center">
  Lagrange parses markdown with a pest grammar, renders it to HTML through the
  <a href="https://github.com/celestia-island/tairitsu">tairitsu</a> virtual DOM,
  and assembles a multilingual static site — themed with the
  <a href="https://github.com/celestia-island/hikari">hikari</a> palette.
</p>

<p align="center">
[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](./LICENSE)
</p>

<p align="center">
<a href="./docs/en/index.md">English</a> ·
<a href="./docs/zhs/index.md">简体中文</a> ·
<a href="./docs/zht/index.md">繁體中文</a> ·
<a href="./docs/ja/index.md">日本語</a> ·
<a href="./docs/ko/index.md">한국어</a> ·
<a href="./docs/fr/index.md">Français</a> ·
<a href="./docs/es/index.md">Español</a> ·
<a href="./docs/ru/index.md">Русский</a> ·
<a href="./docs/ar/index.md">العربية</a>
</p>

## The closed loop

Lagrange renders **its own documentation** — this README's sibling `docs/`
tree is built by Lagrange itself:

```bash
just docs          # = lagrange build --src docs --out target/site
```

If you are reading the published site, it was produced by Lagrange.

## Features

- **pest-based markdown parser** — block + inline, modelled on
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown).
- **tairitsu VDom rendering** — documents become `VNode` trees, serialised via
  `VNode::render_to_html`.
- **Multilingual** — one directory per language, a built-in language switcher,
  and a root redirect to English.
- **hikari theming** — the stylesheet's base colours come from the hikari
  palette.

## Quick start

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

Open `target/site/index.html`.

## Directory layout

```
docs/
├── en/
│   ├── SUMMARY.md      # drives the sidebar
│   ├── index.md
│   └── guides/*.md
└── <lang>/ …           # one directory per language
```

## Modules

| Module | Responsibility |
|--------|----------------|
| `markdown` | pest grammar, AST, block + inline parser |
| `render` | AST → tairitsu `VNode` → HTML string |
| `theme` | CSS generated from the hikari palette |
| `site` | walks the docs tree, renders pages, writes a static site |
| `cli` | the `lagrange` binary |

## Development

```bash
just ci          # fmt-check + clippy + test
just docs        # build the docs site with lagrange itself
```

> Lagrange depends on the in-tree sibling crates `tairitsu-vdom` and
> `hikari-palette` (path dependencies `../tairitsu` and `../hikari`). The CI
> workflows check those repositories out alongside Lagrange to satisfy them.

## License

SySL-1.0 (Synthetic Source License). See [LICENSE](./LICENSE).
