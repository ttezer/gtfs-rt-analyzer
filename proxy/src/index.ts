/**
 * GTFS-RT CORS proxy — Cloudflare Worker.
 *
 * ## Neden var
 *
 * Birçok public GTFS-RT feed'i `Access-Control-Allow-Origin` göndermez. Tarayıcıdan
 * doğrudan `fetch` o durumda başarısız olur; bu bir feed hatası DEĞİLDİR ve validator
 * bulgusu olarak raporlanmamalıdır — yalnızca tarayıcının erişemediği anlamına gelir.
 *
 * Bu Worker o boşluğu kapatır: baytları sunucu tarafında çeker ve CORS başlıklarıyla
 * geri verir. **Veriyi yorumlamaz.** Protobuf çözümlemesi, doğrulama ve raporlama
 * tarayıcıdaki Rust/WASM katmanında kalır. Bu ayrım kasıtlı: şema ya da kural
 * değiştiğinde proxy'nin yeniden dağıtılması gerekmez ve kullanıcının verisi
 * sunucuda işlenmez.
 *
 * ## Ne değildir
 *
 * Açık (open) proxy değildir. Yalnızca [`allowlist`]'teki adreslere gider ve yalnızca
 * yapılandırılmış origin'lere yanıt verir. Feed'i **saklamaz**: `Cache-Control: no-store`
 * ile döner, disk ya da KV'ye hiçbir şey yazılmaz.
 */

import { catalogue, feedById, feedByUrl, type FeedEntry } from './allowlist';
import { fetchFeed, type FetchOptions } from './fetcher';
import { isAllowedOrigin, parseAllowedOrigins, rejectTarget } from './guards';
import { SlidingWindow } from './ratelimit';

export interface Env {
  /** Virgülle ayrılmış izinli origin listesi (GitHub Pages origin'i). */
  ALLOWED_ORIGINS?: string;
  UPSTREAM_TIMEOUT_MS?: string;
  MAX_RESPONSE_BYTES?: string;
  RATE_LIMIT_REQUESTS?: string;
  RATE_LIMIT_WINDOW_MS?: string;
}

const DEFAULTS = {
  timeoutMs: 10_000,
  /** 32 MiB — ölçülen en büyük public RT feed'lerinin bir mertebe üstü. */
  maxBytes: 32 * 1024 * 1024,
  rateLimit: 60,
  rateWindowMs: 60_000,
  maxRedirects: 2,
} as const;

/** İsolate ömrü boyunca yaşayan sayaç; sınırı `ratelimit.ts` başında yazılı. */
let limiter: SlidingWindow | null = null;
let limiterConfig = '';

function limiterFor(limit: number, windowMs: number): SlidingWindow {
  const key = `${limit}/${windowMs}`;
  if (limiter === null || limiterConfig !== key) {
    limiter = new SlidingWindow(limit, windowMs);
    limiterConfig = key;
  }
  return limiter;
}

