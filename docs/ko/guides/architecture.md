# 아키텍처

Lagrange는 단일 crate(`lagrange-library`)로, 라이브러리와
`lagrange` CLI를 함께 제공합니다.

## 파이프라인

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## 모듈

| 모듈 | 역할 |
|--------|----------------|
| `markdown` | pest 문법, AST, 블록 + 인라인 파서 |
| `render` | AST -> tairitsu `VNode` -> HTML 문자열 |
| `theme` | hikari 팔레트로부터 생성된 CSS |
| `site` | docs 트리를 순회하며 각 페이지를 렌더링하고 정적 사이트를 작성 |
| `cli` | `lagrange` 바이너리 |

## 닫힌 순환

Lagrange는 자체 문서를 렌더링합니다. `just docs`는
`lagrange build docs`을 실행하고, CI 워크플로가 그 결과를 배포합니다. 게시된 사이트에서
이 글을 읽고 있다면, 그것은 Lagrange가 만든 것입니다.

## 구성 기반

- [tairitsu](https://github.com/celestia-island/tairitsu) —— 렌더링 IR로 사용되는 가상 DOM.
- [hikari](https://github.com/celestia-island/hikari) —— 테마에 사용되는 색상 팔레트.
- [pest](https://pest.rs) —— 파서 생성기.
