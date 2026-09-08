//! Bir `.pb` dosyasını çözer ve gördüklerini döker.
//!
//! Geliştirme aracıdır, ürün CLI'ı değil: gerçek feed'ler üzerinde decoder'ın
//! davranışını ölçmek için. `cargo run --example inspect -- feed.pb`

use std::collections::BTreeMap;

fn main() {
    let path = match std::env::args().nth(1) {
        Some(p) => p,
        None => {
            eprintln!("kullanım: inspect <dosya.pb>");
            std::process::exit(2);
        }
    };

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: okunamadı: {e}");
            std::process::exit(2);
        }
    };

    let started = std::time::Instant::now();
    let out = gtfs_rt_model::decode_feed_message(&bytes);
    let elapsed = started.elapsed();

    let header = out.message.header.as_ref();
    println!("dosya            : {path}");
    println!("boyut            : {} bayt", bytes.len());
    println!(
        "çözme süresi     : {:.1} ms",
        elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "rt sürümü        : {}",
        header
            .and_then(|h| h.gtfs_realtime_version.as_deref())
            .unwrap_or("-")
    );
    println!(
        "header timestamp : {}",
        header
            .and_then(|h| h.timestamp)
            .map_or("-".to_string(), |t| t.to_string())
    );
    println!("entity sayısı    : {}", out.message.entity.len());

    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for e in &out.message.entity {
        let kind = if e.trip_update.is_some() {
            "trip_update"
        } else if e.vehicle.is_some() {
            "vehicle"
        } else if e.alert.is_some() {
            "alert"
        } else if e.shape.is_some() {
            "shape"
        } else if e.stop.is_some() {
            "stop"
        } else if e.trip_modifications.is_some() {
            "trip_modifications"
        } else {
            "(payload yok)"
        };
        *kinds.entry(kind).or_default() += 1;
    }
    for (kind, count) in &kinds {
        println!("  {kind:<20} {count}");
    }

    println!("anomali sayısı   : {}", out.anomalies.len());
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for a in &out.anomalies {
        *by_kind.entry(a.kind.stable_name()).or_default() += 1;
    }
    for (kind, count) in &by_kind {
        println!("  {kind:<28} {count}");
    }

    // İlk birkaç örneği tam yoluyla göster — teşhis için sayı değil konum gerekir.
    for a in out.anomalies.iter().take(8) {
        println!("    · {a}");
    }
}
