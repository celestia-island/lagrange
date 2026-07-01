# Lagrange

**pest 기반 Markdown 문서 렌더링 도구입니다.**

Lagrange는 pest 문법으로 Markdown을 파싱하고,
[tairitsu](https://github.com/celestia-island/tairitsu) 가상 DOM을 통해 HTML로 렌더링하며,
[hikari](https://github.com/celestia-island/hikari) 팔레트로 테마를 적용한
다국어 정적 사이트를 조립합니다.

> 지금 보고 계신 이 페이지 역시 Lagrange가 직접 렌더링한 것입니다.

## 기능

- **pest 기반 Markdown 파서** —— 블록 + 인라인, ratatui-markdown을 모델로 삼습니다.
- **tairitsu VDom 렌더링** —— 문서는 `VNode` 트리가 되며,
  `render_to_html`로 직렬화됩니다.
- **다국어** —— 언어마다 하나의 디렉터리, 내장 언어 전환기,
  그리고 영어로의 루트 리다이렉트.
- **자체 호스팅** —— Lagrange 자체의 문서도 Lagrange가
  (`just docs`로) 빌드합니다.

## 빠른 시작

```bash
cargo run --release -- build docs --out target/site
```

[빠른 시작](./guides/quickstart.md)과 [아키텍처](./guides/architecture.md)를 참고하세요.
