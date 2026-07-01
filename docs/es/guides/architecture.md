# Arquitectura

Lagrange es un único crate (`lagrange-library`) que proporciona una biblioteca
y una CLI `lagrange`.

## Tubería

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## Módulos

| Módulo | Responsabilidad |
|--------|----------------|
| `markdown` | gramática pest, AST, analizador de bloques + en línea |
| `render` | AST -> tairitsu `VNode` -> cadena HTML |
| `theme` | CSS generado desde la paleta hikari |
| `site` | recorre el árbol docs, renderiza cada página, escribe un sitio estático |
| `cli` | el binario `lagrange` |

## El ciclo cerrado

Lagrange renderiza su propia documentación: `just docs` ejecuta
`lagrange build docs`, y el flujo de trabajo de CI despliega el resultado. Si estás leyendo
esto en el sitio publicado, fue producido por Lagrange.

## Construido sobre

- [tairitsu](https://github.com/celestia-island/tairitsu) — el DOM virtual
  utilizado como IR de renderizado.
- [hikari](https://github.com/celestia-island/hikari) — la paleta de colores utilizada
  para la tematización.
- [pest](https://pest.rs) — el generador de analizadores.
