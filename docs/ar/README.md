# Lagrange

**منشأة عرض لوثائق markdown مبنية على pest.**

يحلل Lagrange لغة markdown بقواعد pest، ويعرضها إلى HTML عبر
DOM الافتراضي [tairitsu](https://github.com/celestia-island/tairitsu)،
ويجمع موقعًا ثابتًا متعدد اللغات — ذا سمة مأخوذة من لوحة
[hikari](https://github.com/celestia-island/hikari).

> هذه الصفحة ذاتها يعرضها Lagrange بنفسه.

## الميزات

- **محلل markdown مبني على pest** —— كتل + سطري، على غرار ratatui-markdown.
- **عرض VDom عبر tairitsu** —— تتحول المستندات إلى أشجار `VNode`، تُسلسَل عبر
  `render_to_html`.
- **متعدد اللغات** —— مجلد واحد لكل لغة، ومبدّل لغات مدمج،
  وإعادة توجيه جذرية إلى الإنجليزية.
- **استضافة ذاتية** —— توثيق Lagrange نفسه يُبنى بواسطة Lagrange
  (`just docs`).

## البداية السريعة

```bash
cargo run --release -- build docs --out target/site
```

انظر [البداية السريعة](./guides/quickstart.md) و[البنية](./guides/architecture.md).
