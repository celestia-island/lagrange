// The docs page root: the hikari shell around the page body. Mounted by
// both lanes — scripts/ssr-docs.ts (build-time SSR, the no-JS baseline)
// and docsMain.ts (browser hydration over that HTML).
import { defineComponent, type PropType } from "vue";
import type { DocsPageData } from "./types";
import { LagrangeDocsShell } from "./LagrangeDocsShell";

export const DocsPage = defineComponent({
  name: "LagrangeDocsPage",
  props: {
    page: { type: Object as PropType<DocsPageData>, required: true },
  },
  setup(props) {
    return () => <LagrangeDocsShell page={props.page} />;
  },
});
