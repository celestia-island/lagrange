<p align="center"><img src="https://raw.githubusercontent.com/celestia-island/lagrange/master/docs/logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>Un générateur de site statique Markdown via WASI — multilingue prêt à l'emploi</strong></p>

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
**Français** ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## Introduction

Lagrange transforme un dossier de fichiers Markdown en un site de documentation
statique multilingue — rendu WASI, avec un sélecteur de langue intégré et une
redirection racine vers l'anglais. Aucun framework JavaScript, pas de mdBook,
pas de chaîne d'outils Node.

Lagrange génère **sa propre documentation** : l'arborescence `docs/` à côté de
ce README est construite par Lagrange lui-même (`just docs`). Si vous lisez le
site publié, il a été produit par Lagrange.

## Démarrage rapide

Construire le site pour ce dépôt même :

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

Ouvrez `target/site/index.html` — il redirige vers le livre en anglais.

Pointez Lagrange vers n'importe quelle arborescence de documentation ayant la
même structure :

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

`README.md` et `index.md` correspondent tous deux à `index.html` ; un lien
symbolique `docs/en/README.md` pointant vers le `README.md` racine est la
méthode recommandée pour garder la page d'accueil GitHub et l'index de la
documentation synchronisés.

## Déploiement du site

Lagrange produit un simple répertoire statique, il se déploie donc partout. La
compilation se résume à `lagrange build --src docs --out <dir>` (ou
`cargo run --release -- build …` lorsqu'il est consommé depuis les sources).

### GitHub Actions → GitHub Pages

Une action composite prête à l'emploi récupère Lagrange en même temps que votre
dépôt et exécute la compilation. C'est ce que Lagrange utilise lui-même
([`.github/workflows/docs.yml`](../../.github/workflows/docs.yml)) :

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

Commande de build `cargo run --release -- build --src docs --out _site`,
répertoire de sortie `_site`. Pour un Lagrange précompilé, faites un miroir de
[`celestia-island/lagrange`](https://github.com/celestia-island/lagrange) et
exécutez
`cargo run --release -- build --src $DOCS_DIR --site_url $CF_PAGES_URL --out _site`.
`--site_url` est optionnel et n'affecte que les liens absolus dans le sélecteur
de langue.

### Vercel

Préréglage de framework **Other**, commande de build
`cargo run --release -- build --src docs --out public`, répertoire de sortie
`public`. (Vercel nécessite la chaîne d'outils Rust — utilisez le runtime
builder [`vercel-rust`](https://github.com/vercel-community/rust), ou compilez
dans une étape CI préalable et déployez la sortie statique.)

## Fonctionnalités

- **Parseur Markdown basé sur pest** — blocs + inline, modélisé d'après
  [ratatui-markdown](https://github.com/celestia-island/ratatui-markdown), avec
  prise en charge des blocs HTML bruts pour que le balisage centré des README
  soit préservé.
- **Rendu via le VDom tairitsu** — les documents deviennent des arbres `VNode`
  sérialisés via `render_to_html`.
- **Multilingue** — un répertoire par langue, un sélecteur de langue intégré et
  une redirection racine vers l'anglais.
- **Auto-hébergé** — la documentation de Lagrange est construite par Lagrange
  lui-même.

## Licence

SySL-1.0 (Synthetic Source License). Voir [LICENSE](https://sysl.celestia.world).
