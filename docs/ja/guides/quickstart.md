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
├── zhs/
│   └── ...
```

`docs/` の各サブディレクトリが 1 つの言語です。`SUMMARY.md` がサイドバーを駆動します。

## Markdown サポート

見出し、段落、フェンスコード、リスト、引用、表、区切り線、
画像、そしてインライン `コード`、**太字**、*イタリック*、[リンク](./architecture.md)。

> 引用にも対応しています。
