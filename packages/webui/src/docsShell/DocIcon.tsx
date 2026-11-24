// Sidebar nav glyphs for the docs shell — the three fallback tiers the
// family's rail grammar expects:
//
//   - a page with frontmatter `icon: "book-open"` renders that lucide icon;
//   - `icon: "/img/x.svg"` (or any URL) renders that image;
//   - no icon at all falls back to the dot language: a SOLID dot for group
//     rows, a HOLLOW dot for page rows — so a collapsed rail never strands
//     a destination behind a missing glyph (hikari's rail falls back to the
//     label initial, which for long doc titles reads as noise).
//
// SHAPE CONTRACT: hikari renders these as `h(item.icon, { size: 16 })` —
// ONLY a size prop crosses the boundary. The glyph value therefore lives in
// the component's closure (the same pattern as lucide's
// `createLucideIcon(name, nodes)`), never in a prop: a prop would be
// `undefined` inside hikari's render and crash the glyph resolver.
//
// Lucide names load through a lazy chunk (`lucide-vue-next` is already a
// dependency for the search button; the dynamic import keeps the full icon
// set out of the entry bundle until a page actually uses a named icon).
// SSR behavior: @vue/server-renderer AWAITS async setup, so the built HTML
// carries the resolved lucide SVG and the client hydration replaces the
// async wrapper's placeholder — the icon "pops in" on hydration. A
// frontmatter name with no lucide counterpart degrades to an empty glyph
// (loud console warning) rather than failing the SSR render.
import { defineComponent, defineAsyncComponent, h, type Component } from "vue";

/** A filled circle — the GROUP glyph (sections have children). */
export function docIconDotSolid(): Component {
  return defineComponent({
    name: "DocIconDotSolid",
    props: { size: { type: Number, default: 16 } },
    setup(props) {
      return () =>
        h("span", {
          "aria-hidden": "true",
          style: {
            display: "inline-block",
            width: `${Math.max(6, Math.round(props.size * 0.375))}px`,
            height: `${Math.max(6, Math.round(props.size * 0.375))}px`,
            borderRadius: "50%",
            background: "currentColor",
            flex: "none",
          },
        });
    },
  });
}

/** A hollow circle — the PAGE glyph (documents are destinations). */
export function docIconDotHollow(): Component {
  return defineComponent({
    name: "DocIconDotHollow",
    props: { size: { type: Number, default: 16 } },
    setup(props) {
      return () =>
        h("span", {
          "aria-hidden": "true",
          style: {
            display: "inline-block",
            width: `${Math.max(7, Math.round(props.size * 0.44))}px`,
            height: `${Math.max(7, Math.round(props.size * 0.44))}px`,
            borderRadius: "50%",
            border: "1.5px solid currentColor",
            boxSizing: "border-box",
            flex: "none",
          },
        });
    },
  });
}

const lucideCache = new Map<string, Component>();

/** A lucide icon by PascalCase name, loaded lazily; kebab-case converts. */
function lucideIcon(name: string): Component {
  const pascal = name
    .split("-")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
  let cached = lucideCache.get(pascal);
  if (!cached) {
    cached = defineAsyncComponent(() =>
      import("lucide-vue-next").then((mod) => {
        const icon = (mod as Record<string, unknown>)[pascal];
        if (typeof icon !== "function" && typeof icon !== "object") {
          // A declared-but-unknown name must not fail the SSR render —
          // degrade to an empty glyph (the row keeps its label).
          console.warn(`lucide icon not found: ${pascal}`);
          return defineComponent({ render: () => h("span", { "aria-hidden": "true" }) });
        }
        return icon as Component;
      }),
    );
    lucideCache.set(pascal, cached);
  }
  return cached;
}

/** Is this icon value a URL (image) rather than a lucide name? Anything
 * with a scheme or a root-relative path counts; bare kebab-case names go
 * to lucide. */
function isIconUrl(value: string): boolean {
  return /^(https?:)?\/\//.test(value) || value.startsWith("/");
}

/** The glyph component for a frontmatter `icon` value: URL → image,
 * kebab name → lucide. The value is captured in the closure — hikari
 * renders `h(glyph, { size })` and nothing else crosses. */
export function docIconGlyph(value: string): Component {
  if (isIconUrl(value)) {
    return defineComponent({
      name: "DocIconImage",
      props: { size: { type: Number, default: 16 } },
      setup(props) {
        return () =>
          h("img", {
            src: value,
            alt: "",
            width: props.size,
            height: props.size,
            style: { flex: "none", verticalAlign: "middle" },
          });
      },
    });
  }
  return defineComponent({
    name: "DocIconLucide",
    props: { size: { type: Number, default: 16 } },
    setup(props) {
      return () => h(lucideIcon(value), { size: props.size, "aria-hidden": "true" });
    },
  });
}
