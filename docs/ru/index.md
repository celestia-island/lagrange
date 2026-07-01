# Lagrange

**Инструмент рендеринга markdown-документации на основе pest.**

Lagrange разбирает markdown с помощью грамматики pest, рендерит его в HTML через
виртуальный DOM [tairitsu](https://github.com/celestia-island/tairitsu) и
собирает многоязычный статический сайт — с темой на основе палитры
[hikari](https://github.com/celestia-island/hikari).

> Эта самая страница отрендерена самим Lagrange.

## Возможности

- **markdown-парсер на основе pest** — блочный + строчный, по образцу ratatui-markdown.
- **Рендеринг VDom tairitsu** — документы превращаются в деревья `VNode`, сериализуемые через
  `render_to_html`.
- **Многоязычность** — один каталог на язык, встроенный переключатель языков
  и корневой редирект на английский.
- **Самохостинг** — собственная документация Lagrange собирается самим Lagrange
  (`just docs`).

## Быстрый старт

```bash
cargo run --release -- build docs --out target/site
```

См. [Быстрый старт](./guides/quickstart.md) и [Архитектура](./guides/architecture.md).
