// Live preview of a built-site page: fetches `path` (watch, immediate),
// reports idle/loading/ok/unavailable, and renders the fetched HTML in a
// sandboxed iframe via srcdoc. Formerly PreviewPane.vue — the SFC moved
// to TSX + a side SCSS file (family form); props contract (`path`) and
// rendered tree unchanged. The scoped style block is reproduced nested
// under the component's own root class `.preview-pane` (scoped-data-v
// semantics).
import { createCommentVNode, defineComponent, ref, watch, type PropType } from "vue";
import "./PreviewPane.scss";

export default defineComponent({
  name: "PreviewPane",
  props: {
    path: { type: String as PropType<string>, required: true },
  },
  setup(props) {
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

    return () => (
      <div class="preview-pane">
        <div class="preview-head">
          <span class="preview-title">{props.path || "No page selected"}</span>
          <span class="preview-status" data-status={status.value}>{status.value}</span>
        </div>
        {status.value === "unavailable" ? (
          <p class="hint"> This page is not reachable from here. Preview works when the console is served by <code>lagrange dev --port N</code> (which serves the built site, including these pages) or from the deployed site itself. </p>
        ) : (
          // Template v-if's false-branch anchor — see App.tsx.
          createCommentVNode("v-if")
        )}
        {status.value === "ok" ? (
          <iframe
            class="preview-frame"
            srcdoc={html.value}
            sandbox="allow-same-origin"
            title="Document preview"
          />
        ) : (
          // Template v-if's false-branch anchor — see App.tsx.
          createCommentVNode("v-if")
        )}
      </div>
    );
  },
});
