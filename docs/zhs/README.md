<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/lagrange/master/docs/logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>基于 WASI 渲染的 Markdown 静态站点设施 — 开箱即用多语言</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-lagrange.celestia.world-blue)](https://lagrange.celestia.world)

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

Lagrange 把一个 Markdown 文件夹变成静态的多语言文档站点 —— WASI 渲染，
内置语言切换器，根重定向到英文。无需 JavaScript 框架、无需 mdBook、
无需 Node 工具链。

Lagrange 用它自身渲染自己的文档：README 旁边的 `docs/` 目录树就是 Lagrange 构建的
（`just docs`）。如果你正在阅读已发布的站点，它就是由 Lagrange 生成的。

## 快速开始

为本仓库构建站点：

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

打开 `target/site/index.html`——它会重定向到英文文档。

将 Lagrange 指向任意同形文档树：

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

`README.md` 和 `index.md` 都会被映射为 `index.html`；推荐将 `docs/en/README.md`
创建为指向根目录 `README.md` 的软链接，这样可以保持 GitHub 着陆页与文档索引同步。

## 部署站点

Lagrange 输出纯静态目录，因此可以部署到任何地方。构建命令就是
`lagrange build --src docs --out <dir>`（从源码使用时为 `cargo run --release -- build …`）。

### GitHub Actions → GitHub Pages

提供了一个即用型复合 action，它会拉取 Lagrange 到你的仓库旁边并运行构建。Lagrange
自身用的就是它（[`../../.github/workflows/docs.yml`](../../.github/workflows/docs.yml)）：

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

构建命令 `cargo run --release -- build --src docs --out _site`，构建输出目录
`_site`。如果使用预构建的 Lagrange，镜像
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange) 仓库并运行
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`。
`--site_url` 是可选的，仅影响语言切换器中的绝对链接。

### Vercel

框架预设选 **Other**，构建命令
`cargo run --release -- build --src docs --out public`，输出目录 `public`。
（Vercel 需要 Rust 工具链——使用
[`vercel-rust`](https://github.com/vercel-community/rust) 运行时构建器，或在上游
CI 步骤中构建后部署静态输出。）

## 特性

- **基于 pest 的 Markdown 解析器**——块级 + 行内，以
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown) 为蓝本，
  同时支持原始 HTML 块透传，使居中 README 标记能够原样保留。
- **tairitsu VDom 渲染**——文档变为 `VNode` 树，通过 `render_to_html` 序列化。
- **多语言**——每种语言一个目录、内置语言切换器、根重定向到英文。
- **自托管**——Lagrange 自身的文档由 Lagrange 构建。

## 许可

SySL-1.0（Synthetic Source License）。见 [LICENSE](https://sysl.celestia.world)。
