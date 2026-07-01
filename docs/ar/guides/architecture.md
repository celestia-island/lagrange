# البنية

Lagrange هو crate واحد (`lagrange-library`) يوفّر مكتبة وأداة
CLI باسم `lagrange`.

## خط الأنابيب

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## الوحدات

| الوحدة | المسؤولية |
|--------|----------------|
| `markdown` | قواعد pest، وAST، ومحلل الكتل + السطري |
| `render` | AST -> tairitsu `VNode` -> سلسلة HTML |
| `theme` | CSS مُولّد من لوحة hikari |
| `site` | يجتاز شجرة docs، ويعرض كل صفحة، ويكتب موقعًا ثابتًا |
| `cli` | الملف التنفيذي `lagrange` |

## الحلقة المغلقة

يعرض Lagrange توثيقه الخاص: إذ ينفّذ `just docs` أمر
`lagrange build docs`، ثم ينشر سير عمل CI النتيجة. إن كنت تقرأ
هذا على الموقع المنشور، فقد أنتجه Lagrange.

## مبني على

- [tairitsu](https://github.com/celestia-island/tairitsu) —— DOM الافتراضي
  المُستخدَم بوصفه تمثيلًا وسيطًا للعرض (IR).
- [hikari](https://github.com/celestia-island/hikari) —— لوحة الألوان المُستخدَمة
  في وضع السمات.
- [pest](https://pest.rs) —— مولّد المحللات.
