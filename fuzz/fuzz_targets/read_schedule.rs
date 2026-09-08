#![no_main]

use gtfs_static::StaticFeed;
use libfuzzer_sys::fuzz_target;

// Statik GTFS okuyucusu tarayıcıda KULLANICININ YÜKLEDİĞİ dosyayı açar: ZIP
// çerçevelemesi, sıkıştırma ve CSV ayrıştırma tamamen düşmanca girdiye bakar.
// Hiçbir arşiv panik, sonsuz döngü ya da denetimsiz bellek büyümesi üretmemeli;
// bozuk arşivin doğru sonucu bir `Err`'dir.
fuzz_target!(|data: &[u8]| {
    let _ = StaticFeed::from_zip_bytes(data);
});
