# lagrange — 项目状态与计划 (PLAN)

> 本文件由自动化扫描于 **2026-07-04** 生成，记录项目当前状态、近期进展与后续计划。

## Refresh log 2026-07-14

- **当前分支**：`dev` · 领先 `origin/dev` 0 commits · 工作区干净
- **最近提交**：`⬆️ Upgrade GitHub Actions versions.` (`a405226`)
- **未提交改动**：无
- **后续动作**：
  1. CI 升级到新 GitHub Actions 版本后，回跑 WASI 渲染 / Markdown 多语言静态站点构建流水线，确认无回归。
  2. 关注跨仓 `[patch]` 收敛到 `~/.cargo/config.toml`（见 `entelecheia/PLAN.md` §6 跨仓依赖约定）后，WASI target 链接与缓存命中行为。
  3. 顶层 `patches/` 长期方案中，评估 lagrange 作为多语言静态站点设施对 entelecheia 文档站点的可复用性。
- **跨仓依赖**：作为 Markdown 多语言静态站点基础设施，被 entelecheia 文档站点消费；与 shittim-chest 等 sibling 仓共享多语言 i18n 资源。

## 1. 项目概述

- **名称**：`lagrange`
- **简介**：WASI 渲染的 Markdown 静态站点工具，多语言开箱即用。
- **远程仓库**：git@github.com:celestia-island/lagrange.git
- **技术栈**：Rust / just
- **类别**：rust-lib

## 2. 当前状态

- **当前分支**：`dev`
- **工作区**：干净
- **最近提交时间**：2026-07-04
- **最近提交**：docs: add docs.rs badge + metadata + refresh PLAN.md
- **分支对比**：`dev` 领先 `master` 39 个提交

## 3. 未提交改动

无。

## 4. 近期进展（最近提交）

- docs: use GitHub raw URL for logo, bold English without self-link
- style: enforce use statement grouping (3-group layout)
- fix: restore docs/en/README.md symlink + update malkuth intro bullet #2
- Merge master — 🚀 Initial commit.
- 🚀 Initial commit.
- docs: update description to WASI-rendered, simplify Introduction

## 5. 后续计划

1. ~~完善文档示例与 `crates.io` 发布元数据（rust-version / metadata / docs.rs badge）。~~ ✅
2. 补充单元/集成测试，保持 `just test` 与 clippy `-D warnings` 通过。
3. 定期刷新本 PLAN.md 以反映最新状态。

### 已完成
- `rust-version = "1.80"` ✅（已有）
- `keywords` / `categories` / `description` / `repository` ✅（已有）
- `docs.rs` badge + `[package.metadata.docs.rs]` all-features ✅（2026-07-04）

