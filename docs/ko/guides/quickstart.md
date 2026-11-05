# 빠른 시작

## 사이트 빌드하기

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

`target/site/index.html`을 열면 —— 영어 책으로 리다이렉트됩니다.

## 디렉터리 구성

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zh-Hans/
│   └── ...
```

`docs/`의 각 하위 디렉터리가 하나의 언어입니다. `SUMMARY.md`가 사이드바를 구성합니다.

## Markdown 지원

제목, 단락, 펜스 코드, 목록, 인용구, 표, 주제 구분선,
이미지, 그리고 인라인 `코드`, **굵게**, *기울임*, [링크](./architecture.md).

> 인용구도 지원됩니다.

## 라이트/다크별 이미지

라이트 모드와 다크 모드에서 서로 다른 이미지를 보여야 하는 스크린샷은 생태계의
URL 프래그먼트 규칙을 따릅니다 — GitHub의 `#gh-light-mode-only` /
`#gh-dark-mode-only`, 또는 mkdocs-material 별칭 `#only-light` / `#only-dark`:

```markdown
![라이트 모드 대시보드](dashboard.webp#gh-light-mode-only)
![다크 모드 대시보드](dashboard.dark.webp#gh-dark-mode-only)
```

프래그먼트는 출력되는 `src`에서 제거되고, 페이지는 독자의
`prefers-color-scheme`와 맞지 않는 쪽을 숨깁니다. 프래그먼트가 없는 이미지는 두
모드 모두에 속하므로 일반 스크린샷은 기본(라이트) 파일 하나면 충분합니다. 같은
markdown은 GitHub(`gh-*` 표기 지원)에서도 올바르게 렌더링됩니다.
