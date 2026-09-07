# proxy

A deliberately restricted CORS proxy for GTFS-Realtime feeds, deployed as a Cloudflare Worker.

## Why it exists

Many public GTFS-RT feeds do not send `Access-Control-Allow-Origin`. A browser `fetch`
against them fails in every browser, and **this is not a feed defect** — it must never be
reported as a validation finding. It only means the browser cannot reach the bytes.

This Worker closes that gap: it fetches the bytes server-side and returns them with CORS
headers. The browser then parses and validates them.

```
GitHub Pages UI
  → direct fetch (preferred)
  → this proxy, only when CORS blocks the direct attempt
  → GTFS-RT feed
  → protobuf bytes
  → Rust/WASM parser in the browser
  → validation and report
```

## What it deliberately does not do

**It does not interpret the data.** Protobuf decoding, validation and reporting all stay in
the browser. This keeps the proxy indifferent to schema and rule changes — neither requires
a redeploy — and means the analysis never runs on a server.

**It does not store anything.** Responses carry `Cache-Control: no-store`, and
`wrangler.toml` binds no KV, R2, D1 or Durable Object. Adding one would break that promise
and should be reviewed as a change of contract.

**It is not an open proxy.** It only fetches addresses listed in `src/allowlist.ts`. Clients
ask for a feed by id (`?feed=<id>`); the URL is resolved server-side, so nothing the client
sends determines the destination. `?url=` is also accepted, but only when it matches an
allow-listed address exactly — no normalisation, because every leniency is a bypass surface.

## API

| Route | Purpose |
|---|---|
| `GET /feeds` | The catalogue, so the UI knows what it may request |
| `GET /fetch?feed=<id>` | Feed bytes as `application/x-protobuf` |
| `OPTIONS *` | Preflight |

Response headers on success: `Access-Control-Allow-Origin` (the configured origin only),
`Access-Control-Allow-Methods: GET, OPTIONS`, `Content-Type: application/x-protobuf`,
`Vary: Origin`, `Cache-Control: no-store`, plus `X-Proxy-Upstream-Status`,
`X-Proxy-Elapsed-Ms` and `X-Proxy-Feed-Id`. These are listed in
`Access-Control-Expose-Headers` so the UI can actually read them.

Failures return JSON and set `X-Proxy-Error`, so the UI can distinguish the cases it must
show the user apart:

| Status | `X-Proxy-Error` | Meaning |
|---|---|---|
| 400 | `missing_feed_parameter` | No feed requested |
| 403 | `feed_not_allowed` | Not in the allowlist |
| 403 | `target_rejected` | Address failed the second-layer checks |
| 405 | `method_not_allowed` | Only GET and OPTIONS |
| 413 | `upstream_too_large` | Response exceeded the cap |
| 429 | `rate_limited` | Window full; `Retry-After` is set |
| 502 | `upstream_status` / `upstream_unreachable` / `upstream_redirect_blocked` | Upstream problem |
| 504 | `upstream_timeout` | Upstream too slow |

## Safety boundaries

- **HTTPS only**, and credentials embedded in a URL are rejected.
- **Private and loopback addresses are blocked**, including the forms that URL parsing
  hides. Measured: WHATWG URL normalises `::ffff:127.0.0.1` to `::ffff:7f00:1` and
  `0177.0.0.1`, `127.1`, `2130706433`, `0x7f000001` all to `127.0.0.1` — so the check
  parses IPv6 into bytes rather than pattern-matching the text.
- **Redirects are handled manually** and only followed within the same origin. Automatic
  following would let an allow-listed address hand us an address outside the allowlist.
- **Response size is capped while reading**, not from `Content-Length`, which may be absent
  or untrue.
- **Timeout** via `AbortController`.

Rate limiting is per-isolate, not global — see the note at the top of `src/ratelimit.ts`.
It exists to stop a looping tab from hammering upstream, not to stop an attacker; the
allowlist is what limits the blast radius.

## Configuration

`wrangler.toml` `[vars]`: `ALLOWED_ORIGINS` (comma-separated), `UPSTREAM_TIMEOUT_MS`,
`MAX_RESPONSE_BYTES`, `RATE_LIMIT_REQUESTS`, `RATE_LIMIT_WINDOW_MS`.

## Development

```
npm install
npm test
npm run typecheck
npm run dev
```

The security gates are mutation-checked: disabling the IPv6 parser, the allowlist, the
origin check, the size cap or the redirect origin check each turns a test red.
