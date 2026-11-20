<script setup lang="ts">
import { computed } from "vue";
import type { KnowledgeBase } from "../composables/types";

const props = defineProps<{ kb: KnowledgeBase }>();

interface TreeDoc {
  id: string;
  title: string;
  kind: string;
  tags: string[];
  langs: { code: string; title: string; path: string; revision: string }[];
}

const tree = computed<TreeDoc[]>(() =>
  props.kb.documents.map((doc) => ({
    id: doc.document_id,
    title: doc.translations["en"]?.title ?? doc.document_id,
    kind: doc.kind,
    tags: doc.tags,
    langs: Object.entries(doc.translations)
      .map(([code, t]) => ({ code, ... t }))
      .sort((a, b) => a.code.localeCompare(b.code)),
  })),
);
</script>

<template>
  <div class="doc-tree">
    <div v-for="doc in tree" :key="doc.id" class="doc-node">
      <div class="doc-header">
        <span class="doc-title">{{ doc.title }}</span>
        <span class="doc-id">{{ doc.id }}</span>
        <span class="doc-kind badge">{{ doc.kind }}</span>
      </div>
      <div v-if="doc.tags.length" class="doc-tags">
        <span v-for="tag in doc.tags" :key="tag" class="tag">{{ tag }}</span>
      </div>
      <div class="lang-list">
        <span v-for="lang in doc.langs" :key="lang.code" class="lang-item">
          <span class="lang-code">{{ lang.code }}</span>
          <span class="lang-title">{{ lang.title }}</span>
          <span class="lang-rev" :title="lang.revision">{{ lang.revision.slice(0, 8) }}</span>
        </span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.doc-tree { display: flex; flex-direction: column; gap: 1rem; }
.doc-node { border: 1px solid #e0e0e0; border-radius: 8px; padding: 1rem; background: #fafafa; }
.doc-header { display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem; flex-wrap: wrap; }
.doc-title { font-weight: 700; font-size: 1.1rem; }
.doc-id { font-family: monospace; font-size: 0.8rem; color: #888; }
.badge { background: #e0e7ff; color: #3730a3; padding: 0.1rem 0.5rem; border-radius: 99px; font-size: 0.75rem; }
.doc-tags { margin-bottom: 0.5rem; display: flex; gap: 0.25rem; flex-wrap: wrap; }
.tag { background: #dcfce7; color: #166534; padding: 0.1rem 0.4rem; border-radius: 99px; font-size: 0.7rem; }
.lang-list { display: flex; flex-direction: column; gap: 0.15rem; }
.lang-item { display: flex; gap: 0.5rem; font-size: 0.85rem; padding: 0.15rem 0.5rem; border-radius: 4px; }
.lang-item:hover { background: #f0f0f0; }
.lang-code { font-weight: 600; min-width: 5rem; color: #555; }
.lang-title { flex: 1; }
.lang-rev { font-family: monospace; font-size: 0.75rem; color: #aaa; }
</style>
