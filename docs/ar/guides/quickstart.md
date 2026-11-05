# البداية السريعة

## بناء موقع

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

افتح `target/site/index.html` — وهو يعيد التوجيه إلى الكتاب الإنجليزي.

## تخطيط المجلدات

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zh-Hans/
│   └── ...
```

كل مجلد فرعي ضمن `docs/` يمثل لغة واحدة. ويقود `SUMMARY.md` الشريط الجانبي.

## دعم Markdown

العناوين، والفقرات، والكود المسوّر، والقوائم، والاقتباسات، والجداول، والفواصل الموضوعية،
والصور، وكذلك `الكود` السطري، و**العريض**، و*المائل*، و[الروابط](./architecture.md).

> الاقتباسات مدعومة أيضًا.

## صور حسب السمة الفاتحة/الداكنة

لقطات الشاشة التي يجب أن تختلف بين الصفحات الفاتحة والداكنة تستخدم اصطلاح جزء
عنوان URL المعتمد في المنظومة — `#gh-light-mode-only` /
`#gh-dark-mode-only` من GitHub، أو الأسماء المستعارة في mkdocs-material
`#only-light` / `#only-dark`:

```markdown
![لوحة المعلومات في الوضع الفاتح](dashboard.webp#gh-light-mode-only)
![لوحة المعلومات في الوضع الداكن](dashboard.dark.webp#gh-dark-mode-only)
```

يزال الجزء من `src` الناتج، وتخفي الصفحة النسخة التي لا تطابق
`prefers-color-scheme` لدى القارئ. الصورة بدون جزء تنتمي إلى السمتين، لذا تكفي
اللقطة العادية بملف واحد افتراضي (فاتح). ويُعرض نفس markdown بشكل صحيح على GitHub
الذي يفهم كتابة `gh-*`.
