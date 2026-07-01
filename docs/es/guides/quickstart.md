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
├── zhs/
│   └── ...
```

Cada subdirectorio de `docs/` es un idioma. `SUMMARY.md` controla la barra lateral.

## Soporte de Markdown

Encabezados, párrafos, código cercado, listas, citas, tablas, separadores temáticos,
imágenes, y `código` en línea, **negrita**, *cursiva*, [enlaces](./architecture.md).

> Las citas también son compatibles.
