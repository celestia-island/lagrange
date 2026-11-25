// Compile the TypeScript sources Rust embeds into the JavaScript artifacts the
// binary carries.
//
// Six assets are `include_str!`-embedded at `cargo build` time:
//
//     src/comments/scrollbar.ts   -> src/comments/scrollbar.js
//     src/comments/modal.ts       -> src/comments/modal.js
//     src/comments/clipboard.ts   -> src/comments/clipboard.js
//     src/comments/ui.ts          -> src/comments/ui.js
//     src/comments/runtime.ts     -> src/comments/runtime.js
//     src/diagram.ts              -> src/diagram.js
//
// `src/**/*.ts` is the SOURCE; the sibling `.js` is BUILD OUTPUT, committed
// because the Rust build is the consumer and has to work from a bare checkout
// (and from a crates.io tarball, where `src/` is what ships). The `.js` is
// never hand-edited: `cargo test --test embedded_ts` fails whenever a
// committed artifact stops matching what this compiler emits, and fails the
// same way when the two sides disagree about WHICH files are compiled. That
// test — not a snapshot — is the oracle, so the lock can never drift from the
// pipeline it claims to pin.
//
// Rebuild after editing any `.ts`:
//
//     node tools/scripts/build-embedded-ts.ts      (or `just embedded`)
//
// Options:
//   --check   compile in memory and exit 1 if any artifact differs from what
//             is on disk, or if the declared set disagrees with the tree.
//             Writes nothing. This is what the Rust test runs.
//
// ── the module problem, and how the output stays a classic script ─────────
//
// A `.ts` file without an import/export is a SCRIPT to esbuild, and a script
// may not `declare global` — so every file here ends with `export {}` to be
// scoped. But these assets are served/embedded as CLASSIC scripts: an inline
// `<script>` is parsed as a script, and a stray top-level `export {}` is a
// SyntaxError in one. Marking the file a module therefore changes the bytes
// Rust embeds. `format: "iife"` swallows the export (the whole file becomes
// one IIFE and the empty export disappears), and every source is itself
// already wrapped in its own IIFE, so the extra wrapper only scrubs the
// module marker. `checkWrapper` asserts that is what happened: a top-level
// `export`/`import` reaching an embedded artifact is a hard failure, not a
// surprise in the browser.
//
// Determinism: esbuild's transform is a pure function of (source text,
// options) and this file passes fixed options, so two runs are byte-identical.
// `just embedded` runs the compile twice and compares sha256 so the claim
// stays checkable rather than asserted.
//
// This script is itself TypeScript run straight off the tree by Node 24's type
// stripping, so it must stay ERASABLE — no enums, no namespaces, no parameter
// properties (nothing the stripper has to translate).

import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { transform } from "esbuild";

const here = dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = join(here, "..", "..");

/** One embedded asset: where its TypeScript lives and where it emits. */
interface Asset {
  /** Repo-relative path of the authored source. */
  source: string;
  /** Repo-relative path of the emitted artifact (what `include_str!` reads). */
  artifact: string;
}

/**
 * The embedded set. This list is the contract: the Rust side must embed the
 * artifact of every entry, and nothing may appear beside a source that is not
 * listed here. `embedded_ts.rs` checks both directions against the real tree,
 * so an asset can be neither forgotten nor smuggled in.
 */
export const ASSETS: readonly Asset[] = [
  { source: "src/comments/scrollbar.ts", artifact: "src/comments/scrollbar.js" },
  { source: "src/comments/modal.ts", artifact: "src/comments/modal.js" },
  { source: "src/comments/clipboard.ts", artifact: "src/comments/clipboard.js" },
  { source: "src/comments/ui.ts", artifact: "src/comments/ui.js" },
  { source: "src/comments/runtime.ts", artifact: "src/comments/runtime.js" },
  { source: "src/diagram.ts", artifact: "src/diagram.js" },
];

/**
 * The banner every artifact carries. Embedded bytes have no filename and no
 * stack frame, so the artifact says where its source is and how to rebuild it
 * — the two facts a debugger in a shipped page needs. It is a byte-literal
 * (no timestamp, no path discovered from disk), so the output stays
 * reproducible.
 */
const BANNER =
  "/* GENERATED FROM %SOURCE% — do not edit.\n" +
  " * Rebuild: node tools/scripts/build-embedded-ts.ts (or `just embedded`).\n" +
  " * The .js beside each source is build output; the pairing is locked by\n" +
  " * cargo test --test embedded_ts. */\n";

/** The esbuild transform options every artifact shares. */
const TRANSFORM_OPTIONS = {
  loader: "ts",
  // Only types are meant to change: the embedded runtime keeps the ES5 shapes
  // it has always shipped (var, function expressions, no arrows), and a newer
  // feature sneaking in fails the compile instead of reaching a page.
  target: "es2015",
  charset: "utf8",
  legalComments: "none",
  sourcemap: false,
  minify: false,
  // Swallows the `export {}` module marker without changing behavior; see the
  // header. checkWrapper enforces the outcome.
  format: "iife",
} as const;

/**
 * A top-level `export`/`import` in an embedded artifact means the module
 * marker survived and an inline `<script>` would throw a SyntaxError. Detect
 * it structurally (line start = column 0) so a string that merely contains
 * "export" cannot report a false failure.
 */
