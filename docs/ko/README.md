<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/lagrange/master/docs/logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>WASI 렌더링 Markdown 정적 사이트 도구 — 다국어 지원</strong></p>

<div align="center">

[![License: SySL-1.0](https://img.shields.io/badge/License-SySL--1.0-blue.svg)](https://sysl.celestia.world)
[![Checks](https://img.shields.io/github/actions/workflow/status/celestia-island/lagrange/checks.yml)](https://github.com/celestia-island/lagrange/actions/workflows/checks.yml)
[![Docs](https://img.shields.io/badge/docs-lagrange.celestia.world-blue)](https://lagrange.celestia.world)

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

Lagrange는 Markdown 폴더를 정적 다국어 문서 사이트로 변환합니다 — WASI로 렌더링되며, 내장 언어 전환기와 루트에서 영어로의 리다이렉트를 제공합니다. JavaScript 프레임워크도, mdBook도, Node 도구 체인도 필요 없습니다.

Lagrange는 **자신의 문서를 직접 렌더링**합니다: 이 README 옆에 있는 `docs/`
트리는 Lagrange 자체로 빌드됩니다(`just docs`). 게시된 사이트를 보고 계신다면,
그것은 Lagrange에 의해 생성된 것입니다.

## 빠른 시작

이 저장소의 사이트를 빌드합니다:

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

`target/site/index.html`을 열면 — 영어 문서로 리다이렉트됩니다.

같은 구조를 가진 어떤 docs 트리에도 Lagrange를 사용할 수 있습니다:

```
docs/
├── logo.webp          # shared assets live at the docs root
├── en/
│   ├── README.md      # becomes <site>/en/index.html
│   ├── SUMMARY.md     # drives the sidebar
│   └── guides/*.md
└── zhs/ …             # one directory per language
```

```bash
lagrange build --src docs --out _site
```

`README.md`와 `index.md`는 모두 `index.html`로 매핑됩니다. 루트 `README.md`를
`docs/en/README.md`로 심링크하는 것이 GitHub 랜딩 페이지와 문서 인덱스를
동기화하는 권장 방법입니다.

## 사이트 배포

Lagrange는 순수한 정적 디렉터리를 생성하므로 어디에나 배포할 수 있습니다.
빌드는 단순히 `lagrange build --src docs --out <dir>` (또는 소스에서 사용할 때는
`cargo run --release -- build …`)입니다.

### GitHub Actions → GitHub Pages

미리 만들어진 composite action이 저장소와 함께 Lagrange를 체크아웃하고 빌드를
실행합니다. 이것이 Lagrange 자체에서 사용하는 방식입니다
([`.github/workflows/docs.yml`](../../.github/workflows/docs.yml)):

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: celestia-island/lagrange/.github/actions/build@dev
        with:
          src: docs
          out: _site
      - run: echo "your-project.docs.celestia.world" > _site/CNAME
      - uses: actions/upload-pages-artifact@v3
        with:
          path: _site
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - uses: actions/deploy-pages@v4
```

### Cloudflare Pages

빌드 명령어 `cargo run --release -- build --src docs --out _site`, 빌드 출력
디렉터리 `_site`. 미리 빌드된 Lagrange를 사용하려면
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange)를
미러링하고
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`를
실행하세요. `--site_url`은 선택 사항이며 언어 전환기의 절대 링크에만 영향을 줍니다.

### Vercel

프레임워크 프리셋 **Other**, 빌드 명령어
`cargo run --release -- build --src docs --out public`, 출력 디렉터리
`public`. (Vercel은 Rust 도구 체인이 필요합니다 —
[`vercel-rust`](https://github.com/vercel-community/rust) 런타임 빌더를
사용하거나, 사전 CI 단계에서 빌드하고 정적 출력을 배포하세요.)

## 특징

- **pest 기반 Markdown 파서** — 블록 + 인라인,
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown)을
  모델로 하였으며, raw-HTML-block 통과 기능으로 중앙 정렬된 README 마크업이
  그대로 유지됩니다.
- **tairitsu VDom 렌더링** — 문서가 `VNode` 트리로 변환되어
  `render_to_html`을 통해 직렬화됩니다.
- **다국어** — 언어별로 하나의 디렉터리, 내장 언어 전환기,
  루트에서 영어로의 리다이렉트.
- **자체 호스팅** — Lagrange의 자체 문서는 Lagrange로 빌드됩니다.

## 라이선스

SySL-1.0 (Synthetic Source License). [LICENSE](https://sysl.celestia.world)를 참고하세요.
