# 快速入門

## 建置網站

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

開啟 `target/site/index.html`——它會重新導向到英文版手冊。

## 目錄配置

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zhs/
│   └── ...
```

`docs/` 的每個子目錄代表一種語言。`SUMMARY.md` 驅動側邊欄。

## Markdown 支援

標題、段落、圍欄程式碼、清單、引述區塊、表格、分隔線、
圖片，以及行內 `程式碼`、**粗體**、*斜體*、[連結](./architecture.md)。

> 引述區塊同樣受到支援。
