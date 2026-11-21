<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";

interface ActivityEntry {
  ts: string;
  tool: string;
  ok: boolean;
  detail: string;
}

const entries = ref<ActivityEntry[]>([]);
const available = ref(true);
const error = ref("");
let timer: ReturnType<typeof setInterval> | undefined;

async function refresh() {
  try {
    const res = await fetch("/mcp-activity.json", { cache: "no-store" });
    if (!res.ok) {
      available.value = false;
      entries.value = [];
      return;
    }
    available.value = true;
    const data = (await res.json()) as ActivityEntry[];
    entries.value = Array.isArray(data) ? data : [];
    error.value = "";
  } catch (e) {
    available.value = false;
    error.value = e instanceof Error ? e.message : String(e);
  }
}

onMounted(() => {
  refresh();
  // Poll every 5 s while the tab is open; the dev server mutates the
  // feed as agents call MCP tools.
  timer = setInterval(refresh, 5000);
});
onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});
</script>

<template>
  <div class="agent-activity">
    <h2>Agent Activity</h2>
    <p v-if="!available" class="hint">
      No activity feed at <code>/mcp-activity.json</code> — start the dev
      server with <code>lagrange dev --mcp --port N</code> and open this
      console through it.
    </p>
    <p v-else-if="entries.length === 0" class="hint">
      No tool calls recorded yet. Call any MCP tool
      (<code>docs.search</code>, <code>docs.write</code>, …) from an agent.
    </p>
    <table v-else class="activity-table">
      <thead>
        <tr><th>Time (UTC)</th><th>Tool</th><th>Result</th><th>Detail</th></tr>
      </thead>
      <tbody>
        <tr v-for="(e, i) in entries" :key="`${e.ts}-${i}`">
          <td class="mono">{{ e.ts }}</td>
          <td><code>{{ e.tool }}</code></td>
          <td><span :class="e.ok ? 'ok' : 'fail'">{{ e.ok ? "ok" : "error" }}</span></td>
          <td class="detail">{{ e.detail }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.agent-activity { padding: 1rem; }
.hint { color: #888; font-size: 0.85rem; }
.activity-table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
.activity-table th, .activity-table td { text-align: left; padding: 0.35rem 0.6rem; border-bottom: 1px solid #eee; }
.mono { font-family: monospace; font-size: 0.8rem; }
.ok { color: #16a34a; font-weight: 600; }
.fail { color: #dc2626; font-weight: 600; }
.detail { max-width: 24rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
