<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/lagrange/master/docs/logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>أداة إنشاء مواقع Markdown ثابتة بتصيير WASI — متعددة اللغات</strong></p>

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
[Русский](../ru/README.md) ·
**العربية**

</div>

## مقدمة

يحوّل Lagrange مجلدًا من ملفات Markdown إلى موقع توثيق ثابت متعدد اللغات —
يُصيّر عبر WASI، مع مبدّل لغة مدمج وإعادة توجيه من الجذر إلى الإنجليزية.
بلا أطر JavaScript، بلا mdBook، بلا سلسلة أدوات Node.

يعرض Lagrange **توثيق نفسه**: شجرة `docs/` المجاورة لهذا الملف التمهيدي
مبنية بواسطة Lagrange نفسه (`just docs`). إذا كنت تقرأ الموقع المنشور،
فقد أُنتج بواسطة Lagrange.

## بداية سريعة

قم ببناء الموقع الخاص بهذا المستودع:

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

افتح `target/site/index.html` — سيُعاد توجيهك إلى الكتاب الإنجليزي.

وجّه Lagrange إلى أي شجرة توثيق بنفس الشكل:

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

كل من `README.md` و `index.md` يُؤديان إلى `index.html`؛ ويُوصى بربط
`docs/en/README.md` رمزيًا إلى `README.md` الجذري للحفاظ على تزامن
صفحة GitHub الرئيسية مع فهرس التوثيق.

## نشر الموقع

يُنتج Lagrange دليلًا ثابتًا خالصًا، لذا يمكن نشره في أي مكان. عملية البناء
ببساطة هي `lagrange build --src docs --out <dir>` (أو `cargo run --release -- build …`
عند الاستخدام من المصدر).

### GitHub Actions → GitHub Pages

إجراء مركّب جاهز (_composite action_) يجلب Lagrange إلى جانب مستودعك ويُنفذ
البناء. هذا ما يستخدمه Lagrange نفسه
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

أمر البناء `cargo run --release -- build --src docs --out _site`، دليل الإخراج
`_site`. لـ Lagrange جاهز مسبقًا، قم بنسخ
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange)
وشغّل `cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`.
`--site_url` اختياري ويؤثر فقط على الروابط المطلقة في مبدّل اللغة.

### Vercel

الإعداد المسبق للإطار **Other**، أمر البناء
`cargo run --release -- build --src docs --out public`، دليل الإخراج
`public`. (يحتاج Vercel إلى سلسلة أدوات Rust — استخدم مشغّل
[`vercel-rust`](https://github.com/vercel-community/rust)، أو قم بالبناء
في خطوة CI سابقة وانشر المخرجات الثابتة.)

## الميزات

- **محلّل Markdown قائم على pest** — كتل + سطور، مستند إلى
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown)،
  بالإضافة إلى تمرير كتل HTML الخام لكي تنجو ترميزات توسيط README.
- **عرض VDom عبر tairitsu** — تُصبح المستندات أشجار `VNode` تُسلسل عبر
  `render_to_html`.
- **متعدد اللغات** — دليل واحد لكل لغة، ومبدّل لغة مدمج، وإعادة توجيه من
  الجذر إلى الإنجليزية.
- **الاستضافة الذاتية** — توثيق Lagrange نفسه مبني بواسطة Lagrange.

## الترخيص

SySL-1.0 (Synthetic Source License — رخصة المصدر التركيبي). راجع [LICENSE](https://sysl.celestia.world).
