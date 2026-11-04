# クイックスタート

## サイトをビルドする

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

`target/site/index.html` を開くと —— 英語版のブックへリダイレクトされます。

## ディレクトリ構成

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zh-Hans/
│   └── ...
```

`docs/` の各サブディレクトリが 1 つの言語です。`SUMMARY.md` がサイドバーを駆動します。

## Markdown サポート

見出し、段落、フェンスコード、リスト、引用、表、区切り線、
画像、そしてインライン `コード`、**太字**、*イタリック*、[リンク](./architecture.md)。

> 引用にも対応しています。

## ライト／ダーク別の画像

ライトとダークで別の画像を出したいスクリーンショットは、エコシステム共通の
URL フラグメント記法に従います — GitHub の `#gh-light-mode-only` /
`#gh-dark-mode-only`、または mkdocs-material の別名 `#only-light` /
`#only-dark`：

```markdown
![ライトモードのダッシュボード](dashboard.webp#gh-light-mode-only)
![ダークモードのダッシュボード](dashboard.dark.webp#gh-dark-mode-only)
```

フラグメントは出力される `src` から取り除かれ、ページは読者の
`prefers-color-scheme` に合わない側を非表示にします。フラグメントのない画像は
両モードに属するため、通常のスクリーンショットは既定の（ライト用）ファイルだけで
足ります。同じ markdown は GitHub（`gh-*` 記法を解釈）でも正しく描画されます。
