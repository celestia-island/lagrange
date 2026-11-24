// Agent activity table for the console: polls /mcp-activity.json every
// 5 s while mounted (the dev server mutates the feed as agents call MCP
// tools) and degrades to a hint when the feed is absent. Formerly
// AgentActivity.vue — the SFC moved to TSX + a side SCSS file (family
// form); logic and rendered tree unchanged. The scoped style block is
// reproduced nested under the component's own root class so the rules
// still apply only inside `.agent-activity` (scoped-data-v semantics).
import { defineComponent, onBeforeUnmount, onMounted, ref } from "vue";
import "./AgentActivity.scss";

interface ActivityEntry {
  ts: string;
  tool: string;
  ok: boolean;
  detail: string;
}

export default defineComponent({
  name: "AgentActivity",
  setup() {
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

    return () => (
      <div class="agent-activity">
        <h2>Agent Activity</h2>
        {!available.value ? (
          <p class="hint"> No activity feed at <code>/mcp-activity.json</code> — start the dev server with <code>lagrange dev --mcp --port N</code> and open this console through it. </p>
        ) : entries.value.length === 0 ? (
          <p class="hint"> No tool calls recorded yet. Call any MCP tool (<code>docs.search</code>, <code>docs.write</code>, …) from an agent. </p>
        ) : (
          <table class="activity-table">
            <thead>
              <tr><th>Time (UTC)</th><th>Tool</th><th>Result</th><th>Detail</th></tr>
            </thead>
            <tbody>
              {entries.value.map((e, i) => (
                <tr key={`${e.ts}-${i}`}>
                  <td class="mono">{e.ts}</td>
                  <td><code>{e.tool}</code></td>
                  <td><span class={e.ok ? "ok" : "fail"}>{e.ok ? "ok" : "error"}</span></td>
                  <td class="detail">{e.detail}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    );
  },
});
