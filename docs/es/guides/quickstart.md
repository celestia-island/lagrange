# Inicio rápido

## Construir un sitio

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

Abre `target/site/index.html` — redirige al libro en inglés.

## Disposición de directorios

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zh-Hans/
│   └── ...
```

Cada subdirectorio de `docs/` es un idioma. `SUMMARY.md` controla la barra lateral.

## Soporte de Markdown

Encabezados, párrafos, código cercado, listas, citas, tablas, separadores temáticos,
imágenes, y `código` en línea, **negrita**, *cursiva*, [enlaces](./architecture.md).

> Las citas también son compatibles.

## Imágenes según el tema claro/oscuro

Las capturas que deben diferir entre una página clara y una oscura usan la
convención de fragmento de URL del ecosistema — `#gh-light-mode-only` /
`#gh-dark-mode-only` de GitHub, o los alias de mkdocs-material `#only-light` /
`#only-dark`:

```markdown
![Panel en modo claro](dashboard.webp#gh-light-mode-only)
![Panel en modo oscuro](dashboard.dark.webp#gh-dark-mode-only)
```

El fragmento se elimina del `src` generado y la página oculta la variante que no
coincide con el `prefers-color-scheme` del lector. Una imagen sin fragmento
pertenece a ambos esquemas, así que una captura normal solo necesita el archivo
predeterminado (claro). El mismo markdown se muestra correctamente en GitHub, que
entiende la notación `gh-*`.
