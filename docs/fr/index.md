# Lagrange

**Un système de rendu de documentation markdown basé sur pest.**

Lagrange analyse le markdown avec une grammaire pest, le restitue en HTML
via le DOM virtuel [tairitsu](https://github.com/celestia-island/tairitsu), et
assemble un site statique multilingue — mis en thème avec la palette
[hikari](https://github.com/celestia-island/hikari).

> Cette page elle-même est générée par Lagrange.

## Fonctionnalités

- **Analyseur markdown basé sur pest** — blocs + en ligne, inspiré de ratatui-markdown.
- **Rendu VDom tairitsu** — les documents deviennent des arbres `VNode`, sérialisés via
  `render_to_html`.
- **Multilingue** — un répertoire par langue, un sélecteur de langue intégré,
  et une redirection racine vers l'anglais.
- **Auto-hébergé** — la documentation de Lagrange elle-même est produite par Lagrange
  (`just docs`).

## Démarrage rapide

```bash
cargo run --release -- build docs --out target/site
```

Voir [Démarrage rapide](./guides/quickstart.md) et [Architecture](./guides/architecture.md).
