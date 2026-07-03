<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>Un motor de renderizado de documentación Markdown basado en pest — VDom tairitsu + paleta hikari, multilingüe desde el primer momento</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-lagrange.docs.celestia.world-blue)](https://lagrange.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
[繁體中文](../zht/README.md) ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
**Español** ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## Introducción

Lagrange convierte una carpeta de archivos Markdown en un sitio de documentación
estático y multilingüe. Analiza el Markdown con una gramática
[pest](https://pest.rs) escrita a mano, renderiza el AST a HTML mediante el DOM
virtual de [tairitsu](https://github.com/celestia-island/tairitsu) y aplica temas
con la paleta [hikari](https://github.com/celestia-island/hikari). Un directorio
por idioma, un selector de idioma integrado y una redirección raíz al inglés —
sin frameworks JavaScript, sin mdBook, sin cadena de herramientas Node.

Lagrange renderiza **su propia documentación**: el árbol `docs/` junto a este
README es construido por Lagrange mismo (`just docs`). Si estás leyendo el sitio
publicado, fue generado por Lagrange.

## Inicio rápido

Construye el sitio para este mismo repositorio:

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

Abre `target/site/index.html` — redirige al libro en inglés.

Apunta Lagrange a cualquier árbol de documentación con la misma estructura:

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

`README.md` e `index.md` se mapean ambos a `index.html`; un enlace simbólico de
`docs/en/README.md` hacia el `README.md` raíz es la forma recomendada de
mantener sincronizadas la página de inicio de GitHub y el índice de la
documentación.

## Desplegando el sitio

Lagrange produce un directorio estático simple, por lo que se puede desplegar en
cualquier lugar. La construcción es simplemente
`lagrange build --src docs --out <dir>` (o
`cargo run --release -- build …` cuando se compila desde el código fuente).

### GitHub Actions → GitHub Pages

Una acción compuesta lista para usar obtiene Lagrange junto a tu repositorio y
ejecuta la construcción. Esto es lo que usa el propio Lagrange
([`.github/workflows/docs.yml`](../../.github/workflows/docs.yml)):

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

Comando de construcción
`cargo run --release -- build --src docs --out _site`, directorio de salida
`_site`. Para un Lagrange precompilado, haz un mirror de
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange) y
ejecuta
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`.
`--site_url` es opcional y solo afecta los enlaces absolutos en el selector de
idioma.

### Vercel

Preajuste de framework **Other**, comando de construcción
`cargo run --release -- build --src docs --out public`, directorio de salida
`public`. (Vercel necesita la cadena de herramientas de Rust — usa el runtime
builder [`vercel-rust`](https://github.com/vercel-community/rust), o construye en
un paso CI previo y despliega la salida estática.)

## Características

- **Parser Markdown basado en pest** — bloques + inline, inspirado en
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown), además
  de paso directo de bloques HTML crudos para que el marcado centrado de los
  README se conserve.
- **Renderizado VDom tairitsu** — los documentos se convierten en árboles
  `VNode` serializados mediante `render_to_html`.
- **Multilingüe** — un directorio por idioma, un selector de idioma integrado y
  una redirección raíz al inglés.
- **Auto-alojado** — la propia documentación de Lagrange es construida por
  Lagrange.

## Licencia

SySL-1.0 (Synthetic Source License). Ver [LICENSE](../../LICENSE).
