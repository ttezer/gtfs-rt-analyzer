# GTFS-RT Analyzer UI

Bu klasör bağımlılıksız, statik bir tarayıcı arayüzüdür.

## WASM çıktısını üretme

Repo kökünden:

```sh
wasm-pack build crates/rt-wasm --target web --release --out-dir ../../ui/pkg
```

Sonra herhangi bir statik HTTP sunucusuyla `ui/` klasörünü servis edin. `file://`
üzerinden açmayın; ES module ve WASM yükleme politikaları HTTP ister.

## Akış

- Direct fetch tercih edilir.
- Direct fetch tarayıcı tarafından CORS nedeniyle engellenirse ve Proxy URL verilmişse
  aynı feed proxy üzerinden denenir.
- Proxy, yalnızca kendi allowlist'indeki adresleri kabul eder.
- Yerel dosya analizi tarayıcıda yapılır; ham baytlar saklanmaz.
- Geçmişte yalnızca son sekiz rapor özeti tutulur.

## Feed kataloğu

`feeds.json`, MobilityDatabase'den alınmış sabit bir GTFS-Realtime snapshot'ıdır.
İlk sürümde yalnızca HTTPS kullanan, kimlik doğrulaması istemeyen ve URL'sinde
credential benzeri sorgu parametresi bulunmayan kayıtlar arayüzde çalıştırılabilir
aday olarak tutulur. Katalog gerektiğinde elle yenilenir; tarayıcı her açılışta
MobilityDatabase API'sine bağlanmaz.

Katalogda `static_reference` varsa feed seçildiğinde `schedule-scores.json`
içindeki hazır Yayın ve Genel skoru gösterilir. Bu skorlar bağlı Schedule'a
aittir; GTFS-Realtime verisinin kalite skoru değildir. Schedule bağlantısı
olmayan veya henüz analiz edilmemiş kayıtlarda skor gösterilmez. Skor snapshot'ı
GitHub Actions tarafından haftalık yenilenir.
