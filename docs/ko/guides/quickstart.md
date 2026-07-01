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
├── zhs/
│   └── ...
```

`docs/`의 각 하위 디렉터리가 하나의 언어입니다. `SUMMARY.md`가 사이드바를 구성합니다.

## Markdown 지원

제목, 단락, 펜스 코드, 목록, 인용구, 표, 주제 구분선,
이미지, 그리고 인라인 `코드`, **굵게**, *기울임*, [링크](./architecture.md).

> 인용구도 지원됩니다.
