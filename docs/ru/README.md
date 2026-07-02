<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>Инструмент рендеринга Markdown-документации на основе pest — VDom tairitsu + палитра hikari, многоязычный из коробки.</strong></p>

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
[Español](../es/README.md) ·
**Русский** ·
[العربية](../ar/README.md)

</div>

## Введение

Lagrange превращает папку с Markdown-файлами в многоязычный статический
документационный сайт. Парсинг осуществляется собственной грамматикой
[pest](https://pest.rs), AST рендерится в HTML через виртуальный DOM
[tairitsu](https://github.com/celestia-island/tairitsu), темы берутся из
палитры [hikari](https://github.com/celestia-island/hikari). По одному
каталогу на язык, встроенный переключатель языков, корневой редирект на
английский — без JavaScript-фреймворков, без mdBook, без Node.

## Быстрый старт

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

## Развёртывание

Вывод — статический каталог. Разворачивается на **GitHub Pages** (встроенный
composite action), **Cloudflare Pages** или **Vercel**.

## Возможности

- **Парсер Markdown на основе pest** (блоки+inline, сырой HTML сохранён)
- **Рендеринг VDom tairitsu**
- **Многоязычный** — каталог на язык, встроенный переключатель
- **Самодокументирование** — документация Lagrange собрана самим Lagrange

## Лицензия

SySL-1.0 (Synthetic Source License). См. [LICENSE](../../LICENSE).
