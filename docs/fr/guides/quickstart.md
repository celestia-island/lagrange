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
├── zh-Hans/
│   └── ...
```

Chaque sous-répertoire de `docs/` correspond à une langue. `SUMMARY.md` pilote la barre latérale.

## Prise en charge de Markdown

Titres, paragraphes, code clôturé, listes, citations, tableaux, séparateurs thématiques,
images, et `code` en ligne, **gras**, *italique*, [liens](./architecture.md).

> Les citations sont également prises en charge.

## Images selon le thème clair/sombre

Les captures qui doivent différer entre une page claire et une page sombre
utilisent la convention de fragment d'URL de l'écosystème — `#gh-light-mode-only`
/ `#gh-dark-mode-only` de GitHub, ou les alias mkdocs-material `#only-light` /
`#only-dark` :

```markdown
![Tableau de bord en mode clair](dashboard.webp#gh-light-mode-only)
![Tableau de bord en mode sombre](dashboard.dark.webp#gh-dark-mode-only)
```

Le fragment est retiré du `src` produit, et la page masque la variante qui ne
correspond pas au `prefers-color-scheme` du lecteur. Une image sans fragment
appartient aux deux schémas : une capture ordinaire n'a donc besoin que du fichier
par défaut (clair). Le même markdown s'affiche correctement sur GitHub, qui
comprend la notation `gh-*`.
