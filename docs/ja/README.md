<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>pest ベースの Markdown ドキュメントレンダラー — tairitsu VDom + hikari パレット、多言語対応済。</strong></p>

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

Lagrange は Markdown フォルダを多言語の静的ドキュメントサイトに変換します。
[pest](https://pest.rs) 文法で Markdown を解析し、[tairitsu](https://github.com/celestia-island/tairitsu)
仮想 DOM で HTML に、[hikari](https://github.com/celestia-island/hikari)
パレットでテーマを適用します。言語ごとに 1 ディレクトリ、組み込みの言語スイッチャー、
ルートから英語へのリダイレクト——JavaScript フレームワークも mdBook も Node ツール
チェーンも不要です。

Lagrange 自身のドキュメントも Lagrange でビルドされています（`just docs`）。

## クイックスタート

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

## デプロイ

静的なディレクトリを出力するため、**GitHub Pages**（組み込み composite action）・
**Cloudflare Pages**・**Vercel** にそのままデプロイできます。

## 特徴

- **pest ベースのパーサー**（ブロック＋インライン、raw HTML ブロック透過）
- **tairitsu VDom レンダリング**
- **多言語**（言語ごと 1 ディレクトリ、言語スイッチャー内蔵）
- **セルフホスティング**（Lagrange のドキュメントは自身でビルド）

## ライセンス

SySL-1.0（Synthetic Source License）。[LICENSE](../../LICENSE) を参照。
