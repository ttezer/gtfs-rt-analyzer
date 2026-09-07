/**
 * İzin verilen GTFS-RT kaynakları.
 *
 * Bu dosya proxy'nin güvenlik sınırıdır. Burada olmayan hiçbir adrese istek
 * yapılmaz — proxy açık (open) bir proxy DEĞİLDİR ve öyle olmamalıdır: açık bir
 * proxy, üçüncü tarafların bizim adımıza istek yapmasına ve bizim IP'mizin
 * kötüye kullanılmasına yol açar.
 *
 * İstemci feed'i **kimliğiyle** ister (`?feed=<id>`); URL sunucu tarafında
 * çözülür. Böylece istemcinin gönderdiği hiçbir dize hedef adresi belirlemez.
 * Geriye dönük kolaylık için `?url=` de kabul edilir, ama yalnızca burada birebir
 * kayıtlı bir adresle eşleşiyorsa.
 */

export interface FeedEntry {
  /** Kısa, kalıcı kimlik — istemci bunu gönderir. */
  id: string;
  /** İnsan tarafından okunur ad; UI'da gösterilir. */
  label: string;
  /** Mutlak HTTPS adresi. */
  url: string;
  /** Feed türü; yalnızca bilgilendirme amaçlı. */
  kind: 'trip_updates' | 'vehicle_positions' | 'alerts' | 'combined';
}

/**
 * İlk sürüm kaydı.
 *
 * NOT: buradaki girdiler örnek değil, üretimde çekilecek gerçek adreslerdir.
 * Yeni feed eklemek bilinçli bir karardır ve gözden geçirilmelidir — her ekleme
 * proxy'nin erişebildiği adres uzayını genişletir.
 */
export const FEEDS: readonly FeedEntry[] = [
  {
    id: 'mbta-vehicles',
    label: 'MBTA — Vehicle Positions',
    url: 'https://cdn.mbta.com/realtime/VehiclePositions.pb',
    kind: 'vehicle_positions',
  },
  {
    id: 'mbta-trips',
    label: 'MBTA — Trip Updates',
    url: 'https://cdn.mbta.com/realtime/TripUpdates.pb',
    kind: 'trip_updates',
  },
  {
    id: 'mbta-alerts',
    label: 'MBTA — Service Alerts',
    url: 'https://cdn.mbta.com/realtime/Alerts.pb',
    kind: 'alerts',
  },
  {
    id: 'septa-trips',
    label: 'SEPTA — Trip Updates',
    url: 'https://www3.septa.org/gtfsrt/septa-pa-us/Trip/rtTripUpdates.pb',
    kind: 'trip_updates',
  },
  {
    id: 'septa-alerts',
    label: 'SEPTA — Service Alerts',
    url: 'https://www3.septa.org/gtfsrt/septa-pa-us/Service/rtServiceAlerts.pb',
    kind: 'alerts',
  },
] as const;

const BY_ID = new Map(FEEDS.map((f) => [f.id, f]));
const BY_URL = new Map(FEEDS.map((f) => [f.url, f]));

/** Kimliğe göre feed getirir. */
export function feedById(id: string): FeedEntry | undefined {
  return BY_ID.get(id);
}

/** Birebir eşleşen adrese göre feed getirir. Normalizasyon yapılmaz: bir dizeyi
 *  "aynı adres" saymak için yapılan her esneklik, atlatma yüzeyi açar. */
export function feedByUrl(url: string): FeedEntry | undefined {
  return BY_URL.get(url);
}

/** UI'ın listeleyebilmesi için genel katalog (adresler dahil, gizli değiller). */
export function catalogue(): FeedEntry[] {
  return FEEDS.map((f) => ({ ...f }));
}
