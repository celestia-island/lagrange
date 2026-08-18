# Changelog

All notable changes to lagrange are documented in this file.
Versions before 0.2.0 predate this changelog.

## [0.2.8] - 2026-08-18

- Bridge the code-highlight class-name contract: hikari's component
  stylesheet styles `.hk-code-highlight-*` but lagrange's renderer emits
  `hi-code-highlight-*`, so every code block shipped unstyled despite the
  rules being present. The component CSS now duplicates each rule under the
  `hi-` alias, restoring container/line-number/token styling (verified:
  rendered keyword tokens carry the theme syntax colors).

## [0.2.7] - 2026-08-17


- Add a mobile navigation drawer: below 768px the sidebar becomes an
  off-canvas drawer behind a floating toggle (hamburger) with a scrim,
  Escape/link-click close and body scroll-lock, instead of a fixed
  half-screen strip; the content keeps the full viewport.
- Fix chrome i18n language mapping: `zh-Hans`/`zh-Hant` now resolve their
  UI strings (search placeholder, code-copy labels, nav toggle title)
  instead of silently falling back to English.
- Warn at build time when a language falls back to the default language's
  SUMMARY (sidebar titles stay untranslated) — names the exact missing path.
- Floor hikari-components at 0.3.20 so component stylesheets compile in
  crates.io layout (all `hi-*` component CSS used to be missing).

## [0.2.6] - 2026-08-16

- Publish 0.2.6 to crates.io (version alignment release; see git history).

## [0.2.5] - 2026-08-14

- Fix the CI gates for fmt drift, clippy lints and the JWT crypto provider.
- Unify the workspace crate versions and add the verify-versions gate.

## [0.2.4] - 2026-08-01

- Unify npm specs to caret-star and upgrade to the latest dependency series.

## [0.2.3] - 2026-07-18

- Add configurable content width and responsive padding via lagrange.toml.

## [0.2.2] - 2026-07-17

- Decode HTML entities in inline text and require scss!-fixed dependencies.

## [0.2.1] - 2026-07-16

- Add the --host flag, scss! layout, hero pages, div parsing and theme config.

## [0.2.0] - 2026-07-13

- Add live code blocks, overlay scrollbars, clipboard binding and multi-language rendering.
