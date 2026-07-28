# 快速开始

## 构建站点

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

打开 `target/site/index.html`——它会重定向到英文版手册。

## 目录布局

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zhs/
│   └── ...
```

`docs/` 的每个子目录代表一种语言。`SUMMARY.md` 驱动侧边栏。

## Markdown 支持

标题、段落、围栏代码、列表、引用块、表格、分隔线、
图片，以及行内 `代码`、**粗体**、*斜体*、[链接](./architecture.md)。

> 引用块同样受支持。
