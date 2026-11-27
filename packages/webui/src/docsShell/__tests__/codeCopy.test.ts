// @vitest-environment happy-dom
// Code-block copy buttons (PR #119 round 8 S1): the webui lane replaces
// the flat pages that carried the legacy copy runtime, so the hydration
// entry must re-implement it. Without a handler every copy button on the
// site is inert — measured in Chrome, invisible to every other gate.
// MUT: M40 (dropping installCodeCopy from boot() turns the boot test RED).
import { afterEach, describe, expect, it, vi } from "vitest";
import { copyText, installCodeCopy } from "../docsMain";

function fixture(): HTMLElement {
  document.body.innerHTML = `
    <div class="hi-code-highlight">
      <div class="hi-code-highlight-header">
        <span class="hi-code-highlight-language">bash</span>
        <button class="hi-code-highlight-copy" data-copy="cargo run --release">copy</button>
      </div>
      <pre><code>cargo run --release</code></pre>
    </div>`;
  return document.querySelector<HTMLElement>("button.hi-code-highlight-copy")!;
}

let writes: string[] = [];

afterEach(() => {
  document.body.innerHTML = "";
  writes = [];
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the library code card's copy button", () => {
  /** The card markup the Rust renderer now emits (`hk-code-highlight*`):
   *  the button carries NO payload — the code text beside it is the target. */
  function cardFixture(): HTMLElement {
    document.body.innerHTML = `
      <div class="hk-code-highlight">
        <div class="hk-code-highlight-header">
          <div class="hk-code-highlight-label">rust</div>
          <div class="hk-code-highlight-actions">
            <button aria-label="Copy" title="Copy" class="hk-code-highlight-copy">Copy</button>
          </div>
        </div>
        <div class="hk-code-highlight-content">
          <pre class="hk-code-highlight-code"><code class="language-rust"><span class="k">fn</span> main() {}</code></pre>
        </div>
      </div>`;
    return document.querySelector<HTMLElement>("button.hk-code-highlight-copy")!;
  }

  /// MUT: dropping the `.hk-code-highlight-copy` arm from installCodeCopy
  /// leaves every fence copy button inert — this is the only gate on it.
  it("copies the card's own code text, not a payload attribute", async () => {
    const button = cardFixture();
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async (t: string) => { writes.push(t); } },
    });
    installCodeCopy();
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 10));
    // textContent — the highlighted spans must not leak into the clipboard.
    expect(writes).toEqual(["fn main() {}"]);
    expect(button.classList.contains("copied")).toBe(true);
  });

  it("still honours the legacy payload button", async () => {
    const button = fixture();
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async (t: string) => { writes.push(t); } },
    });
    installCodeCopy();
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 10));
    expect(writes).toEqual(["cargo run --release"]);
  });
});

describe("copyText", () => {
  it("uses navigator.clipboard when present", async () => {
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async (t: string) => { writes.push(t); } },
    });
    expect(await copyText("hello")).toBe(true);
    expect(writes).toEqual(["hello"]);
  });

  it("falls back to execCommand when the clipboard API is missing", async () => {
    vi.stubGlobal("navigator", {});
    const exec = vi.fn(() => true);
    (document as unknown as { execCommand: typeof exec }).execCommand = exec;
    expect(await copyText("hi")).toBe(true);
    expect(exec).toHaveBeenCalledWith("copy");
    expect(document.querySelector("textarea"), "the carrier textarea is removed").toBeNull();
  });

  it("reports failure when neither path works", async () => {
    vi.stubGlobal("navigator", {});
    (document as unknown as { execCommand: () => boolean }).execCommand = () => {
      throw new Error("blocked");
    };
    expect(await copyText("hi")).toBe(false);
  });
});

describe("installCodeCopy", () => {
  it("copies the button's data-copy payload on click", async () => {
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async (t: string) => { writes.push(t); } },
    });
    installCodeCopy();
    const button = fixture();
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 10));
    expect(writes).toEqual(["cargo run --release"]);
  });

  it("paints the copied state only on success, then clears it", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async (t: string) => { writes.push(t); } },
    });
    installCodeCopy();
    const button = fixture();
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await Promise.resolve();
    await Promise.resolve();
    expect(button.classList.contains("copied")).toBe(true);
    vi.advanceTimersByTime(1600);
    expect(button.classList.contains("copied")).toBe(false);
    vi.useRealTimers();
  });

  it("does NOT claim success when the copy failed", async () => {
    vi.stubGlobal("navigator", {});
    (document as unknown as { execCommand: () => boolean }).execCommand = () => false;
    installCodeCopy();
    const button = fixture();
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 10));
    expect(button.classList.contains("copied")).toBe(false);
  });

  it("ignores clicks that are not on a copy button", async () => {
    vi.stubGlobal("navigator", {
      clipboard: { writeText: async (t: string) => { writes.push(t); } },
    });
    installCodeCopy();
    fixture();
    document.querySelector("code")!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await new Promise((r) => setTimeout(r, 10));
    expect(writes).toEqual([]);
  });
});
