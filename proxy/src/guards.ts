/**
 * İstek ve hedef denetimleri.
 *
 * Allowlist tek başına yeterli bir savunma olurdu; buradaki kontroller onun
 * arkasındaki ikinci kat. Sebep: allowlist bir gün yanlışlıkla genişletilirse
 * (ör. bir yapılandırma hatası), SSRF yüzeyi tek satırlık bir değişiklikle
 * açılmasın.
 */

/** Yalnızca bu şemaya izin verilir. */
const ALLOWED_PROTOCOL = 'https:';

/**
 * Özel/ayrılmış IPv4 aralıkları. (127/8 ayrıca `localhost_blocked` olarak
 * sınıflanır, bu yüzden burada yok.)
 *
 * Hedef bir alan adıysa bu liste tam koruma sağlamaz — alan adı özel bir adrese
 * çözülebilir (DNS rebinding) ve Cloudflare Workers'ta çözülen IP'yi göremeyiz.
 * Asıl koruma allowlist'tir; burası IP literali içeren adresleri eler.
 */
const PRIVATE_V4 = [
  /^10\./,
  /^169\.254\./,
  /^172\.(1[6-9]|2\d|3[01])\./,
  /^192\.168\./,
  /^0\./,
  /^100\.(6[4-9]|[7-9]\d|1[01]\d|12[0-7])\./, // CGNAT 100.64/10
];

const LOCAL_HOSTNAMES = new Set(['localhost', 'ip6-localhost', 'ip6-loopback']);

export type TargetRejection =
  | 'not_https'
  | 'localhost_blocked'
  | 'private_address_blocked'
  | 'credentials_in_url'
  | 'malformed_url';

/** IPv4 metni özel aralıkta mı? */
function isPrivateV4(ip: string): boolean {
  return PRIVATE_V4.some((re) => re.test(ip));
}

/**
 * IPv6 metnini 16 bayta açar. IPv6 değilse `null`.
 *
 * Regex ile yüzeyden bakmak yetmiyor — ÖLÇÜLDÜ: WHATWG URL çözümlemesi
 * `[::ffff:127.0.0.1]` adresini `[::ffff:7f00:1]`, `[::127.0.0.1]` adresini
 * `[::7f00:1]` biçimine **normalize ediyor**. Noktalı gösterimi arayan bir kontrol
 * loopback'i kaçırır. Adresi baytlara açmak, gösterimden bağımsız tek doğru yol.
 */
export function ipv6Bytes(host: string): Uint8Array | null {
  if (!host.includes(':')) return null;

  // Sondaki IPv4 kuyruğu (::ffff:127.0.0.1) ayrı çözülür.
  let text = host;
  let tail: number[] = [];
  const v4 = /(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})$/.exec(text);
  if (v4 !== null) {
    const parts = v4[1]!.split('.').map((p) => Number.parseInt(p, 10));
    if (parts.some((n) => !Number.isFinite(n) || n < 0 || n > 255)) return null;
    tail = parts;
    text = text.slice(0, v4.index);
    if (text.endsWith(':') && !text.endsWith('::')) text = text.slice(0, -1);
  }

  const halves = text.split('::');
  if (halves.length > 2) return null;

  const toGroups = (chunk: string): number[] | null => {
    if (chunk === '') return [];
    const out: number[] = [];
    for (const g of chunk.split(':')) {
      if (g === '' || g.length > 4 || !/^[0-9a-f]+$/i.test(g)) return null;
      out.push(Number.parseInt(g, 16));
    }
    return out;
  };

  const head = toGroups(halves[0] ?? '');
  const rest = halves.length === 2 ? toGroups(halves[1] ?? '') : [];
  if (head === null || rest === null) return null;

  const tailGroups = tail.length === 4 ? 2 : 0;
  let groups: number[];
  if (halves.length === 2) {
    const fill = 8 - head.length - rest.length - tailGroups;
    if (fill < 0) return null;
    groups = [...head, ...new Array<number>(fill).fill(0), ...rest];
  } else {
    groups = [...head, ...rest];
  }

  const bytes: number[] = [];
  for (const g of groups) bytes.push((g >> 8) & 0xff, g & 0xff);
  bytes.push(...tail);

  return bytes.length === 16 ? new Uint8Array(bytes) : null;
}

