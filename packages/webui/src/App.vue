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

const navMenuItems = kb.documents.map((doc) => ({
  key: doc.document_id,
  label: doc.translations["en"]?.title ?? doc.document_id,
  children: Object.entries(doc.translations).map(([lang, t]) => ({
    key: `${doc.document_id}:${lang}`,
    label: `${t.title} (${lang})`,
  })),
}));

const chunks = computed(() => {
  if (!currentDocId.value) return [];
  return kb.chunks.filter(
    (c) => c.document_id === currentDocId.value && c.lang === currentLang.value,
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
    :sidebar-rail-style="railStyle"
    @update:sidebar-rail-style="onRailStyleChange"
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
    <template #default>
      <div v-if="chunks.length === 0" class="lagrange-empty">
        Select a document from the sidebar.
      </div>
      <div v-else class="lagrange-chunks">
        <div v-for="chunk in chunks" :key="chunk.anchor" class="lagrange-chunk">
          <HkMarkdownRenderer :content="chunk.title" :plain="false" />
        </div>
      </div>
    </template>
    <template #footer>
      <span class="lagrange-footer-text">
        Lagrange — agent-ready documentation knowledge base
      </span>
    </template>
  </HkFaceShell>
</template>

<style>
.lagrange-empty { padding: 2rem; color: #888; }
.lagrange-chunks { padding: 0 1rem; }
.lagrange-chunk { margin-bottom: 1.5rem; }
.lagrange-footer-text { font-size: 0.8rem; opacity: 0.6; }
</style>
