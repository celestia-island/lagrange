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
├── zhs/
│   └── ...
```

كل مجلد فرعي ضمن `docs/` يمثل لغة واحدة. ويقود `SUMMARY.md` الشريط الجانبي.

## دعم Markdown

العناوين، والفقرات، والكود المسوّر، والقوائم، والاقتباسات، والجداول، والفواصل الموضوعية،
والصور، وكذلك `الكود` السطري، و**العريض**، و*المائل*، و[الروابط](./architecture.md).

> الاقتباسات مدعومة أيضًا.
