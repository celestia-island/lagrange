<script setup lang="ts">
import { computed } from "vue";
import type { KnowledgeBase } from "../composables/types";

const props = defineProps<{ kb: KnowledgeBase }>();

const docStats = computed(() => {
  const docs = props.kb.documents;
  const totalTranslations = docs.reduce((acc, d) => acc + Object.keys(d.translations).length, 0);
  const totalChunks = props.kb.chunks.length;
  const langs = [...new Set(docs.flatMap((d) => Object.keys(d.translations)))].sort();
  return { docCount: docs.length, translationCount: totalTranslations, chunkCount: totalChunks, langs };
});

const buildInfo = computed(() => ({
  schemaVersion: props.kb.schema_version,
  parserVersion: props.kb.parser_version,
  generator: props.kb.generator,
  sourceCommit: props.kb.source.source_commit ?? "not in a git checkout",
}));
</script>

<template>
  <div class="console-page">
    <h2>Build Diagnostics</h2>
    <div class="stat-grid">
      <div class="stat-card">
        <div class="stat-value">{{ docStats.docCount }}</div>
        <div class="stat-label">Documents</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ docStats.translationCount }}</div>
        <div class="stat-label">Translations</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{{ docStats.chunkCount }}</div>
        <div class="stat-label">Chunks</div>
      </div>
    </div>
    <div class="stat-grid">
      <div class="stat-card">
        <div class="stat-label">Languages</div>
        <div class="stat-detail">{{ docStats.langs.join(", ") }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Source commit</div>
        <div class="stat-detail">{{ buildInfo.sourceCommit }}</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Parser</div>
        <div class="stat-detail">v{{ buildInfo.parserVersion }} (schema v{{ buildInfo.schemaVersion }})</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.console-page { padding: 1rem; }
.stat-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; margin-bottom: 1.5rem; }
.stat-card { border: 1px solid #e0e0e0; border-radius: 6px; padding: 1rem; }
.stat-value { font-size: 2rem; font-weight: 700; }
.stat-label { font-size: 0.85rem; color: #666; margin-top: 0.25rem; }
.stat-detail { font-size: 0.8rem; word-break: break-all; }
</style>
