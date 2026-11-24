// Sheet-shape guard: the console's two kinds of stylesheet must keep the
// reach they had as single-file components.
//
// The 2026-11 migration moved every `<style>` block out of its `.vue` and
// into a side `.scss`. Two shapes came across, and they are NOT
// interchangeable:
//
//   * `App.vue`'s block was `<style>` — plain, unscoped, global. Its
//     selectors must stay FLAT. Nesting one under a page class silently
//     narrows a rule the app relied on everywhere (e.g.
//     `.lagrange-footer-text` would stop matching outside `.lagrange-render`).
//   * the five panel components' blocks were `<style scoped>`. Their rules
//     MUST nest under the component's own root class, because the scoped
//     attribute is gone — flat rules would leak into every other view.
//
// Why a guard: the wrong shape in either direction is invisible to
// `vue-tsc`, `vite build` and the whole test suite (proved by mutation —
// narrowing `App.scss` still shipped and every gate stayed green), yet it
// changes what the browser paints. Nothing else in the pipeline can see it.
//
// Note the honest limit recorded with the migration: the five panel
// components are currently unreferenced by any entry point, so their
// sheets never reach a bundle and no render test can observe them. This
// guard therefore pins the SOURCE shape (which is what a future
// re-wiring would inherit), not a rendered result.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));

/** Every class selector in a sheet, with the rule depth it was written at.
 *
 *  Depth is tracked by BRACE COUNT, not by leading whitespace: SCSS allows
 *  `.a { .b { … } }` on one line, and a whitespace-only reader would call
 *  `.b` top-level — exactly the shape a careless narrowing edit produces.
 *  Comments (`//` and `/* … *\/`) are stripped first so a documented
 *  selector never counts as a real one. */
function selectors(src: string): { name: string; nested: boolean }[] {
  const code = src
    .replace(/\/\*[\s\S]*?\*\//g, " ")
    .replace(/\/\/[^\n]*/g, " ");
  const out: { name: string; nested: boolean }[] = [];
  // Depth the NEXT character sits at.
  let depth = 0;
  for (let i = 0; i < code.length; i += 1) {
    const ch = code[i]!;
    if (ch === "{") {
      depth += 1;
      continue;
    }
    if (ch === "}") {
      depth = Math.max(0, depth - 1);
      continue;
    }
    if (ch !== ".") continue;
    // A class selector only counts at the START of a compound selector:
    // the character before must be a delimiter (or the string start).
    const prev = i === 0 ? "" : code[i - 1]!;
    if (prev !== "" && !/[\s,{}>+~(]/.test(prev)) continue;
    const m = /^\.([A-Za-z][\w-]*)\s*[,{]/.exec(code.slice(i));
    if (m) out.push({ name: m[1]!, nested: depth > 0 });
  }
  return out;
}

/** The panel sheets whose `.vue` blocks were `<style scoped>`, with the
 *  root class each one must nest under. */
const SCOPED_SHEETS: ReadonlyArray<{ sheet: string; root: string }> = [
  { sheet: "components/AgentActivity.scss", root: "agent-activity" },
  { sheet: "components/ConsolePage.scss", root: "console-page" },
  { sheet: "components/DocTree.scss", root: "doc-tree" },
  { sheet: "components/PreviewPane.scss", root: "preview-pane" },
  { sheet: "components/SettingsPage.scss", root: "settings-page" },
];

/** Every sheet this guard reads. Kept in one place so the interpolation
 *  check below cannot drift from the shape checks above. */
function sheetsInScope(): string[] {
  return ["App.scss", ...SCOPED_SHEETS.map((entry) => entry.sheet)];
}

describe("migrated sheets keep their original reach", () => {
  it("no sheet hides a selector behind SCSS interpolation", () => {
    // `#{".chunk-span"} { … }` nests a rule for real while the literal text
    // the scanner looks for never appears — the narrowing G1 exists to catch
    // would pass unseen (proved by mutation on the real sheet). No sheet
    // needs interpolation today, so refusing it outright keeps the reader
    // honest instead of teaching it to parse SassScript.
    for (const sheet of sheetsInScope()) {
      const src = readFileSync(resolve(here, sheet), "utf8");
      expect(
        src.includes("#{"),
        `${sheet}: SCSS interpolation (#{…}) hides selectors from this guard's ` +
          `scanner — write the selector literally so its nesting depth is visible`,
      ).toBe(false);
    }
  });

  it("App.scss stays entirely flat (its block was never scoped)", () => {
    const src = readFileSync(resolve(here, "App.scss"), "utf8");
    const all = selectors(src);
    // Zero-scan is never green: this sheet declares the console's own classes.
    expect(all.length, "App.scss declares no selectors — the scan broke").toBeGreaterThan(3);
    const nested = all.filter((s) => s.nested).map((s) => `.${s.name}`);
    expect(
      nested,
      `App.scss must stay FLAT — its block was an unscoped <style> and these ` +
        `selectors were app-global; nesting them narrows the reach: ${nested.join(", ")}`,
    ).toEqual([]);
  });

  it("each scoped sheet still nests under its own root class", () => {
    for (const { sheet, root } of SCOPED_SHEETS) {
      const src = readFileSync(resolve(here, sheet), "utf8");
      const all = selectors(src);
      expect(all.length, `${sheet} declares no selectors — the scan broke`).toBeGreaterThan(0);
      // The root class itself must be a top-level declaration …
      expect(
        all.some((s) => s.name === root && !s.nested),
        `${sheet}: root class .${root} is not declared at the top level`,
      ).toBe(true);
      // … and every other rule must sit inside it.
      const leaked = all.filter((s) => s.name !== root && !s.nested).map((s) => `.${s.name}`);
      expect(
        leaked,
        `${sheet}: these rules left .${root} and are now GLOBAL — the scoped ` +
          `attribute is gone, so flat rules leak into every other view: ${leaked.join(", ")}`,
      ).toEqual([]);
    }
  });
});
