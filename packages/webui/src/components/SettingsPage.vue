<script setup lang="ts">
import { ref, computed } from "vue";
import type { KnowledgeBase } from "../composables/types";

const props = defineProps<{ kb: KnowledgeBase }>();

const siteTitle = ref("Lagrange");
const defaultLang = ref("en");
const contentWidth = ref("800px");

const langs = computed(() => props.kb.source.langs);
const sourceCommit = computed(() => props.kb.source.source_commit ?? "unknown");
const parserVersion = computed(() => `v${props.kb.parser_version}`);
const schemaVersion = computed(() => `v${props.kb.schema_version}`);

const buildInfo = computed(() => ({
  generator: props.kb.generator ?? "lagrange",
  sourceCommit: (props.kb.source?.source_commit ?? "unknown").slice(0, 12),
  parserVersion: `v${props.kb.parser_version}`,
  schemaVersion: `v${props.kb.schema_version}`,
  langs: props.kb.source?.langs?.join(", ") ?? "",
}));
</script>

<template>
  <div class="settings-page">
    <h2>Settings</h2>

    <section class="settings-section">
      <h3>Language</h3>
      <p class="hint">Default language for the knowledge base.</p>
      <select v-model="defaultLang" class="settings-select">
        <option v-for="lang in langs" :key="lang" :value="lang">{{ lang }}</option>
      </select>
    </section>

    <section class="settings-section">
      <h3>Site</h3>
      <div class="settings-field">
        <label>Site title</label>
        <input v-model="siteTitle" type="text" class="settings-input" />
      </div>
      <div class="settings-field">
        <label>Content width</label>
        <input v-model="contentWidth" type="text" class="settings-input" />
      </div>
    </section>

    <section class="settings-section">
      <h3>About</h3>
      <div class="about-grid">
        <div class="about-item"><span class="about-label">Generator</span><span>{{ buildInfo.generator }}</span></div>
        <div class="about-item"><span class="about-label">Source commit</span><span class="mono">{{ sourceCommit }}</span></div>
        <div class="about-item"><span class="about-label">Parser</span><span>{{ parserVersion }}</span></div>
        <div class="about-item"><span class="about-label">Schema</span><span>{{ schemaVersion }}</span></div>
        <div class="about-item"><span class="about-label">Languages</span><span>{{ langs.join(", ") }}</span></div>
      </div>
    </section>
  </div>
</template>


<style scoped>
.settings-page { padding: 1rem; max-width: 600px; }
.settings-section { margin-bottom: 2rem; }
.settings-section h3 { font-size: 1rem; font-weight: 700; margin-bottom: 0.5rem; }
.hint { font-size: 0.8rem; color: #888; margin-bottom: 0.5rem; }
.settings-select { padding: 0.4rem; border: 1px solid #ccc; border-radius: 4px; min-width: 200px; }
.settings-field { margin-bottom: 0.75rem; }
.settings-field label { display: block; font-size: 0.85rem; font-weight: 600; margin-bottom: 0.25rem; }
.settings-input { padding: 0.4rem; border: 1px solid #ccc; border-radius: 4px; width: 100%; max-width: 400px; }
.about-grid { display: grid; gap: 0.5rem; }
.about-item { display: flex; gap: 1rem; font-size: 0.85rem; }
.about-label { min-width: 140px; font-weight: 600; color: #555; }
.mono { font-family: monospace; }
</style>
