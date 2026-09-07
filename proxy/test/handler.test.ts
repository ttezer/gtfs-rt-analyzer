import { describe, expect, it, vi } from 'vitest';
import { handleRequest, type Env } from '../src/index';

const ORIGIN = 'https://ttezer.github.io';
const ENV: Env = { ALLOWED_ORIGINS: ORIGIN, RATE_LIMIT_REQUESTS: '1000' };

const PB = new Uint8Array([0x0a, 0x05, 0x0a, 0x03, 0x32, 0x2e, 0x30]);

function req(path: string, init: RequestInit = {}): Request {
  return new Request(`https://proxy.test${path}`, {
    headers: { Origin: ORIGIN },
    ...init,
  });
}

/** Sabit bir protobuf yanıtı döndüren sahte upstream. */
function okFetch(body: Uint8Array = PB): typeof fetch {
  return vi.fn(async () =>
    new Response(body, { status: 200, headers: { 'Content-Type': 'application/x-protobuf' } }),
  ) as unknown as typeof fetch;
}

describe('CORS', () => {
  it('answers preflight with the allowed methods', async () => {
    const res = await handleRequest(req('/fetch', { method: 'OPTIONS' }), ENV, okFetch());
    expect(res.status).toBe(204);
    expect(res.headers.get('Access-Control-Allow-Origin')).toBe(ORIGIN);
    expect(res.headers.get('Access-Control-Allow-Methods')).toBe('GET, OPTIONS');
    expect(res.headers.get('Vary')).toBe('Origin');
  });

  it('withholds the allow-origin header from other origins', async () => {
    const res = await handleRequest(
      new Request('https://proxy.test/fetch?feed=mbta-alerts', {
        headers: { Origin: 'https://evil.test' },
      }),
      ENV,
      okFetch(),
    );
    expect(res.headers.get('Access-Control-Allow-Origin')).toBeNull();
    // Vary her durumda dönmeli, aksi halde ara katmanlar yanıtı yanlış paylaşır.
    expect(res.headers.get('Vary')).toBe('Origin');
  });

  it('exposes the diagnostic headers so the UI can read them', async () => {
    const res = await handleRequest(req('/fetch?feed=mbta-alerts'), ENV, okFetch());
    const exposed = res.headers.get('Access-Control-Expose-Headers') ?? '';
    for (const h of ['X-Proxy-Upstream-Status', 'X-Proxy-Elapsed-Ms', 'X-Proxy-Error']) {
      expect(exposed).toContain(h);
    }
  });
});

describe('method and routing', () => {
  it('rejects anything but GET and OPTIONS', async () => {
    for (const method of ['POST', 'PUT', 'DELETE', 'PATCH']) {
      const res = await handleRequest(req('/fetch', { method }), ENV, okFetch());
      expect(res.status, method).toBe(405);
      expect(res.headers.get('Allow')).toBe('GET, OPTIONS');
    }
  });

  it('serves the feed catalogue', async () => {
    const res = await handleRequest(req('/feeds'), ENV, okFetch());
    expect(res.status).toBe(200);
    const body = (await res.json()) as { feeds: { id: string }[] };
    expect(body.feeds.map((f) => f.id)).toContain('mbta-alerts');
  });

  it('404s unknown paths', async () => {
    expect((await handleRequest(req('/'), ENV, okFetch())).status).toBe(404);
    expect((await handleRequest(req('/admin'), ENV, okFetch())).status).toBe(404);
  });
});

describe('allowlist enforcement', () => {
  it('is not an open proxy', async () => {
    const upstream = okFetch();
    const res = await handleRequest(
      req('/fetch?url=https://evil.test/steal.pb'),
      ENV,
      upstream,
    );
    expect(res.status).toBe(403);
    expect(res.headers.get('X-Proxy-Error')).toBe('feed_not_allowed');
    expect(upstream).not.toHaveBeenCalled();
  });

  it('blocks internal addresses even when supplied directly', async () => {
    const upstream = okFetch();
    for (const target of [
      'http://169.254.169.254/latest/meta-data/',
      'https://localhost:8080/a.pb',
      'https://10.0.0.5/a.pb',
      'file:///etc/passwd',
    ]) {
      const res = await handleRequest(
        req(`/fetch?url=${encodeURIComponent(target)}`),
        ENV,
        upstream,
      );
      expect(res.status, target).toBe(403);
    }
    expect(upstream).not.toHaveBeenCalled();
  });

  it('rejects an unknown feed id without calling upstream', async () => {
    const upstream = okFetch();
    const res = await handleRequest(req('/fetch?feed=nope'), ENV, upstream);
    expect(res.status).toBe(403);
    expect(upstream).not.toHaveBeenCalled();
  });

  it('requires a feed parameter', async () => {
    const res = await handleRequest(req('/fetch'), ENV, okFetch());
    expect(res.status).toBe(400);
    expect(res.headers.get('X-Proxy-Error')).toBe('missing_feed_parameter');
  });

  it('accepts an allow-listed url spelled exactly', async () => {
    const res = await handleRequest(
      req('/fetch?url=https%3A%2F%2Fcdn.mbta.com%2Frealtime%2FAlerts.pb'),
      ENV,
      okFetch(),
    );
    expect(res.status).toBe(200);
  });
});

