# Démarrage rapide

## Construire un site

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build docs --out target/site
```

Ouvrez `target/site/index.html` — il redirige vers le livre anglais.

## Disposition des répertoires

```
docs/
├── en/
│   ├── SUMMARY.md
│   ├── index.md
│   └── guides/quickstart.md
├── zhs/
│   └── ...
```

Chaque sous-répertoire de `docs/` correspond à une langue. `SUMMARY.md` pilote la barre latérale.

## Prise en charge de Markdown

Titres, paragraphes, code clôturé, listes, citations, tableaux, séparateurs thématiques,
images, et `code` en ligne, **gras**, *italique*, [liens](./architecture.md).

> Les citations sont également prises en charge.
