<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>pest ベースの Markdown ドキュメントレンダラー — tairitsu VDom + hikari パレット、多言語対応済</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-lagrange.docs.celestia.world-blue)](https://lagrange.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
[繁體中文](../zht/README.md) ·
**日本語** ·
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## 概要

Lagrange は Markdown のフォルダを静的で多言語対応のドキュメントサイトに変換します。
独自に記述した [pest](https://pest.rs) 文法で Markdown を解析し、AST を
[tairitsu](https://github.com/celestia-island/tairitsu) 仮想 DOM で HTML にレンダリングし、
[hikari](https://github.com/celestia-island/hikari) パレットでテーマを適用します。
言語ごとに 1 ディレクトリ、組み込みの言語スイッチャー、ルートから英語へのリダイレクト —
JavaScript フレームワークも mdBook も Node ツールチェーンも不要です。

Lagrange は**自身のドキュメント**をビルドします: この README と同階層の `docs/` ツリーは
Lagrange 自身によって生成されています (`just docs`)。公開サイトをご覧の場合、それは Lagrange
によって生成されたものです。

## クイックスタート

このリポジトリのサイトをビルドするには:

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

`target/site/index.html` を開くと、英語版のドキュメントにリダイレクトされます。

同じ構造を持つ任意のドキュメントツリーに対して Lagrange を使用できます:

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

`README.md` と `index.md` はどちらも `index.html` にマッピングされます。GitHub の
ランディングページとドキュメントのインデックスを同期させるには、ルートの `README.md`
へのシンボリックリンク `docs/en/README.md` を作成するのが推奨される方法です。

## デプロイ

Lagrange は純粋な静的ディレクトリを出力するため、あらゆる場所にデプロイできます。
ビルドは単に `lagrange build --src docs --out <dir>` です（ソースから利用する場合は
`cargo run --release -- build …`）。

### GitHub Actions → GitHub Pages

すぐに使える composite アクションがリポジトリと共に Lagrange をチェックアウトして
ビルドを実行します。以下は Lagrange 自身が使用しているものです
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

ビルドコマンド `cargo run --release -- build --src docs --out _site`、ビルド出力
ディレクトリ `_site`。事前ビルドされた Lagrange を使用する場合は、
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange) をミラーして
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`
を実行します。`--site_url` はオプションで、言語スイッチャー内の絶対リンクにのみ影響します。

### Vercel

フレームワークプリセット **Other**、ビルドコマンド
`cargo run --release -- build --src docs --out public`、出力ディレクトリ `public`。
（Vercel は Rust ツールチェーンが必要です —
[`vercel-rust`](https://github.com/vercel-community/rust) ランタイムビルダーを使用するか、
先行する CI ステップでビルドし、静的出力をデプロイしてください。）

## 特徴

- **pest ベースの Markdown パーサー** — ブロック＋インライン、
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown) をモデルにし、
  生の HTML ブロックも透過的に扱うため、中央揃えの README マークアップもそのまま残ります。
- **tairitsu VDom レンダリング** — ドキュメントは `VNode` ツリーに変換され、
  `render_to_html` によってシリアライズされます。
- **多言語対応** — 言語ごとに 1 ディレクトリ、言語スイッチャー内蔵、ルートから英語への
  リダイレクト。
- **セルフホスティング** — Lagrange のドキュメントは Lagrange 自身によって
  ビルドされています。

## ライセンス

SySL-1.0 (Synthetic Source License)。[LICENSE](../../LICENSE) を参照してください。
