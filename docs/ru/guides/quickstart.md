# Быстрый старт

## Сборка сайта

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

Откройте `target/site/index.html` — он перенаправит на английскую книгу.

## Структура каталогов

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zhs/
│   └── ...
```

Каждый подкаталог `docs/` — это один язык. `SUMMARY.md` управляет боковой панелью.

## Поддержка Markdown

Заголовки, абзацы, код в ограждении, списки, цитаты, таблицы, тематические разделители,
изображения, а также строчный `код`, **жирный**, *курсив*, [ссылки](./architecture.md).

> Цитаты также поддерживаются.
