# 架构

Lagrange 是单个 crate（`lagrange-library`），同时提供一个库和一个
`lagrange` 命令行工具。

## 流水线

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## 模块

| 模块 | 职责 |
|--------|----------------|
| `markdown` | pest 语法、AST、块级 + 行内解析器 |
| `render` | AST -> tairitsu `VNode` -> HTML 字符串 |
| `theme` | 由 hikari 调色板生成的 CSS |
| `site` | 遍历 docs 树，渲染每个页面，写出静态站点 |
| `cli` | `lagrange` 二进制文件 |

## 闭环

Lagrange 渲染自身的文档：`just docs` 运行
`lagrange build docs`，而 CI 工作流负责部署结果。如果你是在
已发布的站点上阅读本文，那么它正是由 Lagrange 生成的。

## 构建于

- [tairitsu](https://github.com/celestia-island/tairitsu) —— 用作渲染中间表示的虚拟 DOM。
- [hikari](https://github.com/celestia-island/hikari) —— 用于主题化的调色板。
- [pest](https://pest.rs) —— 解析器生成器。
