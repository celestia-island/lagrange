<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>一个基于 pest 的 Markdown 文档渲染器——tairitsu VDom + hikari 调色板，开箱即用多语言。</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-lagrange.docs.celestia.world-blue)](https://lagrange.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
**简体中文** ·
[繁體中文](../zht/README.md) ·
[日本語](../ja/README.md) ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## 简介

Lagrange 把一个 Markdown 文件夹变成多语言静态文档站点。它用自己编写的
[pest](https://pest.rs) 语法解析 Markdown，通过 [tairitsu](https://github.com/celestia-island/tairitsu)
虚拟 DOM 将 AST 渲染为 HTML，并用 [hikari](https://github.com/celestia-island/hikari)
调色板进行主题美化。每种语言一个目录、内置语言切换器、根重定向到英文——无需
JavaScript 框架、无需 mdBook、无需 Node 工具链。

Lagrange 用它自身渲染自己的文档：README 旁边的 `docs/` 目录树就是 Lagrange 构建的
（`just docs`）。

## 快速开始

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

打开 `target/site/index.html`。

指向任意同形文档树：

```
docs/
├── logo.webp          # 共享资源在 docs 根
├── en/
│   ├── README.md      # → <site>/en/index.html
│   ├── SUMMARY.md     # 驱动侧边栏
│   └── guides/*.md
└── zhs/ …             # 每种语言一个目录
```

```bash
lagrange build --src docs --out _site
```

## 部署

输出是纯静态目录，可以部署到 **GitHub Pages**（使用内置复合 action）、
**Cloudflare Pages** 或 **Vercel**。详见根 [README](../../README.md#deploying-the-site)。

## 特性

- **pest 语法解析**——块级 + 行内，以 ratatui-markdown 为蓝本，支持 raw HTML 透传
  （居中 README 标签可以原样通过）。
- **tairitsu VDom 渲染**——文档变为 `VNode` 树，通过 `render_to_html` 序列化。
- **多语言**——每种语言一个目录、内置语言切换器、根重定向到英文。
- **自托管**——Lagrange 自身的文档由 Lagrange 构建。

## 许可

SySL-1.0（Synthetic Source License）。见 [LICENSE](../../LICENSE)。
