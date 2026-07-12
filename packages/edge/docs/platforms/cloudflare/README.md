# Cloudflare Workers binding for lagrange-edge

This is a reference binding that turns the runtime-neutral
[`lagrange_edge::handle`] function into a Cloudflare Worker. It is **not** a
workspace member (it targets `wasm32-unknown-unknown` and needs `wrangler`),
so it lives here as a copy-paste starting point rather than something `cargo
build` compiles in the default workspace.

## Layout

```
cloudflare/
├── Cargo.toml      # the worker crate (NOT a workspace member)
├── wrangler.toml   # deploy config — binds a D1 database
└── src/lib.rs      # ~40 lines: worker fetch → lagrange_edge::handle
```

## Deploy steps

1. `npm create cloudflare@latest lagrange-comments -- --type=rust-module`
2. Copy `src/lib.rs` from here.
3. Create a D1 database: `wrangler d1 create lagrange-comments`, paste the id
   into `wrangler.toml`.
4. Apply the schema (same SQL as `lagrange-adapter/src/storage/sqlite.rs`):
   `wrangler d1 execute lagrange-comments --file=schema.sql`
5. `wrangler deploy`.
6. Point your lagrange site at the worker:
   ```toml
   [comments]
   enabled = true
   mode = "faas"
   endpoint = "https://lagrange-comments.<your-subdomain>.workers.dev"
   auth = ["anonymous"]
   ```

## Why not worker-rs in the workspace?

`worker` (the `worker-rs` crate) pins `wasm32-unknown-unknown` and a `wasm-bindgen` toolchain that
conflicts with the host-targeting crates in this workspace (`rusqlite`,
`axum`, …). Keeping the binding here as a standalone crate means the workspace
stays host-buildable, and the binding inherits all protocol logic from
`lagrange-edge` + `lagrange-adapter` unchanged.

## Other platforms

The same `handle(...)` signature maps onto:
- **Vercel Edge Functions** — translate the `Request`/`Response` web standard
  types into the `(method, path, query, headers, body)` tuple.
- **Firebase Functions** (2nd gen) — same, with the Callable HTTPS signature.
- **Deno Deploy** — `Deno.serve` handler → same tuple.
