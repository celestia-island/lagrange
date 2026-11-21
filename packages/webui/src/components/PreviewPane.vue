<script setup lang="ts">
import { ref, watch } from "vue";

const props = defineProps<{ path: string }>();
const html = ref("");
const status = ref<"idle" | "loading" | "ok" | "unavailable">("idle");

async function load(path: string) {
  if (!path) {
    status.value = "idle";
    html.value = "";
    return;
  }
  status.value = "loading";
  try {
    const res = await fetch(path, { cache: "no-store" });
    if (!res.ok) {
      status.value = "unavailable";
      html.value = "";
      return;
    }
    html.value = await res.text();
    status.value = "ok";
  } catch {
    status.value = "unavailable";
    html.value = "";
  }
}

watch(() => props.path, (p) => load(p), { immediate: true });
</script>

<template>
  <div class="preview-pane">
    <div class="preview-head">
      <span class="preview-title">{{ path || "No page selected" }}</span>
      <span class="preview-status" :data-status="status">{{ status }}</span>
    </div>
    <p v-if="status === 'unavailable'" class="hint">
      This page is not reachable from here. Preview works when the console
      is served by <code>lagrange dev --port N</code> (which serves the
      built site, including these pages) or from the deployed site itself.
    </p>
    <iframe
      v-if="status === 'ok'"
      class="preview-frame"
      :srcdoc="html"
      sandbox="allow-same-origin"
      title="Document preview"
    ></iframe>
  </div>
</template>

<style scoped>
.preview-pane { padding: 1rem; }
.preview-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
.preview-title { font-family: monospace; font-size: 0.85rem; }
.preview-status[data-status="ok"] { color: #16a34a; }
.preview-status[data-status="loading"] { color: #d97706; }
.preview-status[data-status="unavailable"], .preview-status[data-status="idle"] { color: #888; }
.hint { color: #888; font-size: 0.85rem; }
.preview-frame { width: 100%; height: 60vh; border: 1px solid #e0e0e0; border-radius: 6px; background: #fff; }
</style>
