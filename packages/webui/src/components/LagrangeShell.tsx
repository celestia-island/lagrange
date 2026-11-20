import { defineComponent, h, type PropType } from "vue";

export default defineComponent({
  name: "LagrangeShell",
  props: {
    navItems: {
      type: Array as PropType<
        Array<{ key: string; label: string; children?: Array<{ key: string; label: string }> }>
      >,
      required: true,
    },
  },
  setup(props, { slots }) {
    return () =>
      h("div", { class: "lagrange-shell" }, [
        h("nav", { class: "lagrange-sidebar" }, [
          ...props.navItems.map((item) =>
            h("div", { key: item.key, class: "lagrange-nav-group" }, [
              h("div", { class: "lagrange-nav-label" }, item.label),
              ...(item.children ?? []).map((child) =>
                h("div", { key: child.key, class: "lagrange-nav-item" }, child.label),
              ),
            ]),
          ),
        ]),
        h("main", { class: "lagrange-content" }, slots.default?.()),
      ]);
  },
});
