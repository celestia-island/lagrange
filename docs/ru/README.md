<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/lagrange/master/docs/logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>Генератор статических сайтов на Markdown с WASI-рендерингом — мультиязычный</strong></p>

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
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
**Русский** ·
[العربية](../ar/README.md)

</div>

## Введение

Lagrange превращает папку с Markdown-файлами в статический, многоязычный
документационный сайт — с WASI-рендерингом, встроенным переключателем языков
и корневым редиректом на английский. Никаких JavaScript-фреймворков, никакого
mdBook, никакого инструментария Node.

Lagrange рендерит **собственную документацию**: дерево `docs/` рядом с этим
README собирается самим Lagrange (`just docs`). Если вы читаете
опубликованный сайт, он был создан с помощью Lagrange.

## Быстрый старт

Сборка сайта для этого самого репозитория:

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

Откройте `target/site/index.html` — он перенаправляет в английскую книгу.

Направьте Lagrange на любое дерево документации с той же структурой:

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

`README.md` и `index.md` оба отображаются в `index.html`; символьная ссылка
`docs/en/README.md` на корневой `README.md` — рекомендуемый способ
синхронизации посадочной страницы GitHub и индекса документации.

## Развёртывание сайта

Lagrange создаёт обычный статический каталог, поэтому развёртывается где
угодно. Сборка выполняется командой `lagrange build --src docs --out <dir>`
(или `cargo run --release -- build …` при работе из исходного кода).

### GitHub Actions → GitHub Pages

Готовый composite action извлекает Lagrange рядом с вашим репозиторием и
запускает сборку. Именно так использует его сам Lagrange
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

Команда сборки `cargo run --release -- build --src docs --out _site`, выходной
каталог `_site`. Для предварительно собранного Lagrange — зеркалируйте
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange) и
запустите
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`.
`--site_url` необязателен и влияет только на абсолютные ссылки в
переключателе языков.

### Vercel

Предустановка фреймворка **Other**, команда сборки
`cargo run --release -- build --src docs --out public`, выходной каталог
`public`. (Vercel требуется инструментарий Rust — используйте
[`vercel-rust`](https://github.com/vercel-community/rust) runtime builder,
либо соберите в предыдущем шаге CI и разверните статический вывод.)

## Возможности

- **Парсер Markdown на основе pest** — блочный + строчный, по образцу
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown),
  плюс сквозной пропуск сырых HTML-блоков, чтобы центрированная разметка
  README сохранялась.
- **Рендеринг VDom tairitsu** — документы становятся деревьями `VNode`,
  сериализуются через `render_to_html`.
- **Многоязычность** — один каталог на язык, встроенный переключатель
  языков и корневой редирект на английский.
- **Самодокументирование** — документация Lagrange собрана самим Lagrange.

## Лицензия

SySL-1.0 (Synthetic Source License). См. [LICENSE](https://sysl.celestia.world).
