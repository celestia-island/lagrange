# Lagrange

**A pest-based markdown documentation rendering facility.**

Lagrange parses markdown with a pest grammar, renders it to HTML through the
[tairitsu](https://github.com/celestia-island/tairitsu) virtual DOM, and
assembles a multilingual static site — themed with the
[hikari](https://github.com/celestia-island/hikari) palette.

> This very page is rendered by Lagrange itself.

## Features

- **pest-based markdown parser** — block + inline, modelled on ratatui-markdown.
- **tairitsu VDom rendering** — documents become `VNode` trees, serialised via
  `render_to_html`.
- **Multilingual** — one directory per language, a built-in language switcher,
  and a root redirect to English.
- **Self-hosting** — Lagrange's own documentation is built by Lagrange
  (`just docs`).

## Quick start

```bash
cargo run --release -- build docs --out target/site
```

See [Quick Start](./guides/quickstart.md) and [Architecture](./guides/architecture.md).
