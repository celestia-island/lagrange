<script setup lang="ts">
import { ref, computed } from "vue";
import {
  HkFaceShell,
  HkMenu,
  HkMarkdownRenderer,
  HkLogo,
} from "@celestia-island/hikari";
import type { KnowledgeBase } from "./composables/types";
import kbData from "./data/knowledge.json";

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
railStyle.value =
  (localStorage.getItem("lagrange-sidebar-style") as "expanded" | "rail") ?? "expanded";
</script>

<template>
  <HkFaceShell
    :sidebar-collapsed="false"
    sidebar-width="260px"
    sidebar-collapsible
    nav-title="Lagrange"
    :sidebar-rail-style="railStyle as any"
    @update:sidebar-rail-style="onRailStyleChange as any"
  >
    <template #sidebar>
      <HkMenu
        variant="sidebar"
        :open="true"
        :items="navMenuItems"
        :active-key="currentDocId ? `${currentDocId}:${currentLang}` : undefined"
        default-expand-groups
        @select="onSelect"
      />
    </template>
    <template #content>
      <div class="lagrange-render">
        <HkLogo size="md" alt="Lagrange" />
        <HkMarkdownRenderer :content="SAMPLE_MARKDOWN" :plain="false" />
        <div v-if="chunks.length > 0" class="lagrange-chunk-meta">
          <h3>Chunk metadata ({{ chunks.length }} chunks)</h3>
          <div v-for="c in chunks" :key="c.anchor" class="chunk-card">
            <strong>{{ c.heading_path.join(" > ") }}</strong>
            <span class="chunk-anchor">{{ c.anchor }}</span>
            <span class="chunk-span">{{ c.span.start_line }}–{{ c.span.end_line }}</span>
          </div>
        </div>
      </div>
    </template>
    <template #footer>
      <span class="lagrange-footer-text">Lagrange — agent-ready docs</span>
    </template>
  </HkFaceShell>
</template>

<style>
.lagrange-render { max-width: 800px; margin: 0 auto; }
.lagrange-chunk-meta { margin-top: 2rem; }
.chunk-card { display: flex; gap: 0.5rem; padding: 0.5rem; border: 1px solid #e0e0e0; border-radius: 4px; margin-bottom: 0.5rem; font-size: 0.85rem; align-items: center; flex-wrap: wrap; }
.chunk-anchor { font-family: monospace; color: #7aa2f7; }
.chunk-span { color: #888; }
.lagrange-footer-text { font-size: 0.8rem; opacity: 0.6; }
</style>
