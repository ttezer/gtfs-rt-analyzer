#![no_main]

use gtfs_rt_model::decode_feed_message;
use libfuzzer_sys::fuzz_target;

// Herhangi bir bayt dizisi decoder için geçerli bir girdidir: başarı, kısmi
// sonuç ya da anomali dönebilir; hiçbir girdi panik üretmemelidir.
fuzz_target!(|data: &[u8]| {
    let _ = decode_feed_message(data);
});