/**
 * IPv6 baytları erişilmemesi gereken bir adres mi?
 *
 * IPv4-mapped (`::ffff:0:0/96`) ve IPv4-compatible (`::/96`) adresler gömülü IPv4
 * kurallarına göre değerlendirilir: `::ffff:127.0.0.1` loopback'tir ve gösterimi
 * ne olursa olsun engellenir.
 */
export function rejectIpv6(bytes: Uint8Array): TargetRejection | null {
  const zeroPrefix = bytes.slice(0, 10).every((b) => b === 0);
  const v4Of = (offset: number): string =>
    `${bytes[offset]}.${bytes[offset + 1]}.${bytes[offset + 2]}.${bytes[offset + 3]}`;

  if (zeroPrefix && bytes[10] === 0 && bytes[11] === 0) {
    const low = bytes.slice(12);
    if (low.every((b) => b === 0)) return 'localhost_blocked'; // ::
    if (low[0] === 0 && low[1] === 0 && low[2] === 0 && low[3] === 1) {
      return 'localhost_blocked'; // ::1
    }
    const v4 = v4Of(12); // ::a.b.c.d (IPv4-compatible)
    if (/^127\./.test(v4)) return 'localhost_blocked';
    return isPrivateV4(v4) ? 'private_address_blocked' : null;
  }

  if (zeroPrefix && bytes[10] === 0xff && bytes[11] === 0xff) {
    const v4 = v4Of(12); // ::ffff:a.b.c.d (IPv4-mapped)
    if (/^127\./.test(v4)) return 'localhost_blocked';
    return isPrivateV4(v4) ? 'private_address_blocked' : null;
  }

  const first = bytes[0] ?? 0;
  const second = bytes[1] ?? 0;
  if (first === 0xfe && (second & 0xc0) === 0x80) return 'private_address_blocked'; // fe80::/10
  if ((first & 0xfe) === 0xfc) return 'private_address_blocked'; // fc00::/7

  return null;
}

/** Hedef adresi denetler. `null` dönerse adres kabul edilebilir. */
export function rejectTarget(raw: string): TargetRejection | null {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return 'malformed_url';
  }

  if (url.protocol !== ALLOWED_PROTOCOL) return 'not_https';
  if (url.username || url.password) return 'credentials_in_url';

  // URL çözümlemesi IPv6'yı köşeli parantezle bırakır; adres için soyulur.
  const host = url.hostname.toLowerCase().replace(/^\[|\]$/g, '');

  if (LOCAL_HOSTNAMES.has(host)) return 'localhost_blocked';
  if (host.endsWith('.localhost')) return 'localhost_blocked';

  const v6 = ipv6Bytes(host);
  if (v6 !== null) return rejectIpv6(v6);

  // ÖLÇÜLDÜ: klasik atlatma biçimleri (2130706433, 0177.0.0.1, 127.1, 0x7f000001)
  // WHATWG URL çözümlemesi tarafından zaten noktalı gösterime normalize ediliyor,
  // dolayısıyla burada tek biçimi denetlemek yeterli.
  if (/^\d+\.\d+\.\d+\.\d+$/.test(host)) {
    if (/^127\./.test(host)) return 'localhost_blocked';
    if (isPrivateV4(host)) return 'private_address_blocked';
  }

  return null;
}

/**
 * İstek origin'i izinli mi?
 *
 * `Origin` başlığı yoksa istek tarayıcıdan gelmiyordur (curl, sunucu-sunucu).
 * Bunu reddetmiyoruz — proxy'nin işi CORS'u çözmek, kimlik doğrulamak değil —
 * ama CORS başlığı da vermiyoruz, dolayısıyla tarayıcı yine de okuyamaz.
 */
export function isAllowedOrigin(origin: string | null, allowed: readonly string[]): boolean {
  return origin !== null && allowed.includes(origin);
}

/** Yapılandırmadaki virgülle ayrılmış origin listesini çözer. */
export function parseAllowedOrigins(raw: string | undefined): string[] {
  if (!raw) return [];
  return raw
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}
