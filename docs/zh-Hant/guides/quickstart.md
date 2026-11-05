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
├── zh-Hans/
│   └── ...
```

`docs/` 的每個子目錄代表一種語言。`SUMMARY.md` 驅動側邊欄。

## Markdown 支援

標題、段落、圍欄程式碼、清單、引述區塊、表格、分隔線、
圖片，以及行內 `程式碼`、**粗體**、*斜體*、[連結](./architecture.md)。

> 引述區塊同樣受到支援。

## 亮暗主題配圖

需要在亮色與暗色頁面呈現不同圖片的截圖，沿用生態中的 URL 片段慣例 ——
GitHub 的 `#gh-light-mode-only` / `#gh-dark-mode-only`，或 mkdocs-material
的別名 `#only-light` / `#only-dark`：

```markdown
![亮色模式下的儀表板](dashboard.webp#gh-light-mode-only)
![暗色模式下的儀表板](dashboard.dark.webp#gh-dark-mode-only)
```

片段會從輸出的 `src` 剝離，頁面依讀者的 `prefers-color-scheme` 隱藏不符合
的那一張；不帶片段的圖片在兩種模式下都會顯示，因此一般截圖只需預設的（亮色）
檔案。同一份 markdown 在 GitHub 上也能正確算繪（它認得 `gh-*` 寫法）。
