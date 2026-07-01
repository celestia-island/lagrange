# Lagrange

**一個基於 pest 的 Markdown 文件渲染設施。**

Lagrange 使用 pest 語法解析 Markdown，透過
[tairitsu](https://github.com/celestia-island/tairitsu) 虛擬 DOM 將其渲染為 HTML，並組裝成一個多語言靜態網站——使用
[hikari](https://github.com/celestia-island/hikari) 調色盤進行主題化。

> 你正在瀏覽的這個頁面正是由 Lagrange 自身渲染的。

## 特色

- **基於 pest 的 Markdown 解析器** —— 區塊 + 行內，以 ratatui-markdown 為藍本。
- **tairitsu VDom 渲染** —— 文件轉換為 `VNode` 樹，透過
  `render_to_html` 序列化。
- **多語言** —— 每種語言一個目錄，內建語言切換器，
  以及到英文版的根重新導向。
- **自架** —— Lagrange 自身的文件由 Lagrange 建置
  （`just docs`）。

## 快速入門

```bash
cargo run --release -- build docs --out target/site
```

請參見[快速入門](./guides/quickstart.md)與[架構](./guides/architecture.md)。
