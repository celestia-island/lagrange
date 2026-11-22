# 架构

Lagrange 是单个 crate（`lagrange-library`），同时提供一个库和一个
`lagrange` 命令行工具。

## 流水线

docs 树经由**两条车道**产出：

```
markdown -> pest parser -> AST (Block / Inline)
              |                          |
              |                          +--> tairitsu VDom (VNode) -> HTML
              |                                （旧车道：手写壳）
              |
              +--> knowledge.json -----> docs-manifest.json
                    （chunk/span）             |
                                               v
                                    packages/webui（构建期 Node SSR）
                                    HkFaceShell + HkAdminHeader + HkSidebar
                                    -> 每语言一页 <lang>/<page>.html
                                    + hydration 包（mermaid 走
                                      hikari/tairitsu wasm 引擎）
```

**webui 车道**在构建能找到 webui 产物时运行——从仓根
（`packages/webui/dist-docs`），或任意位置经 `LAGRANGE_WEBUI_DIR` 指定；
`just docs` 与 CI 都先经 pnpm 构建产物：每个页面都由真正的 hikari 组件树
渲染——与家族各控制台同一套 `HkFaceShell` / `HkAdminHeader` 外壳，
站点 logo 占品牌槽、语言与主题按钮在右上角动作区、SUMMARY 侧栏带
轨道折叠、页脚单一状态带。页面使用按语言的 URL
（`<lang>/<path>.html`）；旧的扁平 URL 变为按语言协商的重定向。
**旧车道**保留为无 Node 环境的回退。

## 模块

| 模块 | 职责 |
|--------|----------------|
| `markdown` | pest 语法、AST、块级 + 行内解析器 |
| `render` | AST -> tairitsu `VNode` -> HTML 字符串 |
| `theme` | 由 hikari 调色板生成的 CSS |
| `site` | 遍历 docs 树，渲染每个页面，写出静态站点 |
| `webui_lane` | docs-manifest.json 生成与 Node SSR 渲染器拉起 |
| `cli` | `lagrange` 二进制文件 |

## 闭环

Lagrange 渲染自身的文档：`just docs` 运行
`lagrange build docs`，而 CI 工作流负责部署结果。如果你是在
已发布的站点上阅读本文，那么它正是由 Lagrange 生成的。

## 构建于

- [tairitsu](https://github.com/celestia-island/tairitsu) —— 用作渲染中间表示的虚拟 DOM。
- [hikari](https://github.com/celestia-island/hikari) —— 用于主题化的调色板。
- [pest](https://pest.rs) —— 解析器生成器。
