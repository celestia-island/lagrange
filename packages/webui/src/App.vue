<script setup lang="ts">
import { ref, computed } from "vue";
import type { KnowledgeBase } from "./composables/types";
import kbData from "./data/knowledge.json";
import ConsolePage from "./components/ConsolePage.vue";

const kb = kbData as unknown as KnowledgeBase;
const tab = ref<"content" | "console">("console");

const navItems = kb.documents.map((doc) => ({
  key: doc.document_id,
  label: doc.translations["en"]?.title ?? doc.document_id,
  children: Object.entries(doc.translations).map(([lang, t]) => ({
    key: `${doc.document_id}:${lang}`,
    label: `${t.title} (${lang})`,
  })),
}));

const currentDocId = ref<string | null>(null);
const currentLang = ref("en");

function onSelect(key: string) {
  const [docId, lang] = key.split(":");
  if (docId) { currentDocId.value = docId; currentLang.value = lang ?? "en"; }
  tab.value = "content";
}

const chunks = computed(() => {
  if (!currentDocId.value) return [];
  return kb.chunks.filter(
    (c) => c.document_id === currentDocId.value && c.lang === currentLang.value,
  );
});
</script>

<template>
  <div class="lagrange-shell">
    <nav class="lagrange-sidebar">
      <div class="lagrange-tabs">
        <button :class="{ active: tab === 'console' }" @click="tab = 'console'">Console</button>
        <button :class="{ active: tab === 'content' }" @click="tab = 'content'">Content</button>
      </div>
      <template v-if="tab === 'content'">
        <div v-for="item in navItems" :key="item.key" class="lagrange-nav-group">
          <div class="lagrange-nav-label">{{ item.label }}</div>
          <a v-for="child in item.children" :key="child.key" class="lagrange-nav-item"
             @click.prevent="onSelect(child.key)">{{ child.label }}</a>
        </div>
      </template>
    </nav>
    <main class="lagrange-content">
      <ConsolePage v-if="tab === 'console'" :kb="kb" />
      <template v-else>
        <div v-if="chunks.length === 0" class="lagrange-empty">Select a document.</div>
        <div v-else>
          <div v-for="chunk in chunks" :key="chunk.anchor" class="lagrange-chunk">
            <h3>{{ chunk.heading_path.join(" > ") }}</h3>
            <p>{{ chunk.title }}</p>
          </div>
        </div>
      </template>
    </main>
  </div>
</template>

<style>
.lagrange-shell { display: flex; min-height: 100vh; }
.lagrange-sidebar { width: 240px; border-right: 1px solid #e0e0e0; padding: 1rem; }
.lagrange-tabs { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
.lagrange-tabs button { padding: 0.3rem 0.8rem; border: 1px solid #ccc; border-radius: 4px; cursor: pointer; background: none; }
.lagrange-tabs button.active { background: #e8e8e8; font-weight: 600; }
.lagrange-nav-group { margin-bottom: 1rem; }
.lagrange-nav-label { font-weight: 600; }
.lagrange-nav-item { display: block; padding: 0.25rem 0.5rem; cursor: pointer; }
.lagrange-nav-item:hover { background: #f0f0f0; }
.lagrange-content { flex: 1; padding: 2rem; }
.lagrange-empty { color: #888; }
.lagrange-chunk { margin-bottom: 1.5rem; }
</style>
