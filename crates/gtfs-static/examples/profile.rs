//! Statik GTFS okumanın süre ve bellek maliyetini ölçer.
//! `cargo run --release -p gtfs-static --example profile -- feed.zip [trip_id ...]`

use std::collections::BTreeSet;
use std::time::Instant;

use gtfs_static::{StaticFeed, TripFilter};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("kullanım: profile <feed.zip> [trip_id ...]");
    let wanted: BTreeSet<String> = args.collect();
    let bytes = std::fs::read(&path).expect("okunamadı");

    let filter = if wanted.is_empty() {
        TripFilter::All
    } else {
        TripFilter::Only(&wanted)
    };

    let t = Instant::now();
    let feed = StaticFeed::from_zip_bytes_filtered(&bytes, filter).expect("çözümlenemedi");
    let elapsed = t.elapsed();

    println!("arşiv        : {:.1} MB", bytes.len() as f64 / 1_048_576.0);
    println!(
        "filtre       : {}",
        if wanted.is_empty() {
            "yok (tüm seferler)".to_owned()
        } else {
            format!("{} sefer", wanted.len())
        }
    );
    println!("süre         : {:.0} ms", elapsed.as_secs_f64() * 1000.0);
    println!("sefer        : {}", feed.trips().len());
    println!("durak        : {}", feed.stops().len());
    println!("tutulan satır: {}", feed.stop_time_count());
}
