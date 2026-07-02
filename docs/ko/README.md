<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>pest 기반 Markdown 문서 렌더러 — tairitsu VDom + hikari 팔레트, 다국어 기본 제공.</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](../../LICENSE)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-lagrange.docs.celestia.world-blue)](https://lagrange.docs.celestia.world)

</div>

<div align="center">

[English](../en/README.md) ·
[简体中文](../zhs/README.md) ·
[繁體中文](../zht/README.md) ·
[日本語](../ja/README.md) ·
**한국어** ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## 소개

Lagrange는 Markdown 폴더를 다국어 정적 문서 사이트로 변환합니다. 자체 작성한
[pest](https://pest.rs) 문법으로 Markdown을 파싱하고, [tairitsu](https://github.com/celestia-island/tairitsu)
가상 DOM을 통해 HTML로 렌더링하며, [hikari](https://github.com/celestia-island/hikari)
팔레트로 테마를 적용합니다. 언어별 1개 디렉터리, 내장 언어 스위처, 루트에서 영어로
리다이렉트 — JavaScript 프레임워크, mdBook, Node 도구 체인 불필요.

## 빠른 시작

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

## 배포

**GitHub Pages** (내장 composite action), **Cloudflare Pages**, **Vercel**에
그대로 배포할 수 있습니다.

## 특징

- **pest 기반 파서** (블록+인라인, raw HTML 통과)
- **tairitsu VDom 렌더링**
- **다국어** (언어별 디렉터리, 내장 스위처)
- **자체 호스팅** (Lagrange 문서 자체를 Lagrange로 빌드)

## 라이선스

SySL-1.0. [LICENSE](../../LICENSE) 참고.
