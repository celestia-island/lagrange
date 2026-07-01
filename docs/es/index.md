# Lagrange

**Un sistema de renderizado de documentación markdown basado en pest.**

Lagrange analiza el markdown con una gramática pest, lo renderiza a HTML a
través del DOM virtual [tairitsu](https://github.com/celestia-island/tairitsu),
y ensambla un sitio estático multilingüe — tematizado con la paleta
[hikari](https://github.com/celestia-island/hikari).

> Esta misma página es renderizada por el propio Lagrange.

## Características

- **Analizador markdown basado en pest** — bloques + en línea, modelado a partir de ratatui-markdown.
- **Renderizado VDom tairitsu** — los documentos se convierten en árboles `VNode`, serializados vía
  `render_to_html`.
- **Multilingüe** — un directorio por idioma, un selector de idioma integrado,
  y una redirección raíz al inglés.
- **Autohospedado** — la propia documentación de Lagrange es construida por Lagrange
  (`just docs`).

## Inicio rápido

```bash
cargo run --release -- build docs --out target/site
```

Consulta [Inicio rápido](./guides/quickstart.md) y [Arquitectura](./guides/architecture.md).
