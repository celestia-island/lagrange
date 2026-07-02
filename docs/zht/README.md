<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>一個基於 pest 的 Markdown 文件渲染器——tairitsu VDom + hikari 調色板，開箱即用多語言。</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-lagrange.docs.celestia.world-blue)](https://lagrange.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
**繁體中文** ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## 簡介

Lagrange 將一個 Markdown 文件夾變成多語言靜態文件站點。它用自己編寫的
[pest](https://pest.rs) 語法解析 Markdown，通過 [tairitsu](https://github.com/celestia-island/tairitsu)
虛擬 DOM 將 AST 渲染為 HTML，並用 [hikari](https://github.com/celestia-island/hikari)
調色板進行主題美化。每種語言一個目錄、內置語言切換器、根重定向到英文——無需
JavaScript 框架、無需 mdBook、無需 Node 工具鏈。

Lagrange 用它自身渲染自己的文件：README 旁邊的 `docs/` 目錄樹就是 Lagrange 構建的
（`just docs`）。

## 快速開始

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

打開 `target/site/index.html`。

詳細用法與部署（GitHub Pages / Cloudflare Pages / Vercel）見根 [README](../../README.md#deploying-the-site)。

## 特性

- **pest 語法解析**——區塊級 + 行內，以 ratatui-markdown 為藍本，支援 raw HTML 透傳。
- **tairitsu VDom 渲染**。
- **多語言**——每種語言一個目錄、內置語言切換器、根重定向到英文。
- **自託管**——Lagrange 自身的文件由 Lagrange 構建。

## 許可

SySL-1.0（Synthetic Source License）。見 [LICENSE](../../LICENSE)。
