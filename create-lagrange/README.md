# create-lagrange

Scaffold a new [lagrange](https://github.com/celestia-island/lagrange) documentation site with one command.

```bash
npx create-lagrange my-docs
```

This downloads the prebuilt `lagrange` binary from GitHub Releases (no Rust toolchain needed) and runs `lagrange init` in the target directory, generating `lagrange.toml` + `docs/en/` skeleton.

## Usage

```bash
# Basic — creates ./my-docs with defaults
npx create-lagrange my-docs

# With a title and native comments
npx create-lagrange my-docs --title "My Project" --comments native

# GitHub Discussions comments
npx create-lagrange my-docs --comments github-discussions
```

Then:

```bash
cd my-docs
lagrange build --src . --out _site
```

## How it works

- `postinstall` (`install.js`): detects the platform, downloads the matching `lagrange-v<ver>-<target>.tar.gz` from the latest GitHub Release, extracts the binary to `bin/`.
- `index.js`: spawns `lagrange init` with forwarded args.

No bundled binary — the package stays tiny; the ~5MB binary is fetched on demand.

## Alternatives

If you have Rust:

```bash
cargo install lagrange-library
cargo binstall lagrange-library   # prebuilt binary, same as this npm package
lagrange init --dir my-docs
```
