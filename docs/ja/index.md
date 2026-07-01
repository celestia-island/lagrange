# Lagrange

**pest ベースの Markdown ドキュメントレンダリング基盤。**

Lagrange は pest 文法で Markdown を解析し、
[tairitsu](https://github.com/celestia-island/tairitsu) 仮想 DOM を介して HTML にレンダリングし、
[hikari](https://github.com/celestia-island/hikari) パレットでテーマ化された
多言語静的サイトを組み立てます。

> このページ自体も Lagrange によってレンダリングされています。

## 特徴

- **pest ベースの Markdown パーサー** —— ブロック + インライン、ratatui-markdown をモデルにしています。
- **tairitsu VDom レンダリング** —— ドキュメントは `VNode` ツリーになり、
  `render_to_html` でシリアライズされます。
- **多言語対応** —— 言語ごとに 1 つのディレクトリ、組み込みの言語切り替え機能、
  そして英語へのルートリダイレクト。
- **自己ホスト** —— Lagrange 自身のドキュメントは Lagrange が
  （`just docs` で）ビルドします。

## クイックスタート

```bash
cargo run --release -- build docs --out target/site
```

[クイックスタート](./guides/quickstart.md)と[アーキテクチャ](./guides/architecture.md)を参照してください。
