// Tiny mount helper for the docs-shell tests: creates an app around the
// component, mounts it into a detached container, and returns the root
// for assertions. Teleported dialogs land on document.body (happy-dom),
// which the tests query directly.
import { createApp, h, type Component } from "vue";

export async function mount(
  component: Component,
  props: Record<string, unknown>,
): Promise<{ root: HTMLElement; unmount: () => void }> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const app = createApp({ render: () => h(component as never, props) });
  app.mount(host);
  return {
    root: host,
    unmount: () => {
      app.unmount();
      host.remove();
    },
  };
}

// Re-export h for the render callback above (keeps the import surface of
// tests free of vue internals).
export { h };
