<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>Un motor de renderizado de documentación Markdown basado en pest — VDom tairitsu + paleta hikari, multilingüe desde el primer momento.</strong></p>

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

Lagrange convierte una carpeta Markdown en un sitio de documentación estático
multilingüe. Analiza Markdown con una gramática [pest](https://pest.rs) escrita
a mano, renderiza el AST a HTML mediante el DOM virtual de
[tairitsu](https://github.com/celestia-island/tairitsu) y aplica temas con la
paleta [hikari](https://github.com/celestia-island/hikari). Un directorio por
idioma, un selector de idioma integrado y una redirección raíz al inglés — sin
frameworks JavaScript, sin mdBook, sin cadena de herramientas Node.

## Inicio rápido

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

## Despliegue

El directorio de salida es estático — se puede desplegar en **GitHub Pages**
(acción compuesta incluida), **Cloudflare Pages** o **Vercel**.

## Características

- **Parser Markdown basado en pest** (bloques+inline, HTML bruto preservado)
- **Renderizado VDom tairitsu**
- **Multilingüe** — un directorio por idioma, selector integrado
- **Auto-alojado** — la documentación de Lagrange se construye con Lagrange

## Licencia

SySL-1.0 (Synthetic Source License). Ver [LICENSE](../../LICENSE).
