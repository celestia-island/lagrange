// lagrange console entry app. Formerly App.vue: the SFC moved to TSX +
// a side SCSS file (family form, chest/hikari alignment). The component
// logic and the rendered tree are unchanged — same refs, same computeds,
// same slot layout on HkFaceShell. The SFC's style block was NOT scoped
// (a plain global <style>), so App.scss keeps those selectors verbatim
// and global; no root-class nesting applies here.
import { computed, createCommentVNode, defineComponent, ref } from "vue";
import {
  HkFaceShell,
  HkMenu,
  HkMarkdownRenderer,
  HkLogo,
} from "@celestia-island/hikari";
import type { KnowledgeBase } from "./composables/types";
import kbData from "./data/knowledge.json";
import "./App.scss";

export default defineComponent({
  name: "App",
  setup() {
    const kb = kbData as unknown as KnowledgeBase;
    const currentLang = ref("en");
    const currentDocId = ref<string | null>(null);
    const railStyle = ref("expanded");

    const SAMPLE_MARKDOWN = `# Quick start

Build the site for this very repo:

\`\`\`bash
git clone https://github.com/celestia-island/lagrange
cd lagrange
cargo run --release -- build --src docs --out target/site
\`\`\`

Open \`target/site/index.html\` — it redirects into the English book.

## Features

- **pest-based markdown parser** — block + inline
- **Multilingual** — one directory per language, built-in language switcher
- **Self-hosting** — Lagrange's own documentation is built by Lagrange
`;

    const navMenuItems = kb.documents.map((doc: import("./composables/types").DocumentInfo) => ({
      key: doc.document_id,
      label: doc.translations["en"]?.title ?? doc.document_id,
      children: Object.entries(doc.translations).map(([lang, t]) => ({
        key: `${doc.document_id}:${lang}`,
        label: `${t.title} (${lang})`,
      })),
    }));

    const previewPath = computed(() => {
      if (!currentDocId.value) return "";
      // Built-site pages live at <lang>/<slug>.html (site.rs output layout).
      return `${currentLang.value}/${currentDocId.value}.html`;
    });

    const chunks = computed(() => {
      if (!currentDocId.value) return [];
      return kb.chunks.filter((c: import("./composables/types").Chunk) =>
        c.document_id === currentDocId.value && c.lang === currentLang.value
      );
    });

    function onSelect(key: string) {
      const [docId, lang] = key.split(":");
      if (docId) {
        currentDocId.value = docId;
        currentLang.value = lang ?? "en";
      }
    }

    function onRailStyleChange(v: "expanded" | "rail") {
      railStyle.value = v;
      localStorage.setItem("lagrange-sidebar-style", v);
    }
    // SSR guard: localStorage only exists in the browser
    if (typeof window !== "undefined") {
      railStyle.value =
        (localStorage.getItem("lagrange-sidebar-style") as "expanded" | "rail") ?? "expanded";
    }

    return () => (
      <HkFaceShell
        sidebarCollapsed={false}
        sidebarWidth="260px"
        sidebarCollapsible
        navTitle="Lagrange"
        sidebarRailStyle={railStyle.value as any}
        onUpdate:sidebarRailStyle={onRailStyleChange as any}
      >
        {{
          sidebar: () => (
            <HkMenu
              variant="sidebar"
              open={true}
              items={navMenuItems}
              activeKey={currentDocId.value ? `${currentDocId.value}:${currentLang.value}` : undefined}
              defaultExpandGroups
              onSelect={onSelect}
            />
          ),
          content: () => (
            <div class="lagrange-render">
              <HkLogo size="md" alt="Lagrange" />
              <HkMarkdownRenderer content={SAMPLE_MARKDOWN} plain={false} />
              {chunks.value.length > 0 ? (
                <div class="lagrange-chunk-meta">
                  <h3>Chunk metadata ({chunks.value.length} chunks)</h3>
                  {chunks.value.map((c) => (
                    <div key={c.anchor} class="chunk-card">
                      <strong>{c.heading_path.join(" > ")}</strong>
                      <span class="chunk-anchor">{c.anchor}</span>
                      <span class="chunk-span">{c.span.start_line}–{c.span.end_line}</span>
                    </div>
                  ))}
                </div>
              ) : (
                // The template v-if's false-branch anchor: keeps the SSR
                // HTML byte-identical to the App.vue source (`<!--v-if-->`
                // instead of the bare `<!---->` an `&&` expression emits).
                createCommentVNode("v-if")
              )}
            </div>
          ),
          footer: () => (
            <span class="lagrange-footer-text">Lagrange — agent-ready docs</span>
          ),
        }}
      </HkFaceShell>
    );
  },
});
