# Lagrange

**一个基于 pest 的 Markdown 文档渲染设施。**

Lagrange 使用 pest 语法解析 Markdown，通过
[tairitsu](https://github.com/celestia-island/tairitsu) 虚拟 DOM 将其渲染为 HTML，并组装成一个多语言静态站点——使用
[hikari](https://github.com/celestia-island/hikari) 调色板进行主题化。

> 你正在浏览的这个页面正是由 Lagrange 自身渲染的。

## 特性

- **基于 pest 的 Markdown 解析器** —— 块级 + 行内，以 ratatui-markdown 为蓝本。
- **tairitsu VDom 渲染** —— 文档转换为 `VNode` 树，通过
  `render_to_html` 序列化。
- **多语言** —— 每种语言一个目录，内置语言切换器，
  以及到英文版的根重定向。
- **自托管** —— Lagrange 自身的文档由 Lagrange 构建
  （`just docs`）。

## 快速开始

```bash
cargo run --release -- build docs --out target/site
```

参见[快速开始](./guides/quickstart.md)和[架构](./guides/architecture.md)。
