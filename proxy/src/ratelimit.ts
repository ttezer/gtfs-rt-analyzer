/**
 * Kayan pencere sayacı.
 *
 * ⚠️ **Sınırı bilinerek kabul edilmiştir:** sayaç Worker isolate'ının belleğindedir,
 * dolayısıyla küresel değildir. Cloudflare aynı anda birden çok isolate çalıştırabilir
 * ve her biri kendi sayacını tutar; gerçek sınır, yapılandırılan değerin isolate
 * sayısıyla çarpımıdır. Bu, kötü niyetli bir aktörü durdurmaz — amacı, tek bir
 * sekmenin döngüye girip upstream'i dövmesini engellemek.
 *
 * Küresel bir sınır Durable Objects ya da KV ister; ikisi de maliyet ve gecikme
 * getirir. Upstream'i korumak asıl olarak **allowlist**'in işi: proxy yalnızca
 * bizim seçtiğimiz birkaç adrese gidebiliyor.
 */
export class SlidingWindow {
  private readonly hits = new Map<string, number[]>();

  constructor(
    private readonly limit: number,
    private readonly windowMs: number,
  ) {}

  /**
   * İsteği kaydeder. `true` → izinli, `false` → sınır aşıldı.
   */
  allow(key: string, now: number): boolean {
    const cutoff = now - this.windowMs;
    const stamps = this.hits.get(key);

    if (stamps === undefined) {
      this.hits.set(key, [now]);
      this.sweep(cutoff);
      return true;
    }

    // Pencere dışına düşenleri at. Diziler kısa (limit kadar), yerinde filtre ucuz.
    let kept = 0;
    for (const t of stamps) {
      if (t > cutoff) stamps[kept++] = t;
    }
    stamps.length = kept;

    if (stamps.length >= this.limit) return false;
    stamps.push(now);
    return true;
  }

  /** Pencere dışında kalmış anahtarları düşürür — sınırsız bellek büyümesini önler. */
  private sweep(cutoff: number): void {
    if (this.hits.size < 1024) return;
    for (const [key, stamps] of this.hits) {
      const last = stamps[stamps.length - 1];
      if (last === undefined || last <= cutoff) {
        this.hits.delete(key);
      }
    }
  }

  /** Test görünürlüğü. */
  get trackedKeys(): number {
    return this.hits.size;
  }
}