export function checkWrapper(code: string, sourcePath: string): void {
  const offending = code.split("\n").filter((line) => /^(export|import)\b/.test(line));
  if (offending.length > 0) {
    throw new Error(
      `${sourcePath}: emitted artifact keeps a module marker (${offending[0].trim()}) — ` +
        `an inline <script> would be a SyntaxError; the iife wrapper in TRANSFORM_OPTIONS is what scrubs it`,
    );
  }
  if (!code.includes("(() => {")) {
    throw new Error(`${sourcePath}: emitted artifact lost its iife wrapper`);
  }
}

/** Compile one source text into the artifact bytes. Pure; writes nothing. */
export async function compile(sourceText: string, sourcePath: string): Promise<string> {
  const banner = BANNER.replace("%SOURCE%", sourcePath);
  const result = await transform(sourceText, TRANSFORM_OPTIONS);
  if (result.warnings.length > 0) {
    const detail = result.warnings
      .map((w) => `${w.text} (${w.location?.line}:${w.location?.column})`)
      .join("; ");
    throw new Error(`${sourcePath}: esbuild warned — ${detail}`);
  }
  checkWrapper(result.code, sourcePath);
  return banner + result.code;
}

/** Read + compile every asset, keyed by artifact path. Writes nothing. */
export async function compileAll(): Promise<Map<string, string>> {
  const out = new Map<string, string>();
  for (const asset of ASSETS) {
    const sourceText = await readFile(join(REPO_ROOT, asset.source), "utf8");
    out.set(asset.artifact, await compile(sourceText, asset.source));
  }
  return out;
}

/**
 * The files living beside an embedded source that claim to be its artifact.
 * `src/comments/*.js` must be exactly {scrollbar, modal, clipboard, ui,
 * runtime}.js — a stray `foo.js` with no `.ts` is either dead weight or a
 * source someone forgot to convert, and either way it breaks the contract that
 * the directory is compiled output. Directories are derived from ASSETS, so a
 * new asset's neighbourhood is covered without a second list to forget.
 */
async function siblingArtifacts(): Promise<string[]> {
  const dirs = [...new Set(ASSETS.map((a) => a.artifact.slice(0, a.artifact.lastIndexOf("/"))))].sort();
  const found: string[] = [];
  for (const dir of dirs) {
    for (const name of await readdir(join(REPO_ROOT, dir))) {
      if (name.endsWith(".js")) found.push(`${dir}/${name}`);
    }
  }
  return found.sort();
}

/**
 * Which declared artifacts disagree with the compile, plus any set mismatch.
 * Empty = the tree is exactly this pipeline's output.
 */
export async function drift(): Promise<string[]> {
  const problems: string[] = [];
  const compiled = await compileAll();
  for (const [artifact, expected] of compiled) {
    let actual: string | null = null;
    try {
      actual = await readFile(join(REPO_ROOT, artifact), "utf8");
    } catch {
      actual = null;
    }
    if (actual === null) {
      problems.push(`${artifact}: MISSING (run node tools/scripts/build-embedded-ts.ts)`);
    } else if (actual !== expected) {
      problems.push(`${artifact}: STALE or hand-edited (run node tools/scripts/build-embedded-ts.ts)`);
    }
  }
  const declared = new Set(ASSETS.map((a) => a.artifact));
  const present = new Set(await siblingArtifacts());
  for (const found of present) {
    if (!declared.has(found)) problems.push(`${found}: not in ASSETS (no .ts source claims it)`);
  }
  for (const artifact of declared) {
    if (!present.has(artifact)) problems.push(`${artifact}: declared in ASSETS but absent from the tree`);
  }
  return problems;
}

/** Refuse to run outside a checkout that actually contains the sources. */
async function assertCheckout(): Promise<void> {
  for (const asset of ASSETS) {
    try {
      await stat(join(REPO_ROOT, asset.source));
    } catch {
      throw new Error(
        `embedded TS source is missing: ${asset.source} (generated trees — a crates.io tarball — ` +
          `ship the emitted .js, not the .ts)`,
      );
    }
  }
}

async function main(argv: readonly string[]): Promise<number> {
  const check = argv.includes("--check");
  await assertCheckout();
  if (check) {
    const problems = await drift();
    if (problems.length > 0) {
      console.error(`embedded TS drift (${problems.length}):`);
      for (const problem of problems) console.error(`  - ${problem}`);
      return 1;
    }
    console.log(`embedded TS: ${ASSETS.length} artifacts match ${ASSETS.length} sources`);
    return 0;
  }
  let written = 0;
  for (const [artifact, bytes] of await compileAll()) {
    const path = join(REPO_ROOT, artifact);
    await mkdir(dirname(path), { recursive: true });
    let current: string | null = null;
    try {
      current = await readFile(path, "utf8");
    } catch {
      current = null;
    }
    if (current !== bytes) {
      await writeFile(path, bytes, "utf8");
      written += 1;
    }
  }
  const after = await drift();
  if (after.length > 0) {
    console.error("embedded TS: still inconsistent after writing:");
    for (const problem of after) console.error(`  - ${problem}`);
    return 1;
  }
  console.log(
    `embedded TS: compiled ${ASSETS.length} sources -> ${ASSETS.length} artifacts (${written} written)`,
  );
  return 0;
}

// Only run when executed directly, so a test can import `drift()` from the
// same file without its `process.exit` firing.
const invokedDirectly =
  process.argv[1] !== undefined && pathToFileURL(process.argv[1]).href === import.meta.url;

if (invokedDirectly) {
  process.exit(await main(process.argv.slice(2)));
}
