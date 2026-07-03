# Architecture

Lagrange is a single crate (`lagrange-library`) shipping a library and a
`lagrange` CLI.

## Pipeline

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## Modules

| Module | Responsibility |
|--------|----------------|
| `markdown` | pest grammar, AST, block + inline parser |
| `render` | AST -> tairitsu `VNode` -> HTML string |
| `theme` | CSS generated from the hikari palette |
| `site` | walks the docs tree, renders each page, writes a static site |
| `cli` | the `lagrange` binary |

## The closed loop

Lagrange renders its own documentation: `just docs` runs
`lagrange build docs`, and the CI workflow deploys the result. If you are
reading this on the published site, it was produced by Lagrange.

## Built on

- [tairitsu](https://github.com/celestia-island/tairitsu) — the virtual DOM
  used as the rendering IR.
- [hikari](https://github.com/celestia-island/hikari) — the colour palette used
  for theming.
- [pest](https://pest.rs) — the parser generator.