describe('successful fetch', () => {
  it('returns the bytes untouched as protobuf', async () => {
    const res = await handleRequest(req('/fetch?feed=mbta-alerts'), ENV, okFetch());
    expect(res.status).toBe(200);
    expect(res.headers.get('Content-Type')).toBe('application/x-protobuf');
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(PB);
  });

  it('never stores the feed', async () => {
    const res = await handleRequest(req('/fetch?feed=mbta-alerts'), ENV, okFetch());
    expect(res.headers.get('Cache-Control')).toBe('no-store');
  });

  it('reports which feed it served', async () => {
    const res = await handleRequest(req('/fetch?feed=mbta-trips'), ENV, okFetch());
    expect(res.headers.get('X-Proxy-Feed-Id')).toBe('mbta-trips');
    expect(res.headers.get('X-Proxy-Upstream-Status')).toBe('200');
  });
});

describe('upstream failures are distinguishable', () => {
  it('reports upstream error status as 502', async () => {
    const upstream = vi.fn(async () => new Response('nope', { status: 500 })) as unknown as typeof fetch;
    const res = await handleRequest(req('/fetch?feed=mbta-alerts'), ENV, upstream);
    expect(res.status).toBe(502);
    expect(res.headers.get('X-Proxy-Error')).toBe('upstream_status');
  });

  it('reports an unreachable upstream', async () => {
    const upstream = vi.fn(async () => {
      throw new Error('connection refused');
    }) as unknown as typeof fetch;
    const res = await handleRequest(req('/fetch?feed=mbta-alerts'), ENV, upstream);
    expect(res.status).toBe(502);
    expect(res.headers.get('X-Proxy-Error')).toBe('upstream_unreachable');
  });

  it('caps oversized responses', async () => {
    const big = new Uint8Array(4096);
    const upstream = okFetch(big);
    const res = await handleRequest(
      req('/fetch?feed=mbta-alerts'),
      { ...ENV, MAX_RESPONSE_BYTES: '1024' },
      upstream,
    );
    expect(res.status).toBe(413);
    expect(res.headers.get('X-Proxy-Error')).toBe('upstream_too_large');
  });

  it('refuses to follow a redirect off the allow-listed origin', async () => {
    // Kapının GERÇEKTEN ölçülmesi için sahte upstream'in ikinci çağrıda BAŞARILI
    // olması şart: her çağrıda 302 dönen bir mock, yönlendirme sayacı dolduğu için
    // origin kontrolü kaldırılsa bile aynı hatayı üretir ve testi sahte-yeşil yapar.
    const seen: string[] = [];
    const upstream = vi.fn(async (input: RequestInfo | URL) => {
      const target = String(input);
      seen.push(target);
      if (target === 'https://cdn.mbta.com/realtime/Alerts.pb') {
        return new Response(null, {
          status: 302,
          headers: { Location: 'https://evil.test/steal.pb' },
        });
      }
      return new Response(PB, { status: 200 });
    }) as unknown as typeof fetch;

    const res = await handleRequest(req('/fetch?feed=mbta-alerts'), ENV, upstream);
    expect(res.status).toBe(502);
    expect(res.headers.get('X-Proxy-Error')).toBe('upstream_redirect_blocked');
    expect(seen).not.toContain('https://evil.test/steal.pb');
  });

  it('follows a same-origin redirect', async () => {
    let call = 0;
    const upstream = vi.fn(async () => {
      call += 1;
      if (call === 1) {
        return new Response(null, {
          status: 302,
          headers: { Location: 'https://cdn.mbta.com/realtime/Alerts-v2.pb' },
        });
      }
      return new Response(PB, { status: 200 });
    }) as unknown as typeof fetch;

    const res = await handleRequest(req('/fetch?feed=mbta-alerts'), ENV, upstream);
    expect(res.status).toBe(200);
    expect(call).toBe(2);
  });
});

describe('rate limiting', () => {
  it('returns 429 with Retry-After once the window is full', async () => {
    const env: Env = { ...ENV, RATE_LIMIT_REQUESTS: '2', RATE_LIMIT_WINDOW_MS: '60000' };
    const upstream = okFetch();
    const make = () =>
      new Request('https://proxy.test/fetch?feed=mbta-alerts', {
        headers: { Origin: ORIGIN, 'CF-Connecting-IP': '203.0.113.9' },
      });

    expect((await handleRequest(make(), env, upstream)).status).toBe(200);
    expect((await handleRequest(make(), env, upstream)).status).toBe(200);
    const third = await handleRequest(make(), env, upstream);
    expect(third.status).toBe(429);
    expect(third.headers.get('Retry-After')).toBe('60');
  });
});