function intFromEnv(raw: string | undefined, fallback: number): number {
  if (raw === undefined) return fallback;
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

/** Yanıtlarda ortak duran başlıklar. */
function corsHeaders(origin: string | null, allowed: readonly string[]): Headers {
  const headers = new Headers();
  // `Vary: Origin` şart: yanıt origin'e göre değişiyor, ara katmanlar bunu bilmeli.
  headers.set('Vary', 'Origin');
  headers.set('Access-Control-Allow-Methods', 'GET, OPTIONS');
  headers.set('Access-Control-Max-Age', '86400');
  if (isAllowedOrigin(origin, allowed)) {
    headers.set('Access-Control-Allow-Origin', origin as string);
    // Tarayıcıdaki JS bu başlıkları ancak açıkça expose edilirse okuyabilir;
    // UI kaynak/durum göstergesini bunlardan besliyor.
    headers.set(
      'Access-Control-Expose-Headers',
      'X-Proxy-Upstream-Status, X-Proxy-Elapsed-Ms, X-Proxy-Feed-Id, X-Proxy-Error',
    );
  }
  return headers;
}

interface ErrorBody {
  error: string;
  detail?: string;
}

function errorResponse(
  status: number,
  body: ErrorBody,
  headers: Headers,
): Response {
  headers.set('Content-Type', 'application/json; charset=utf-8');
  headers.set('Cache-Control', 'no-store');
  // UI durum ayrımını başlıktan da okuyabilsin (gövdeyi okumadan).
  headers.set('X-Proxy-Error', body.error);
  return new Response(JSON.stringify(body), { status, headers });
}

/** İstemcinin istediği feed'i çözer. */
function resolveFeed(url: URL): { feed: FeedEntry } | { error: ErrorBody; status: number } {
  const id = url.searchParams.get('feed');
  const raw = url.searchParams.get('url');

  if (id !== null) {
    const feed = feedById(id);
    if (feed === undefined) {
      return { status: 403, error: { error: 'feed_not_allowed', detail: id } };
    }
    return { feed };
  }

  if (raw !== null) {
    // Adres modunda esneklik YOK: birebir eşleşmeyen her şey reddedilir.
    const feed = feedByUrl(raw);
    if (feed === undefined) {
      return { status: 403, error: { error: 'feed_not_allowed', detail: 'url not in allowlist' } };
    }
    return { feed };
  }

  return { status: 400, error: { error: 'missing_feed_parameter' } };
}

/**
 * İstek işleyicisi.
 *
 * `fetchImpl` enjekte edilebilir: testler upstream'i taklit eder, böylece güvenlik
 * kapıları gerçek ağa çıkmadan sınanır. (`gtfs-analyzer` dersi: kapıyı yazınca
 * koruduğu şeyi bozup kırmızı görmeden yeşile güvenme.)
 */
export async function handleRequest(
  request: Request,
  env: Env,
  fetchImpl: typeof fetch = fetch,
): Promise<Response> {
  const allowed = parseAllowedOrigins(env.ALLOWED_ORIGINS);
  const origin = request.headers.get('Origin');
  const headers = corsHeaders(origin, allowed);
  const url = new URL(request.url);

  if (request.method === 'OPTIONS') {
    return new Response(null, { status: 204, headers });
  }

  if (request.method !== 'GET') {
    headers.set('Allow', 'GET, OPTIONS');
    return errorResponse(405, { error: 'method_not_allowed' }, headers);
  }

  // Katalog: UI'ın hangi feed'leri seçebileceğini öğrenmesi için.
  if (url.pathname === '/feeds') {
    headers.set('Content-Type', 'application/json; charset=utf-8');
    headers.set('Cache-Control', 'no-store');
    return new Response(JSON.stringify({ feeds: catalogue() }), { status: 200, headers });
  }

  if (url.pathname !== '/fetch') {
    return errorResponse(404, { error: 'not_found' }, headers);
  }

  const resolved = resolveFeed(url);
  if ('error' in resolved) {
    return errorResponse(resolved.status, resolved.error, headers);
  }
  const { feed } = resolved;

  // İkinci kat: allowlist'teki adres bile denetimden geçer. Allowlist bir gün
  // yanlışlıkla genişletilirse SSRF yüzeyi tek satırla açılmasın.
  const rejection = rejectTarget(feed.url);
  if (rejection !== null) {
    return errorResponse(403, { error: 'target_rejected', detail: rejection }, headers);
  }

  const limit = intFromEnv(env.RATE_LIMIT_REQUESTS, DEFAULTS.rateLimit);
  const windowMs = intFromEnv(env.RATE_LIMIT_WINDOW_MS, DEFAULTS.rateWindowMs);
  const clientKey = request.headers.get('CF-Connecting-IP') ?? origin ?? 'anonymous';
  if (!limiterFor(limit, windowMs).allow(clientKey, Date.now())) {
    headers.set('Retry-After', String(Math.ceil(windowMs / 1000)));
    return errorResponse(429, { error: 'rate_limited' }, headers);
  }

  const options: FetchOptions = {
    timeoutMs: intFromEnv(env.UPSTREAM_TIMEOUT_MS, DEFAULTS.timeoutMs),
    maxBytes: intFromEnv(env.MAX_RESPONSE_BYTES, DEFAULTS.maxBytes),
    maxRedirects: DEFAULTS.maxRedirects,
  };

  const result = await fetchFeed(feed.url, options, fetchImpl);

  if ('kind' in result) {
    switch (result.kind) {
      case 'timeout':
        return errorResponse(504, { error: 'upstream_timeout', detail: `${result.ms}ms` }, headers);
      case 'too_large':
        return errorResponse(
          413,
          { error: 'upstream_too_large', detail: `${result.limit} bytes` },
          headers,
        );
      case 'upstream_status':
        return errorResponse(
          502,
          { error: 'upstream_status', detail: String(result.status) },
          headers,
        );
      case 'redirect_blocked':
        return errorResponse(
          502,
          { error: 'upstream_redirect_blocked', detail: result.location ?? 'no location' },
          headers,
        );
      case 'network':
        return errorResponse(502, { error: 'upstream_unreachable', detail: result.detail }, headers);
    }
  }

  headers.set('Content-Type', 'application/x-protobuf');
  // Saklamıyoruz: ne Worker'da, ne ara katmanda, ne tarayıcıda. Gerçek zamanlı
  // veride bayat bir kopya, yokluğundan daha zararlıdır.
  headers.set('Cache-Control', 'no-store');
  headers.set('X-Proxy-Upstream-Status', String(result.upstreamStatus));
  headers.set('X-Proxy-Elapsed-Ms', String(result.elapsedMs));
  headers.set('X-Proxy-Feed-Id', feed.id);

  return new Response(result.bytes, { status: 200, headers });
}

export default {
  fetch(request: Request, env: Env): Promise<Response> {
    return handleRequest(request, env);
  },
};
