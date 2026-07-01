# 架構

Lagrange 是單一 crate（`lagrange-library`），同時提供一個函式庫與一個
`lagrange` 命令列工具。

## 管線

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## 模組

| 模組 | 職責 |
|--------|----------------|
| `markdown` | pest 語法、AST、區塊 + 行內解析器 |
| `render` | AST -> tairitsu `VNode` -> HTML 字串 |
| `theme` | 由 hikari 調色盤產生的 CSS |
| `site` | 走訪 docs 樹、渲染每個頁面、寫出靜態網站 |
| `cli` | `lagrange` 執行檔 |

## 閉環

Lagrange 渲染自身的文件：`just docs` 執行
`lagrange build docs`，而 CI 工作流程負責部署結果。若你是在
已發布的網站上閱讀本文，那麼它正是由 Lagrange 產生的。

## 建構於

- [tairitsu](https://github.com/celestia-island/tairitsu) —— 作為渲染中間表示的虛擬 DOM。
- [hikari](https://github.com/celestia-island/hikari) —— 用於主題化的調色盤。
- [pest](https://pest.rs) —— 解析器產生器。
