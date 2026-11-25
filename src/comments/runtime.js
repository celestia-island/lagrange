/* GENERATED FROM src/comments/runtime.ts — do not edit.
 * Rebuild: node tools/scripts/build-embedded-ts.ts (or `just embedded`).
 * The .js beside each source is build output; the pairing is locked by
 * cargo test --test embedded_ts. */
(() => {
  (function() {
    "use strict";
    if (customElements && customElements.get("lagrange-comments")) return;
    var CSS = [
      ".lg-comments{margin-top:3rem;padding-top:1.5rem;border-top:1px solid var(--border,#e2e2ea);font-size:.92rem;color:var(--fg,inherit)}",
      ".lg-comments *{box-sizing:border-box}",
      ".lg-c-head-bar{display:flex;align-items:center;justify-content:space-between;margin-bottom:1rem;gap:.5rem;flex-wrap:wrap}",
      ".lg-c-count{font-weight:600}",
      ".lg-c-auth{display:flex;gap:.4rem;align-items:center}",
      ".lg-c-btn{padding:.35rem .7rem;border:1px solid var(--border,#e2e2ea);border-radius:var(--radius,6px);background:var(--bg,inherit);color:var(--fg,inherit);cursor:pointer;font-size:.82rem;transition:all .15s}",
      ".lg-c-btn:hover{border-color:var(--accent,#3a6ea5)}",
      ".lg-c-btn:disabled{opacity:.5;cursor:default}",
      ".lg-c-btn.primary{background:var(--accent,#3a6ea5);color:#fff;border-color:var(--accent,#3a6ea5)}",
      ".lg-c-list{display:flex;flex-direction:column;gap:.75rem}",
      ".lg-c-item{padding:.75rem .9rem;background:var(--bg-subtle,#f7f7fa);border:1px solid var(--border,#e2e2ea);border-radius:var(--radius,6px)}",
      ".lg-c-item.reply{margin-left:1.75rem;border-left:3px solid var(--accent,#3a6ea5)}",
      ".lg-c-row{display:flex;align-items:center;gap:.5rem;margin-bottom:.4rem;font-size:.82rem;flex-wrap:wrap}",
      ".lg-c-avatar{width:1.5rem;height:1.5rem;border-radius:50%;object-fit:cover}",
      ".lg-c-name{font-weight:600}",
      ".lg-c-anon{color:var(--fg-sec,#5a5a6a);font-style:italic}",
      ".lg-c-time{color:var(--fg-sec,#5a5a6a);font-size:.75rem}",
      ".lg-c-mod{font-size:.65rem;background:var(--accent,#3a6ea5);color:#fff;padding:.05rem .3rem;border-radius:3px;text-transform:uppercase}",
      ".lg-c-body p{margin:.3rem 0;line-height:1.55}",
      ".lg-c-body code{font-family:monospace;font-size:.88em;background:var(--code-bg,#f2f2f5);padding:.1em .35em;border-radius:3px}",
      ".lg-c-actions{display:flex;gap:.3rem;margin-top:.4rem;font-size:.75rem}",
      ".lg-c-actions button{background:none;border:none;color:var(--fg-sec,#5a5a6a);cursor:pointer;padding:0;font-size:.75rem;text-decoration:underline}",
      ".lg-c-actions button:hover{color:var(--accent,#3a6ea5)}",
      ".lg-c-vote{display:flex;align-items:center;gap:.15rem;color:var(--fg-sec,#5a5a6a)}",
      ".lg-c-compose{margin:1rem 0;padding:.75rem;border:1px solid var(--border,#e2e2ea);border-radius:var(--radius,6px);background:var(--bg,inherit)}",
      ".lg-c-compose textarea{width:100%;min-height:80px;padding:.5rem;border:1px solid var(--border,#e2e2ea);border-radius:var(--radius,6px);font-family:inherit;font-size:.9rem;background:var(--bg,inherit);color:var(--fg,inherit);resize:vertical}",
      ".lg-c-compose textarea:focus{outline:none;border-color:var(--accent,#3a6ea5)}",
      ".lg-c-compose-row{display:flex;justify-content:space-between;align-items:center;margin-top:.4rem;gap:.5rem}",
      ".lg-c-hint{font-size:.72rem;color:var(--fg-sec,#5a5a6a)}",
      ".lg-c-login{display:flex;gap:.4rem;flex-wrap:wrap;align-items:end}",
      ".lg-c-login input{padding:.35rem .5rem;border:1px solid var(--border,#e2e2ea);border-radius:var(--radius,6px);font-size:.82rem;background:var(--bg,inherit);color:var(--fg,inherit)}",
      ".lg-c-note,.lg-c-empty{color:var(--fg-sec,#5a5a6a);font-style:italic;padding:.5rem 0}",
      ".lg-c-error{color:#c0392b;font-style:normal}",
      ".lg-c-spinner{display:inline-block;width:14px;height:14px;border:2px solid var(--border,#e2e2ea);border-top-color:var(--accent,#3a6ea5);border-radius:50%;animation:lgspin .6s linear infinite}",
      "@keyframes lgspin{to{transform:rotate(360deg)}}"
    ].join("\n");
    function injectStyles() {
      if (document.getElementById("lagrange-comments-css")) return;
      var s = document.createElement("style");
      s.id = "lagrange-comments-css";
      s.textContent = CSS;
      (document.head || document.documentElement).appendChild(s);
    }
    function esc(s) {
      return String(s == null ? "" : s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
    }
    function safeMarkdown(md) {
      var lines = String(md || "").replace(/\r\n?/g, "\n").split("\n");
      var out = [], para = [];
      function flush() {
        if (para.length) out.push("<p>" + inline(para.join(" ")) + "</p>");
        para = [];
      }
      function inline(t) {
        t = esc(t);
        t = t.replace(/`([^`]+)`/g, "<code>$1</code>");
        t = t.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
        t = t.replace(/\*([^*]+)\*/g, "<em>$1</em>");
        t = t.replace(/\[([^\]]+)\]\((https?:[^)]+)\)/g, function(_, l, u) {
          return '<a href="' + u + '" rel="nofollow noopener ugc">' + l + "</a>";
        });
        return t;
      }
      for (var i = 0; i < lines.length; i++)
        lines[i].trim() === "" ? flush() : para.push(lines[i]);
      flush();
      return out.join("");
    }
    function timeAgo(iso) {
      if (!iso) return "";
      try {
        var d = new Date(iso);
        var s = (Date.now() - d.getTime()) / 1e3;
        if (s < 60) return "just now";
        if (s < 3600) return Math.floor(s / 60) + "m ago";
        if (s < 86400) return Math.floor(s / 3600) + "h ago";
        if (s < 2592e3) return Math.floor(s / 86400) + "d ago";
        return d.toISOString().slice(0, 10);
      } catch (e) {
        return iso;
      }
    }
    function api(endpoint, path, method, body, token) {
      return new Promise(function(resolve, reject) {
        var xhr = new XMLHttpRequest();
        xhr.open(method, endpoint.replace(/\/$/, "") + path, true);
        xhr.timeout = 1e4;
        if (body) xhr.setRequestHeader("content-type", "application/json");
        if (token) xhr.setRequestHeader("authorization", "Bearer " + token);
        xhr.onload = function() {
          var data = null;
          try {
            data = xhr.responseText ? JSON.parse(xhr.responseText) : null;
          } catch (e) {
          }
          if (xhr.status >= 200 && xhr.status < 300) resolve({ status: xhr.status, data });
          else reject({ status: xhr.status, data });
        };
        xhr.ontimeout = xhr.onerror = function() {
          reject({ status: 0, data: null });
        };
        xhr.send(body ? JSON.stringify(body) : null);
      });
    }
    var TOKEN_KEY = "lagrange-comment-token";
    function getToken() {
      try {
        return localStorage.getItem(TOKEN_KEY) || null;
      } catch (e) {
        return null;
      }
    }
    function setToken(t) {
      try {
        t ? localStorage.setItem(TOKEN_KEY, t) : localStorage.removeItem(TOKEN_KEY);
      } catch (e) {
      }
    }
    function el(tag, cls, html) {
      var e = document.createElement(tag);
      if (cls) e.className = cls;
      if (html != null) e.innerHTML = html;
      return e;
    }
    var methods = {};
    var proto = methods;
    proto.connectedCallback = function() {
      var s = this;
      if (s._init) return;
      s._init = true;
      injectStyles();
      s.classList.add("lg-comments");
      s._mode = s.getAttribute("data-mode") || "";
      s._endpoint = s.getAttribute("data-endpoint") || "";
      s._nodeId = s.getAttribute("data-node-id") || "";
      s._canonical = s.getAttribute("data-canonical") || location.href;
      s._auth = (s.getAttribute("data-auth") || "").split(",").filter(Boolean);
      s._archive = s.getAttribute("data-archive") || "";
      s._threadId = null;
      s._comments = [];
      s._nextCursor = null;
      s._me = null;
      s._replyingTo = null;
      if (s._mode === "static-json") {
        s._loadStatic();
        return;
      }
      if (s._mode === "faas" || s._mode === "self-host") {
        s._boot();
        return;
      }
      s.innerHTML = '<p class="lg-c-note">Comments unavailable.</p>';
    };
    proto._loadStatic = function() {
      var self = this;
      self.innerHTML = '<p class="lg-c-note">Loading comments…</p>';
      if (!self._archive) {
        self._renderStatic([]);
        return;
      }
      var xhr = new XMLHttpRequest();
      xhr.open("GET", self._archive, true);
      xhr.onload = function() {
        if (xhr.status >= 200 && xhr.status < 300) {
          try {
            self._renderStatic(JSON.parse(xhr.responseText).comments || []);
          } catch (e) {
            self.innerHTML = '<p class="lg-c-error">Archive parse error.</p>';
          }
        } else {
          self._renderStatic([]);
        }
      };
      xhr.onerror = function() {
        self._renderStatic([]);
      };
      xhr.send();
    };
    proto._renderStatic = function(comments) {
      var self = this;
      var html = '<div class="lg-c-head-bar"><span class="lg-c-count">' + comments.length + " comment" + (comments.length === 1 ? "" : "s") + "</span></div>";
      html += self._renderList(comments, false);
      html += '<p class="lg-c-hint">Read-only archive.</p>';
      self.innerHTML = html;
    };
    proto._boot = function() {
      var self = this;
      self.innerHTML = '<p class="lg-c-note"><span class="lg-c-spinner"></span> Loading…</p>';
      var token = getToken();
      self._loadMe(token).then(function() {
        return api(self._endpoint, "/threads?node=" + encodeURIComponent(self._nodeId), "GET", null, token);
      }).then(function(r) {
        if (r.data && r.data.status === "found" && r.data.id) {
          self._threadId = r.data.id;
        } else if (r.data && r.data.thread_id) {
          self._threadId = r.data.thread_id;
        }
        self._renderShell();
        if (self._threadId) self._loadComments();
        else self._renderEmpty();
      }).catch(function() {
        self._renderShell();
        self._renderEmpty();
      });
    };
    proto._loadMe = function(token) {
      var self = this;
      if (!token) {
        self._me = null;
        return Promise.resolve();
      }
      return api(self._endpoint, "/auth/me", "GET", null, token).then(function(r) {
        if (r.data && r.data.authenticated) {
          self._me = { author: r.data.author, moderator: !!r.data.moderator };
        } else {
          self._me = null;
          setToken(null);
        }
      }).catch(function() {
        self._me = null;
      });
    };
    proto._renderShell = function() {
      var self = this;
      self.innerHTML = "";
      var bar = el("div", "lg-c-head-bar");
      self._countEl = el("span", "lg-c-count", "comments");
      bar.appendChild(self._countEl);
      self._authEl = el("div", "lg-c-auth");
      bar.appendChild(self._authEl);
      self.appendChild(bar);
      self._renderAuth();
      self._composeWrap = el("div");
      self.appendChild(self._composeWrap);
      self._renderCompose();
      self._errEl = el("div");
      self.appendChild(self._errEl);
      self._listEl = el("div", "lg-c-list");
      self.appendChild(self._listEl);
      self._moreEl = el("div");
      self.appendChild(self._moreEl);
    };
    proto._renderAuth = function() {
      var self = this;
      self._authEl.innerHTML = "";
      if (self._me) {
        var name = el("span", "", esc(self._me.author.name));
        if (self._me.moderator) name.innerHTML += ' <span class="lg-c-mod">mod</span>';
        self._authEl.appendChild(name);
        var out = el("button", "lg-c-btn", "sign out");
        out.onclick = function() {
          setToken(null);
          self._me = null;
          self._renderAuth();
          self._renderCompose();
          self._refresh();
        };
        self._authEl.appendChild(out);
      } else if (self._auth.indexOf("local") >= 0) {
        self._authEl.appendChild(self._loginForm());
      } else if (self._auth.indexOf("anonymous") >= 0) {
        self._authEl.appendChild(el("span", "lg-c-hint", "posting as guest"));
      } else {
        self._authEl.appendChild(el("span", "lg-c-hint", "sign in to comment"));
      }
    };
    proto._loginForm = function() {
      var self = this;
      var wrap = el("div", "lg-c-login");
      var name = document.createElement("input");
      name.type = "text";
      name.placeholder = "username";
      name.className = "lg-c-name-input";
      var pw = document.createElement("input");
      pw.type = "password";
      pw.placeholder = "password";
      pw.className = "lg-c-pw-input";
      var btn = el("button", "lg-c-btn primary", "sign in");
      btn.disabled = false;
      function submit() {
        if (!name.value || !pw.value) return;
        btn.disabled = true;
        btn.textContent = "…";
        api(self._endpoint, "/auth/login", "POST", { name: name.value, password: pw.value }, null).then(function(r) {
          if (r.data && r.data.token) {
            setToken(r.data.token);
            return self._loadMe(r.data.token).then(function() {
              self._renderAuth();
              self._renderCompose();
              self._refresh();
            });
          }
        }).catch(function() {
          self._showErr("login failed");
        }).finally(function() {
          btn.disabled = false;
          btn.textContent = "sign in";
        });
      }
      btn.onclick = submit;
      pw.addEventListener("keydown", function(e) {
        if (e.key === "Enter") submit();
      });
      wrap.appendChild(name);
      wrap.appendChild(pw);
      wrap.appendChild(btn);
      return wrap;
    };
    proto._renderCompose = function() {
      var self = this;
      self._composeWrap.innerHTML = "";
      var canPost = self._me || self._auth.indexOf("anonymous") >= 0;
      if (!canPost) return;
      var box = el("div", "lg-c-compose");
      var ta = document.createElement("textarea");
      ta.placeholder = self._replyingTo ? "Reply (markdown)…" : "Write a comment (markdown)…";
      ta.className = "lg-c-ta";
      var row = el("div", "lg-c-compose-row");
      var hint = el("span", "lg-c-hint", "**bold** *italic* `code` [link](url)");
      var actions = el("div");
      if (self._replyingTo) {
        var cancel = el("button", "lg-c-btn", "cancel reply");
        cancel.onclick = function() {
          self._replyingTo = null;
          self._renderCompose();
        };
        actions.appendChild(cancel);
      }
      var post = el("button", "lg-c-btn primary", self._replyingTo ? "post reply" : "post comment");
      post.onclick = function() {
        self._postComment(ta, post);
      };
      row.appendChild(hint);
      row.appendChild(actions);
      row.appendChild(post);
      box.appendChild(ta);
      box.appendChild(row);
      self._composeWrap.appendChild(box);
      ta.focus();
    };
    proto._postComment = function(ta, btn) {
      var self = this;
      var body = ta.value.trim();
      if (!body) {
        self._showErr("comment is empty");
        return;
      }
      btn.disabled = true;
      btn.textContent = "posting…";
      var payload = { node_id: self._nodeId, canonical_url: self._canonical, body_markdown: body };
      if (self._threadId) payload.thread_id = self._threadId;
      if (self._replyingTo) payload.parent_id = self._replyingTo;
      if (!self._me && self._auth.indexOf("anonymous") >= 0) payload.author_name = "guest";
      api(self._endpoint, "/comments", "POST", payload, getToken()).then(function(r) {
        ta.value = "";
        self._replyingTo = null;
        self._renderCompose();
        if (r.data) {
          if (!self._threadId && r.data.thread_id) self._threadId = r.data.thread_id;
          self._comments.push(r.data);
          self._renderList(self._comments, true);
          self._updateCount();
          if (r.data.status === "pending")
            self._showErr("Your comment is awaiting moderation.", false);
        }
      }).catch(function(e) {
        self._showErr(self._errMsg(e, "post failed"));
      }).finally(function() {
        btn.disabled = false;
        btn.textContent = self._replyingTo ? "post reply" : "post comment";
      });
    };
    proto._loadComments = function() {
      var self = this;
      var path = "/comments?thread=" + encodeURIComponent(self._threadId) + "&limit=50";
      if (self._nextCursor) path += "&cursor=" + encodeURIComponent(self._nextCursor);
      api(self._endpoint, path, "GET", null, getToken()).then(function(r) {
        if (r.data) {
          self._comments = self._comments.concat(r.data.comments || []);
          self._nextCursor = r.data.next_cursor || null;
          self._renderList(self._comments, true);
          self._updateCount();
          self._renderMore();
        }
      }).catch(function(e) {
        self._showErr(self._errMsg(e, "load failed"));
      });
    };
    proto._refresh = function() {
      var s = this;
      s._comments = [];
      s._nextCursor = null;
      if (s._threadId) s._loadComments();
      else s._renderEmpty();
    };
    proto._renderEmpty = function() {
      var s = this;
      if (s._comments.length === 0)
        s._listEl.innerHTML = '<p class="lg-c-empty">No comments yet. Be the first.</p>';
    };
    proto._renderMore = function() {
      var self = this;
      self._moreEl.innerHTML = "";
      if (!self._nextCursor) return;
      var b = el("button", "lg-c-btn", "load more");
      b.onclick = function() {
        self._loadComments();
      };
      self._moreEl.appendChild(b);
    };
    proto._renderList = function(comments, interactive) {
      var self = this;
      var byParent = {};
      comments.forEach(function(c) {
        var p = c.parent_id || "__root";
        (byParent[p] = byParent[p] || []).push(c);
      });
      function tree(list, depth) {
        return list.map(function(c) {
          var kids = byParent[c.id] || [];
          return self._commentHtml(c, depth, interactive) + (kids.length ? tree(kids, depth + 1) : "");
        }).join("");
      }
      var html = tree(byParent.__root || [], 0);
      if (!html) html = '<p class="lg-c-empty">No comments yet.</p>';
      if (interactive && self._listEl) {
        self._listEl.innerHTML = html;
        self._wireActions();
      }
      return html;
    };
    proto._commentHtml = function(c, depth, interactive) {
      var self = this;
      var cls = "lg-c-item" + (depth > 0 ? " reply" : "");
      var a = c.author || {};
      var body = c.body_html || safeMarkdown(c.body_markdown || "");
      var avatar = a.avatar ? '<img class="lg-c-avatar" src="' + esc(a.avatar) + '" alt="">' : "";
      var name = a.name ? '<span class="lg-c-name">' + esc(a.name) + "</span>" : '<span class="lg-c-anon">anonymous</span>';
      var head = '<div class="lg-c-row">' + avatar + name + '<span class="lg-c-time" datetime="' + esc(c.created_at) + '">' + esc(timeAgo(c.created_at)) + "</span></div>";
      var actions = "";
      if (interactive) {
        actions = '<div class="lg-c-actions">';
        if (self._me || self._auth.indexOf("anonymous") >= 0)
          actions += '<button data-act="reply" data-id="' + esc(c.id) + '">reply</button>';
        var upIcon = window.lgUI && window.lgUI.icon ? window.lgUI.icon("arrow-up", 12) : "";
        actions += '<span class="lg-c-vote"><button data-act="up" data-id="' + esc(c.id) + '">' + upIcon + "</button>" + (c.votes && c.votes.up - c.votes.down || 0) + "</span>";
        var isOwner = self._me && a.id && self._me.author.id === a.id;
        if (isOwner || self._me && self._me.moderator) {
          actions += '<button data-act="edit" data-id="' + esc(c.id) + '">edit</button>';
          actions += '<button data-act="del" data-id="' + esc(c.id) + '">delete</button>';
        }
        actions += "</div>";
      }
      return '<div class="' + cls + '" data-cid="' + esc(c.id) + '">' + head + '<div class="lg-c-body">' + body + "</div>" + actions + "</div>";
    };
    proto._wireActions = function() {
      var self = this;
      self._listEl.querySelectorAll("button[data-act]").forEach(function(b) {
        b.onclick = function() {
          self._onAction(b.dataset.act, b.dataset.id);
        };
      });
    };
    proto._onAction = function(act, id) {
      var self = this;
      if (act === "reply") {
        self._replyingTo = id;
        self._renderCompose();
        return;
      }
      if (act === "up") {
        api(self._endpoint, "/comments/" + encodeURIComponent(id) + "/vote", "POST", { dir: "up" }, getToken()).then(function() {
          self._refresh();
        }).catch(function() {
          self._showErr("vote failed");
        });
        return;
      }
      if (act === "edit") {
        self._editInline(id);
        return;
      }
      if (act === "del") {
        if (!confirm("Delete this comment?")) return;
        api(self._endpoint, "/comments/" + encodeURIComponent(id), "DELETE", null, getToken()).then(function() {
          self._refresh();
        }).catch(function(e) {
          self._showErr(self._errMsg(e, "delete failed"));
        });
        return;
      }
    };
    proto._editInline = function(id) {
      var self = this;
      var node = self._listEl.querySelector('[data-cid="' + id + '"]');
      if (!node) return;
      var c = self._comments.find(function(x) {
        return x.id === id;
      });
      if (!c) return;
      var body = node.querySelector(".lg-c-body");
      var ta = document.createElement("textarea");
      ta.className = "lg-c-ta";
      ta.value = c.body_markdown || "";
      body.innerHTML = "";
      body.appendChild(ta);
      var save = el("button", "lg-c-btn primary", "save");
      var cancel = el("button", "lg-c-btn", "cancel");
      var row = el("div", "lg-c-compose-row");
      row.appendChild(cancel);
      row.appendChild(save);
      body.appendChild(row);
      save.onclick = function() {
        api(self._endpoint, "/comments/" + encodeURIComponent(id), "PATCH", { body_markdown: ta.value }, getToken()).then(function() {
          self._refresh();
        }).catch(function(e) {
          self._showErr(self._errMsg(e, "edit failed"));
        });
      };
      cancel.onclick = function() {
        self._refresh();
      };
    };
    proto._updateCount = function() {
      var s = this;
      var n = s._comments.length;
      if (s._countEl) s._countEl.textContent = n + " comment" + (n === 1 ? "" : "s");
    };
    proto._showErr = function(msg, isError) {
      var s = this;
      if (s._errEl) s._errEl.innerHTML = '<p class="' + (isError === false ? "lg-c-note" : "lg-c-error lg-c-note") + '">' + esc(msg) + "</p>";
    };
    proto._errMsg = function(e, fallback) {
      if (e && e.data && e.data.message) return e.data.message;
      if (e && e.status === 0) return "network error — is the backend reachable?";
      return fallback;
    };
    var CommentsElement = class extends HTMLElement {
    };
    for (var key in methods) {
      Object.defineProperty(CommentsElement.prototype, key, Object.getOwnPropertyDescriptor(methods, key));
    }
    try {
      customElements.define("lagrange-comments", CommentsElement);
    } catch (e) {
      if (console && console.warn) console.warn("lagrange-comments: customElements unavailable", e);
    }
  })();
})();
