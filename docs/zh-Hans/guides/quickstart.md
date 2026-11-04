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
├── zh-Hans/
│   └── ...
```

`docs/` 的每个子目录代表一种语言。`SUMMARY.md` 驱动侧边栏。

## Markdown 支持

标题、段落、围栏代码、列表、引用块、表格、分隔线、
图片，以及行内 `代码`、**粗体**、*斜体*、[链接](./architecture.md)。

> 引用块同样受支持。

## 亮暗主题配图

需要在亮色与暗色页面展示不同图片的截图，沿用生态里的 URL 片段约定 ——
GitHub 的 `#gh-light-mode-only` / `#gh-dark-mode-only`，或 mkdocs-material
的别名 `#only-light` / `#only-dark`：

```markdown
![亮色模式下的仪表盘](dashboard.webp#gh-light-mode-only)
![暗色模式下的仪表盘](dashboard.dark.webp#gh-dark-mode-only)
```

该片段会从输出的 `src` 中剥离，页面按读者的 `prefers-color-scheme` 隐藏
不匹配的那一张；不带片段的图片在两种模式下都会显示，所以普通截图只需默认的
（亮色）文件。同一份 markdown 在 GitHub 上同样能正确渲染（它认识 `gh-*`
写法）。
