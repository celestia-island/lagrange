# Architecture

Lagrange est une crate unique (`lagrange-library`) fournissant une bibliothèque
et une CLI `lagrange`.

## Pipeline

```
markdown -> pest parser -> AST (Block / Inline)
                              |
                              v
                 tairitsu VDom (VNode) -> render_to_html -> HTML
```

## Modules

| Module | Responsabilité |
|--------|----------------|
| `markdown` | grammaire pest, AST, analyseur bloc + en ligne |
| `render` | AST -> tairitsu `VNode` -> chaîne HTML |
| `theme` | CSS généré depuis la palette hikari |
| `site` | parcourt l'arbre docs, rend chaque page, écrit un site statique |
| `cli` | le binaire `lagrange` |

## La boucle fermée

Lagrange rend sa propre documentation : `just docs` exécute
`lagrange build docs`, et le workflow CI déploie le résultat. Si vous lisez
ceci sur le site publié, il a été produit par Lagrange.

## Construit sur

- [tairitsu](https://github.com/celestia-island/tairitsu) — le DOM virtuel
  utilisé comme IR de rendu.
- [hikari](https://github.com/celestia-island/hikari) — la palette de couleurs utilisée
  pour la thématique.
- [pest](https://pest.rs) — le générateur d'analyseur.
