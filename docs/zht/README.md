<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>基於 WASI 繪製的 Markdown 靜態網站設施 — 開箱即用多語言</strong></p>

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

Lagrange 將一個 Markdown 資料夾轉換為靜態的多語言文件站點——以 WASI 繪製，內建語言切換器並將根目錄重定向到英文。無需 JavaScript 框架、無需 mdBook、無需 Node 工具鏈。

Lagrange 渲染**自身的文件**：此 README 旁邊的 `docs/` 目錄樹即由 Lagrange
自身建構（`just docs`）。如果你正在閱讀發布的站點，那麼它正是由 Lagrange 產生的。

## 快速開始

為當前倉庫建構站點：

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

打開 `target/site/index.html`——它會重定向到英文書籍。

將 Lagrange 指向任何具有相同結構的文件目錄樹：

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

`README.md` 和 `index.md` 都會映射到 `index.html`；建議將 `docs/en/README.md`
符號連結到根目錄的 `README.md`，以保持 GitHub 著陸頁和文件索引同步。

## 部署站點

Lagrange 輸出純靜態目錄，因此可以部署到任何地方。建構指令僅為
`lagrange build --src docs --out <dir>`（從原始碼使用時則為
`cargo run --release -- build …`）。

### GitHub Actions → GitHub Pages

一個現成的複合 action 會在你的倉庫旁檢出 Lagrange 並執行建構。
這也是 Lagrange 自身使用的流程
（[`.github/workflows/docs.yml`](../../.github/workflows/docs.yml)）：

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

建構指令 `cargo run --release -- build --src docs --out _site`，建構輸出目錄
`_site`。若要使用預先建構好的 Lagrange，鏡像
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange) 並執行
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`。
`--site_url` 為可選項，僅影響語言切換器中的絕對連結。

### Vercel

框架預設為 **Other**，建構指令
`cargo run --release -- build --src docs --out public`，輸出目錄 `public`。
（Vercel 需要 Rust 工具鏈——使用
[`vercel-rust`](https://github.com/vercel-community/rust) 運行時建構器，
或在先前的 CI 步驟中建構後部署靜態輸出。）

## 特性

- **基於 pest 的 Markdown 解析器**——區塊 + 行內，以
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown) 為藍本，
  外加原始 HTML 區塊透傳，使居中對齊的 README 標記能夠原樣保留。
- **tairitsu VDom 渲染**——文件轉換為 `VNode` 樹，通過 `render_to_html` 序列化輸出。
- **多語言**——每種語言一個目錄、內置語言切換器，以及根目錄重定向到英文。
- **自託管**——Lagrange 自身的文件由 Lagrange 建構。

## 許可

SySL-1.0（Synthetic Source License）。詳見 [LICENSE](../../LICENSE)。
