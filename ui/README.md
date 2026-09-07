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
