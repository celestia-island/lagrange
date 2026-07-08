/* @lagrange-comments ── framework-free comment mount point
 *
 * This is the P1 skeleton: it wires the <lagrange-comments> custom element,
 * reads its data-* attributes, and renders a minimal, theme-aware shell.
 * Behaviour by data-mode:
 *
 *   static-json  → fetch(data-archive) and render the archived comments
 *                  (read-only). Never touches a backend.
 *   faas / self-host → render a shell that will, in P5, call the protocol
 *                  endpoints at data-endpoint. For now it shows a polite
 *                  "loading" state and degrades to a fallback link.
 *   disqus / giscus / github-issue → NOT handled here; the site builder
 *                  emits those vendors' own scripts directly, so this file is
 *                  not loaded for them.
 *
 * Design rules (must hold through P5):
 *  - No build step, no framework. Vanilla custom-elements.
 *  - Never mutate the article DOM — this element owns only its own subtree.
 *  - Fail closed: on any error, show a small note rather than throwing.
 */
(function () {
  "use strict";

  if (customElements && customElements.get("lagrange-comments")) return;

  // Idempotently inject the component stylesheet into <head> so a single
  // <script> tag is all a page needs. The rules mirror
  // assets/lagrange-comments.css (kept as the editable source). Sites can
  // override by shipping a higher-specificity stylesheet.
  function injectStyles() {
    if (document.getElementById("lagrange-comments-css")) return;
    var style = document.createElement("style");
    style.id = "lagrange-comments-css";
    style.textContent = LAGRANGE_COMMENTS_CSS;
    (document.head || document.documentElement).appendChild(style);
  }

  var LAGRANGE_COMMENTS_CSS = [
    ".lg-comments{margin-top:3rem;padding-top:1.5rem;border-top:1px solid var(--border,#e2e2ea);font-size:.92rem;color:var(--fg,inherit)}",
    ".lg-comments .lg-c-list{display:flex;flex-direction:column;gap:1rem}",
    ".lg-comments .lg-c-item{padding:.75rem .9rem;background:var(--bg-subtle,#f7f7fa);border:1px solid var(--border,#e2e2ea);border-radius:var(--radius,6px)}",
    ".lg-comments .lg-c-head{display:flex;align-items:center;gap:.5rem;margin-bottom:.4rem;font-size:.82rem}",
    ".lg-comments .lg-c-avatar{width:1.5rem;height:1.5rem;border-radius:50%;object-fit:cover}",
    ".lg-comments .lg-c-name{font-weight:600}",
    ".lg-comments .lg-c-anon{color:var(--fg-sec,#5a5a6a);font-style:italic}",
    ".lg-comments .lg-c-time{margin-left:auto;color:var(--fg-sec,#5a5a6a);font-size:.75rem}",
    ".lg-comments .lg-c-body p{margin:.3rem 0;line-height:1.55}",
    '.lg-comments .lg-c-body code{font-family:"SFMono-Regular",Consolas,"Liberation Mono",Menlo,monospace;font-size:.88em;background:var(--code-bg,#f2f2f5);padding:.1em .35em;border-radius:3px}',
    ".lg-comments .lg-c-empty,.lg-comments .lg-c-note{color:var(--fg-sec,#5a5a6a);font-style:italic;padding:.5rem 0}",
    ".lg-comments .lg-c-error{color:#c0392b;font-style:normal}",
    ".lg-comments .lg-c-foot{margin-top:.75rem;font-size:.75rem;color:var(--fg-sec,#5a5a6a)}",
    ".lg-comments .lg-c-readonly,.lg-comments .lg-c-backend{opacity:.8}",
  ].join("\n");

  try {
    injectStyles();
  } catch (e) {
    /* headless / odd environments — styles are non-critical */
  }


  function esc(s) {
    return String(s == null ? "" : s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  // Minimal, safe markdown → HTML for archived comments. Intentionally tiny:
  // paragraphs, line breaks, inline code, bold/italic, links. Anything fancier
  // is stripped. This is the only place user markdown becomes HTML client-side,
  // so it stays conservative.
  function safeMarkdown(md) {
    var lines = String(md || "").replace(/\r\n?/g, "\n").split("\n");
    var out = [];
    var para = [];
    function flush() {
      if (para.length) {
        out.push("<p>" + inline(para.join(" ")) + "</p>");
        para = [];
      }
    }
    function inline(t) {
      // escape first, then re-introduce a controlled subset
      t = esc(t);
      t = t.replace(/`([^`]+)`/g, "<code>$1</code>");
      t = t.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
      t = t.replace(/\*([^*]+)\*/g, "<em>$1</em>");
      t = t.replace(/\[([^\]]+)\]\((https?:[^)]+)\)/g, function (_, label, url) {
        return '<a href="' + url + '" rel="nofollow noopener ugc">' + label + "</a>";
      });
      return t;
    }
    for (var i = 0; i < lines.length; i++) {
      var l = lines[i];
      if (l.trim() === "") {
        flush();
      } else {
        para.push(l);
      }
    }
    flush();
    return out.join("");
  }

  function authorHtml(a) {
    if (!a) return "<span class=\"lg-c-anon\">anonymous</span>";
    var name = esc(a.name || a.display_name || "anonymous");
    if (a.avatar || a.avatar_url) {
      return (
        '<img class="lg-c-avatar" src="' +
        esc(a.avatar || a.avatar_url) +
        '" alt="" loading="lazy"> <span class="lg-c-name">' +
        name +
        "</span>"
      );
    }
    return '<span class="lg-c-name">' + name + "</span>";
  }

  function commentHtml(c) {
    var body = c.body_html
      ? c.body_html // trusted server-rendered HTML
      : safeMarkdown(c.body_markdown || c.body || "");
    var when = c.created_at
      ? '<time class="lg-c-time" datetime="' + esc(c.created_at) + '">' + esc(c.created_at) + "</time>"
      : "";
    return (
      '<article class="lg-c-item">' +
      '<header class="lg-c-head">' +
      authorHtml(c.author) +
      when +
      "</header>" +
      '<div class="lg-c-body">' +
      body +
      "</div>" +
      "</article>"
    );
  }

  function renderComments(el, comments, opts) {
    if (!comments || !comments.length) {
      el.innerHTML =
        '<p class="lg-c-empty">' +
        (opts.emptyText || "No comments yet.") +
        "</p>";
      return;
    }
    var html =
      '<div class="lg-c-list">' +
      comments.map(commentHtml).join("") +
      "</div>";
    if (opts.footer) html += '<div class="lg-c-foot">' + opts.footer + "</div>";
    el.innerHTML = html;
  }

  function note(el, text, cls) {
    el.innerHTML =
      '<p class="lg-c-note ' +
      (cls || "") +
      '">' +
      esc(text) +
      "</p>";
  }

  var LagrangeComments = Object.create(HTMLElement.prototype);
  LagrangeComments.connectedCallback = function () {
    var self = this;
    if (self._connected) return;
    self._connected = true;

    var mode = self.getAttribute("data-mode") || "";
    var nodeId = self.getAttribute("data-node-id") || "";
    var endpoint = self.getAttribute("data-endpoint") || "";
    var archive = self.getAttribute("data-archive") || "";
    var canonical = self.getAttribute("data-canonical") || window.location.href;

    self.classList.add("lg-comments");

    if (mode === "static-json") {
      loadStaticJson(self, archive);
      return;
    }

    if (mode === "faas" || mode === "self-host") {
      // P1 skeleton: show a loading shell. P5 replaces this with a real
      // protocol client (fetch threads + comments, render composer).
      note(self, "Loading comments…", "lg-c-loading");
      fetchThreadsAndRender(self, endpoint, nodeId, canonical);
      return;
    }

    // Unknown / future mode: degrade silently.
    note(self, "Comments are not available for this page.", "lg-c-muted");
  };

  function loadStaticJson(el, archive) {
    if (!archive) {
      note(el, "No comment archive configured.", "lg-c-muted");
      return;
    }
    var xhr = new XMLHttpRequest();
    xhr.open("GET", archive, true);
    xhr.onload = function () {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          var data = JSON.parse(xhr.responseText);
          var comments = data.comments || data.items || [];
          renderComments(el, comments, {
            footer:
              '<span class="lg-c-readonly">Read-only archive · ' +
              comments.length +
              " comment(s)</span>",
          });
        } catch (e) {
          note(el, "Could not parse the comment archive.", "lg-c-error");
        }
      } else {
        // No archive file is the normal state for a brand-new page.
        renderComments(el, [], {});
      }
    };
    xhr.onerror = function () {
      renderComments(el, [], {});
    };
    xhr.send();
  }

  // P1 stub for the live backend. P5 will implement the full lagrange-comment
  // protocol client here; for now we probe GET {endpoint}/threads?node=… and
  // render whatever it returns, otherwise degrade.
  function fetchThreadsAndRender(el, endpoint, nodeId, canonical) {
    if (!endpoint) {
      note(el, "Comment endpoint is not configured.", "lg-c-error");
      return;
    }
    var url =
      endpoint.replace(/\/$/, "") +
      "/threads?node=" +
      encodeURIComponent(nodeId) +
      "&canonical=" +
      encodeURIComponent(canonical);
    var xhr = new XMLHttpRequest();
    xhr.open("GET", url, true);
    xhr.timeout = 8000;
    xhr.onload = function () {
      if (xhr.status >= 200 && xhr.status < 300) {
        try {
          var data = JSON.parse(xhr.responseText);
          var comments = data.comments || (data.thread && data.thread.comments) || [];
          renderComments(el, comments, {
            footer:
              '<span class="lg-c-backend">via ' +
              esc(endpoint) +
              "</span>",
          });
        } catch (e) {
          note(el, "The comment backend returned unexpected data.", "lg-c-error");
        }
      } else {
        note(
          el,
          "The comment backend did not respond (HTTP " + xhr.status + ").",
          "lg-c-error"
        );
      }
    };
    xhr.ontimeout = xhr.onerror = function () {
      note(
        el,
        "Could not reach the comment backend. Please try again later.",
        "lg-c-error"
      );
    };
    xhr.send();
  }

  try {
    customElements.define("lagrange-comments", LagrangeComments);
  } catch (e) {
    // Some embeddable contexts (older browsers) lack customElements; fail quiet.
    if (typeof console !== "undefined" && console.warn)
      console.warn("lagrange-comments: customElements unavailable", e);
  }
})();
