/**
 * Upstream'den GTFS-RT baytlarını çekme.
 *
 * Tek iş: baytları getir. Protobuf çözümlemesi ve doğrulama tarayıcıdaki
 * WASM katmanının işidir — proxy veriyi yorumlamaz, bu yüzden şema
 * değişiklikleri proxy'yi ilgilendirmez ve proxy'nin yeniden dağıtılması gerekmez.
 */

import { rejectTarget } from './guards';

export type FetchFailure =
  | { kind: 'timeout'; ms: number }
  | { kind: 'too_large'; limit: number }
  | { kind: 'upstream_status'; status: number }
  | { kind: 'redirect_blocked'; location: string | null }
  | { kind: 'network'; detail: string };

export interface FetchSuccess {
  bytes: Uint8Array;
  upstreamStatus: number;
  upstreamContentType: string | null;
  elapsedMs: number;
}

export interface FetchOptions {
  timeoutMs: number;
  maxBytes: number;
  /** Aynı origin içinde izin verilen yönlendirme sayısı. */
  maxRedirects: number;
}

/**
 * Yanıt gövdesini sınırı aşmadan okur.
 *
 * `Content-Length` başlığına güvenilmez: eksik olabilir, yalan olabilir, ya da
 * chunked kodlamada hiç gelmez. Sınır okuma sırasında sayılarak uygulanır.
 */
async function readCapped(
  response: Response,
  maxBytes: number,
): Promise<Uint8Array | 'too_large'> {
  const body = response.body;
  if (body === null) return new Uint8Array(0);

  const reader = body.getReader();
  const chunks: Uint8Array[] = [];
  let total = 0;

  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      if (value === undefined) continue;
      total += value.byteLength;
      if (total > maxBytes) {
        await reader.cancel();
        return 'too_large';
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }

  const out = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    out.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return out;
}

/**
 * Hedeften baytları çeker.
 *
 * Yönlendirmeler **elle** ele alınır (`redirect: 'manual'`): otomatik takip,
 * allowlist'te olan bir adresin bizi allowlist dışına taşımasına izin verirdi —
 * klasik SSRF atlatması. Yalnızca aynı origin içinde kalan yönlendirmeler izlenir.
 */
export async function fetchFeed(
  url: string,
  options: FetchOptions,
  fetchImpl: typeof fetch = fetch,
): Promise<FetchSuccess | FetchFailure> {
  const started = Date.now();
  let current = url;

  for (let hop = 0; hop <= options.maxRedirects; hop += 1) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), options.timeoutMs);

    let response: Response;
    try {
      response = await fetchImpl(current, {
        method: 'GET',
        redirect: 'manual',
        signal: controller.signal,
        headers: { Accept: 'application/x-protobuf, application/octet-stream, */*' },
      });
    } catch (err) {
      clearTimeout(timer);
      if (controller.signal.aborted) return { kind: 'timeout', ms: options.timeoutMs };
      return { kind: 'network', detail: err instanceof Error ? err.message : 'fetch failed' };
    }
    clearTimeout(timer);

    if (response.status >= 300 && response.status < 400) {
      const location = response.headers.get('location');
      if (location === null || hop === options.maxRedirects) {
        return { kind: 'redirect_blocked', location };
      }
      let next: string;
      try {
        next = new URL(location, current).toString();
      } catch {
        return { kind: 'redirect_blocked', location };
      }
      // Yönlendirme yalnızca aynı origin içinde izlenir ve hedef yine denetlenir.
      if (new URL(next).origin !== new URL(current).origin || rejectTarget(next) !== null) {
        return { kind: 'redirect_blocked', location: next };
      }
      current = next;
      continue;
    }

    if (!response.ok) return { kind: 'upstream_status', status: response.status };

    const bytes = await readCapped(response, options.maxBytes);
    if (bytes === 'too_large') return { kind: 'too_large', limit: options.maxBytes };

    return {
      bytes,
      upstreamStatus: response.status,
      upstreamContentType: response.headers.get('content-type'),
      elapsedMs: Date.now() - started,
    };
  }

  return { kind: 'redirect_blocked', location: null };
}
