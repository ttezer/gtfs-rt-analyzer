#![no_main]

use gtfs_rt_model::decode_feed_message;
use gtfs_rt_rules::check_snapshot;
use gtfs_static::{StaticFeed, TripFilter};
use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;

// Üretimdeki tam yol: realtime çözülür, atıf yapılan seferler toplanır, tarife o
// kümeye göre okunur, kurallar koşar. Tek tek bileşenler sağlam olsa da bu
// bileşimin panik üretmediği ayrıca gösterilmelidir.
//
// Girdi ikiye bölünür: ilk iki bayt realtime payload'ının uzunluğunu verir, kalanı
// arşivdir. Bölme noktasını fuzzer'ın kendisi keşfeder.
fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let split = usize::from(u16::from_le_bytes([data[0], data[1]])).min(data.len() - 2);
    let (realtime, archive) = data[2..].split_at(split);

    let decoded = decode_feed_message(realtime);

    let wanted: BTreeSet<String> = decoded
        .message
        .entity
        .iter()
        .filter_map(|entity| {
            entity
                .trip_update
                .as_ref()
                .and_then(|update| update.trip.as_ref())
                .or_else(|| entity.vehicle.as_ref().and_then(|vehicle| vehicle.trip.as_ref()))
        })
        .filter_map(|trip| trip.trip_id.clone())
        .collect();

    if let Ok(feed) = StaticFeed::from_zip_bytes_filtered(archive, TripFilter::Only(&wanted)) {
        let _ = check_snapshot(&feed, &decoded.message);
    }
});
