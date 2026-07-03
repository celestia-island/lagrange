# アーキテクチャ

Lagrange は単一の crate（`lagrange-library`）で、ライブラリと
`lagrange` CLI を提供します。

## パイプライン

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## モジュール

| モジュール | 役割 |
|--------|----------------|
| `markdown` | pest 文法、AST、ブロック + インラインパーサー |
| `render` | AST -> tairitsu `VNode` -> HTML 文字列 |
| `theme` | hikari パレットから生成される CSS |
| `site` | docs ツリーを走査し、各ページをレンダリングし、静的サイトを書き出す |
| `cli` | `lagrange` バイナリ |

## 閉じたループ

Lagrange は自身のドキュメントをレンダリングします。`just docs` は
`lagrange build docs` を実行し、CI ワークフローがその結果をデプロイします。公開サイトで
これを読んでいるなら、それは Lagrange が生成したものです。

## 構築の基盤

- [tairitsu](https://github.com/celestia-island/tairitsu) —— レンダリング用 IR として使われる仮想 DOM。
- [hikari](https://github.com/celestia-island/hikari) —— テーマ化に使われるカラーパレット。
- [pest](https://pest.rs) —— パーサージェネレーター。
