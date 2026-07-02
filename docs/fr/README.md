<p align="center"><img src="../logo.webp" alt="Lagrange" width="240" /></p>

<h1 align="center">Lagrange</h1>

<p align="center"><strong>Un moteur de rendu de documentation Markdown basé sur pest — VDom tairitsu + palette hikari, multilingue prêt à l'emploi.</strong></p>

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
**Français** ·
[Español](../es/README.md) ·
[Русский](../ru/README.md) ·
[العربية](../ar/README.md)

</div>

## Introduction

Lagrange transforme un dossier Markdown en un site de documentation statique
multilingue. Il parse le Markdown avec une grammaire [pest](https://pest.rs)
écrite à la main, restitue l'AST en HTML via le DOM virtuel
[tairitsu](https://github.com/celestia-island/tairitsu) et applique le thème
avec la palette [hikari](https://github.com/celestia-island/hikari). Un
répertoire par langue, un sélecteur de langue intégré et une redirection racine
vers l'anglais — aucun framework JavaScript, pas de mdBook, pas de chaîne Node.

## Démarrage rapide

```bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
```

## Déploiement

Le répertoire de sortie est statique — déployable sur **GitHub Pages** (action
composite incluse), **Cloudflare Pages** ou **Vercel**.

## Fonctionnalités

- **Parseur Markdown basé sur pest** (blocs + inline, blocs HTML bruts préservés)
- **Rendu via le VDom tairitsu**
- **Multilingue** — un répertoire par langue, sélecteur intégré
- **Auto-hébergé** — la documentation de Lagrange est construite par Lagrange

## Licence

SySL-1.0 (Synthetic Source License). Voir [LICENSE](../../LICENSE).
