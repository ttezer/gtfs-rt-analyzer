// Çözümleme iş parçacığı.
//
// Tarife yüklemek ölçülen en büyük feed'de 26,8 saniye sürüyor ve ana iş parçacığında
// koşarsa arayüz o süre boyunca donar — düğmeler tepki vermez, sayaçlar durur. Burada
// koştuğunda arayüz akıcı kalır ve kullanıcı ne olduğunu görebilir.
//
// Tarife bu iş parçacığında YAŞAR: ana tarafa taşımak, WebAssembly belleğindeki yapıyı
// serileştirmek demek olurdu ve önbelleğin bütün anlamı kaybolurdu.

import init, { analyze_feed, LoadedSchedule } from "./pkg/gtfs_rt_wasm.js";

let started = null;
let schedule = null;

function ready() {
  // `init()` yalnızca bir kez çağrılır; sonraki mesajlar aynı sözü bekler.
  started ??= init();
  return started;
}

function releaseSchedule() {
  // WebAssembly belleği JavaScript çöp toplayıcısına tabi değil: elle bırakılmazsa
  // her yeni tarife öncekinin üstüne birikir.
  schedule?.free();
  schedule = null;
}

self.onmessage = async (event) => {
  const { id, type, bytes } = event.data ?? {};

  try {
    await ready();

    switch (type) {
      case "load-schedule": {
        releaseSchedule();
        const started = performance.now();
        schedule = new LoadedSchedule(new Uint8Array(bytes));
        self.postMessage({
          id,
          type: "schedule-loaded",
          trips: schedule.trips,
          stopTimes: schedule.stopTimes,
          ms: Math.round(performance.now() - started),
        });
        break;
      }

      case "clear-schedule": {
        releaseSchedule();
        self.postMessage({ id, type: "schedule-cleared" });
        break;
      }

      case "analyze": {
        const payload = new Uint8Array(bytes);
        const started = performance.now();
        // Tarife yüklüyse tutarlılık kuralları da koşar; yoksa yalnız protobuf çözülür.
        const json = schedule ? schedule.analyze(payload) : analyze_feed(payload);
        self.postMessage({ id, type: "report", json, ms: Math.round(performance.now() - started) });
        break;
      }

      default:
        self.postMessage({ id, type: "error", error: `bilinmeyen istek: ${type}` });
    }
  } catch (error) {
    // Tarife yükleme başarısızsa önbellek yarım kalmamalı.
    if (type === "load-schedule") releaseSchedule();
    self.postMessage({ id, type: "error", error: String(error) });
  }
};
