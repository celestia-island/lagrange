<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>منشأة عرض وثائق Markdown مبنية على pest — VDom tairitsu + لوحة ألوان hikari، متعددة اللغات جاهزة للاستخدام.</strong></p>

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
[한국어](../ko/README.md) ·
[Français](../fr/README.md) ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
**العربية**

</div>

## مقدمة

يحوّل Lagrange مجلد Markdown إلى موقع وثائق ثابت متعدد اللغات. يعالج Markdown
بقواعد [pest](https://pest.rs) المكتوبة يدويًا، ويعرض شجرة AST إلى HTML عبر
DOM الافتراضي [tairitsu](https://github.com/celestia-island/tairitsu)،
ويُطبّق السمات من لوحة ألوان [hikari](https://github.com/celestia-island/hikari).
دليل واحد لكل لغة، ومبدّل لغة مدمج، وإعادة توجيه من الجذر إلى الإنكليزية —
بلا أطر JavaScript، بلا mdBook، بلا سلسلة أدوات Node.

## بداية سريعة

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

## النشر

الدليل الناتج ثابت — يُمكن نشره على **GitHub Pages** (باستخدام الإجراء المُركّب
المدمج)، أو **Cloudflare Pages**، أو **Vercel**.

## الميزات

- **محلّل Markdown قائم على pest** (كتل+سطور، مع تمرير HTML الخام)
- **عرض عبر VDom tairitsu**
- **متعدد اللغات** — دليل لكل لغة، مبدّل لغة مدمج
- **الاستضافة الذاتية** — وثائق Lagrange مبنية بـ Lagrange نفسه

## الترخيص

SySL-1.0 (Synthetic Source License). راجع [LICENSE](../../LICENSE).
