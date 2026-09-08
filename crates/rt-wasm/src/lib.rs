//! Tarayıcı köprüsü: decoder modelini küçük ve kararlı bir JSON raporuna çevirir.
//!
//! `gtfs-rt-model` tarayıcı API'si bilmez. Bu crate yalnızca WASM sınırında
//! çalışır; doğrulama kuralı eklemez ve ham payload'ı saklamaz.

use gtfs_rt_model::model::{FeedEntity, FeedMessage};
use gtfs_rt_model::{decode_feed_message, Anomaly};
use gtfs_rt_rules::{check_snapshot, ConsistencyReport};
use gtfs_static::{StaticFeed, TripFilter};
use serde::Serialize;
use std::collections::BTreeSet;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u8,
    bytes: usize,
    valid_protobuf: bool,
    header: Option<HeaderReport>,
    entities: EntityCounts,
    anomalies: Vec<AnomalyReport>,
    /// Statik GTFS verilmediğinde `None`. Tutarlılık kuralları yalnızca
    /// karşılaştıracak bir tarife varken anlamlıdır.
    #[serde(skip_serializing_if = "Option::is_none")]
    consistency: Option<ConsistencySection>,
    /// Statik arşiv okunamadıysa sebebi. Realtime raporu yine de üretilir —
    /// tarifenin bozuk olması, snapshot hakkında söylenebilecekleri geçersiz kılmaz.
    #[serde(skip_serializing_if = "Option::is_none")]
    schedule_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConsistencySection {
    schedule: ScheduleSummary,
    checked_entities: usize,
    checked_trip_references: usize,
    checked_stop_time_updates: usize,
    /// `ADDED`/`NEW`/`DUPLICATED` seferler: statik karşılık aranmaz, sayılır.
    skipped_dynamic_trips: usize,
    /// Tarifeden bellekte tutulan `stop_times` satırı sayısı. Snapshot'ta geçen
    /// seferlerle sınırlıdır; arşivin tamamı değil.
    indexed_stop_times: usize,
    vehicle_trips: usize,
    notices: Vec<NoticeReport>,
}

#[derive(Debug, Serialize)]
struct ScheduleSummary {
    routes: usize,
    trips: usize,
    stops: usize,
}

#[derive(Debug, Serialize)]
struct NoticeReport {
    code: &'static str,
    severity: &'static str,
    entity_id: Option<String>,
    path: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct HeaderReport {
    gtfs_realtime_version: Option<String>,
    incrementality: Option<String>,
    timestamp: Option<u64>,
    feed_version: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct EntityCounts {
    total: usize,
    trip_updates: usize,
    vehicles: usize,
    alerts: usize,
    shapes: usize,
    stops: usize,
    trip_modifications: usize,
    without_payload: usize,
    multiple_payloads: usize,
}

#[derive(Debug, Serialize)]
struct AnomalyReport {
    level: &'static str,
    kind: &'static str,
    message: String,
    path: String,
    offset: usize,
}

/// Bir GTFS-RT payload'ını JSON raporu olarak döndürür.
///
/// Fonksiyon hata döndürmez: bozuk veya protobuf olmayan payload da raporlanan
/// bir sonuçtur. JSON serileştirme başarısız olursa bu sözleşme bozulmasın diye
/// sabit bir hata raporu döner.
#[wasm_bindgen]
pub fn analyze_feed(bytes: &[u8]) -> String {
    render(build_report(bytes, None))
}

/// Bir GTFS-RT payload'ını, karşılık gelen statik GTFS arşiviyle birlikte inceler.
///
/// Realtime raporu her durumda üretilir. Statik arşiv açılamazsa tutarlılık bölümü
/// düşer ve sebebi `schedule_error` alanına yazılır: bozuk bir tarife, snapshot
/// hakkında söylenebilecekleri geçersiz kılmaz.
#[wasm_bindgen]
pub fn analyze_feed_with_schedule(bytes: &[u8], schedule_zip: &[u8]) -> String {
    render(build_report(bytes, Some(schedule_zip)))
}

/// Bir kez okunup birden çok snapshot için yeniden kullanılan tarife.
///
/// Periyodik izlemede aynı arşiv 60 saniyede bir yeniden çözülüyordu. ÖLÇÜM (Hollanda
/// ülke feed'i, 229 MB): her tur 21,6 saniye. Bir kez okunup tutulduğunda yalnızca ilk
/// tur o bedeli öder.
///
/// Burada tarife **filtresiz** okunur: hangi seferlerin sorulacağı ancak snapshot
/// geldiğinde bilinir, dolayısıyla önbellek tam olmak zorunda. Bu, tek atışlık yolun
/// tersi bir takas — orada bellek küçük tutulur, burada süre.
#[wasm_bindgen]
pub struct LoadedSchedule {
    feed: StaticFeed,
}

#[wasm_bindgen]
impl LoadedSchedule {
    /// Arşivi çözer. Başarısız olursa hata mesajı döner; çağıran tek atışlık yola
    /// düşebilir.
    #[wasm_bindgen(constructor)]
    pub fn new(schedule_zip: &[u8]) -> Result<LoadedSchedule, JsValue> {
        Self::load(schedule_zip).map_err(|error| JsValue::from_str(&error))
    }

    /// Bu tarifeye karşı bir realtime snapshot'ı inceler.
    pub fn analyze(&self, bytes: &[u8]) -> String {
        let decoded = decode_feed_message(bytes);
        let consistency = check_snapshot(&self.feed, &decoded.message);
        render(base_report(
            bytes,
            &decoded,
            Some(consistency_section(&self.feed, consistency)),
            None,
        ))
    }

    /// Tarifedeki sefer sayısı — UI'ın ne yüklendiğini gösterebilmesi için.
    #[wasm_bindgen(getter)]
    pub fn trips(&self) -> usize {
        self.feed.trips().len()
    }

    /// Bellekte tutulan `stop_times` satırı.
    #[wasm_bindgen(getter, js_name = stopTimes)]
    pub fn stop_times(&self) -> usize {
        self.feed.stop_time_count()
    }
}

impl LoadedSchedule {
    /// `JsValue` içermeyen iç kurucu: `JsValue` yalnızca WASM içinde kurulabildiği
    /// için hata yolu ancak böyle test edilebilir.
    fn load(schedule_zip: &[u8]) -> Result<Self, String> {
        StaticFeed::from_zip_bytes(schedule_zip)
            .map(|feed| LoadedSchedule { feed })
            .map_err(|error| format!("{error:?}"))
    }
}

fn render(report: Report) -> String {
    serde_json::to_string(&report).unwrap_or_else(|_| {
        r#"{"schema_version":1,"error":"report_serialization_failed"}"#.to_owned()
    })
}

fn build_report(bytes: &[u8], schedule_zip: Option<&[u8]>) -> Report {
    let decoded = decode_feed_message(bytes);

    let (consistency, schedule_error) = match schedule_zip {
        None => (None, None),
        Some(zip) => {
            // Sıra kasıtlı: önce realtime çözülür, sonra tarife YALNIZCA orada geçen
            // seferler için indekslenir.
            //
            // ÖLÇÜM (MBTA): arşivde 168.145 sefer / 4.494.139 stop_times satırı var,
            // snapshot bunların 883'üne (%0,5) atıfta bulunuyor. Tamamını okumak
            // 2,27 GB tepe bellek ve tarayıcıda ~20 saniye demekti.
            let wanted = referenced_trip_ids(&decoded.message);
            match StaticFeed::from_zip_bytes_filtered(zip, TripFilter::Only(&wanted)) {
                Ok(feed) => {
                    let report = check_snapshot(&feed, &decoded.message);
                    (Some(consistency_section(&feed, report)), None)
                }
                Err(error) => (None, Some(format!("{error:?}"))),
            }
        }
    };

    base_report(bytes, &decoded, consistency, schedule_error)
}

/// Rapor gövdesi — tek atışlık ve önbellekli yolların ortak parçası.
fn base_report(
    bytes: &[u8],
    decoded: &gtfs_rt_model::DecodedFeed,
    consistency: Option<ConsistencySection>,
    schedule_error: Option<String>,
) -> Report {
    Report {
        schema_version: 1,
        bytes: bytes.len(),
        valid_protobuf: decoded
            .anomalies
            .iter()
            .all(|anomaly| !anomaly.kind.is_payload_level() && !anomaly.kind.is_wire_level()),
        header: decoded.message.header.as_ref().map(header_report),
        entities: entity_counts(&decoded.message),
        anomalies: decoded.anomalies.iter().map(anomaly_report).collect(),
        consistency,
        schedule_error,
    }
}

/// Snapshot'ın atıfta bulunduğu bütün sefer kimlikleri.
///
/// Üç yerden gelir: `TripUpdate`, `VehiclePosition` ve `Alert.informed_entity`.
/// Kümenin fazla geniş olması zararsız (biraz fazla satır tutulur), fazla dar olması
/// ise bulguyu kaybettirir — o yüzden `schedule_relationship`'e bakılmadan hepsi alınır.
fn referenced_trip_ids(message: &FeedMessage) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut add = |trip: Option<&gtfs_rt_model::model::TripDescriptor>| {
        if let Some(id) = trip.and_then(|trip| trip.trip_id.as_ref()) {
            ids.insert(id.clone());
        }
    };

    for entity in &message.entity {
        add(entity
            .trip_update
            .as_ref()
            .and_then(|update| update.trip.as_ref()));
        add(entity
            .vehicle
            .as_ref()
            .and_then(|vehicle| vehicle.trip.as_ref()));
        if let Some(alert) = &entity.alert {
            for selector in &alert.informed_entity {
                add(selector.trip.as_ref());
            }
        }
    }
    ids
}

fn consistency_section(feed: &StaticFeed, report: ConsistencyReport) -> ConsistencySection {
    ConsistencySection {
        schedule: ScheduleSummary {
            routes: feed.routes().len(),
            trips: feed.trips().len(),
            stops: feed.stops().len(),
        },
        checked_entities: report.checked_entities,
        checked_trip_references: report.checked_trip_references,
        checked_stop_time_updates: report.checked_stop_time_updates,
        skipped_dynamic_trips: report.skipped_dynamic_trips,
        indexed_stop_times: feed.stop_time_count(),
        vehicle_trips: report.vehicle_trips,
        notices: report
            .notices
            .into_iter()
            .map(|notice| NoticeReport {
                code: notice.code,
                severity: notice.severity.as_str(),
                entity_id: notice.entity_id,
                path: notice.path,
                message: notice.message,
            })
            .collect(),
    }
}

fn header_report(header: &gtfs_rt_model::model::FeedHeader) -> HeaderReport {
    HeaderReport {
        gtfs_realtime_version: header.gtfs_realtime_version.clone(),
        incrementality: header
            .incrementality
            .map(|value| value.proto_name().to_owned()),
        timestamp: header.timestamp,
        feed_version: header.feed_version.clone(),
    }
}

fn entity_counts(message: &FeedMessage) -> EntityCounts {
    let mut counts = EntityCounts {
        total: message.entity.len(),
        ..EntityCounts::default()
    };

    for entity in &message.entity {
        add_entity(&mut counts, entity);
    }

    counts
}

fn add_entity(counts: &mut EntityCounts, entity: &FeedEntity) {
    let payloads = entity.payload_count();

    if payloads == 0 {
        counts.without_payload += 1;
    } else if payloads > 1 {
        counts.multiple_payloads += 1;
    }

    counts.trip_updates += usize::from(entity.trip_update.is_some());
    counts.vehicles += usize::from(entity.vehicle.is_some());
    counts.alerts += usize::from(entity.alert.is_some());
    counts.shapes += usize::from(entity.shape.is_some());
    counts.stops += usize::from(entity.stop.is_some());
    counts.trip_modifications += usize::from(entity.trip_modifications.is_some());
}

fn anomaly_report(anomaly: &Anomaly) -> AnomalyReport {
    let level = if anomaly.kind.is_payload_level() {
        "payload"
    } else if anomaly.kind.is_wire_level() {
        "wire"
    } else {
        "schema"
    };

    AnomalyReport {
        level,
        kind: anomaly.kind.stable_name(),
        message: anomaly.to_string(),
        path: anomaly.path.clone(),
        offset: anomaly.offset,
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze_feed, analyze_feed_with_schedule, LoadedSchedule};
    use std::io::{Cursor, Write};

    /// Tek seferli, tek duraklı asgari bir GTFS arşivi kurar.
    pub(super) fn schedule_zip() -> Vec<u8> {
        let files = [
            ("routes.txt", "route_id\nR1\n"),
            ("trips.txt", "route_id,service_id,trip_id\nR1,S1,T1\n"),
            ("stops.txt", "stop_id\nS1\n"),
            ("stop_times.txt", "trip_id,stop_id,stop_sequence\nT1,S1,1\n"),
        ];
        let mut output = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut output);
        for (name, content) in files {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        output.into_inner()
    }

    /// `trip_id` taşıyan tek entity'li geçerli bir realtime payload'ı kodlar.
    pub(super) fn realtime_with_trip(trip_id: &str) -> Vec<u8> {
        fn field(number: u8, payload: &[u8]) -> Vec<u8> {
            let mut out = vec![(number << 3) | 2, payload.len() as u8];
            out.extend_from_slice(payload);
            out
        }
        let mut header = vec![0x0a, 3];
        header.extend_from_slice(b"2.0");
        let trip = field(1, trip_id.as_bytes());
        let trip_descriptor = field(1, &trip);
        let mut entity = field(1, b"e1");
        entity.extend_from_slice(&field(3, &trip_descriptor));

        let mut out = field(1, &header);
        out.extend_from_slice(&field(2, &entity));
        out
    }

    #[test]
    fn empty_payload_produces_a_report() {
        let json = analyze_feed(&[]);
        assert!(json.contains(r#""schema_version":1"#));
        assert!(json.contains(r#""missing_required_field"#));
    }

    #[test]
    fn html_payload_is_classified_before_wire_decoding() {
        let json = analyze_feed(b"<!DOCTYPE html><html></html>");
        assert!(json.contains(r#""valid_protobuf":false"#));
        assert!(json.contains(r#""kind":"not_protobuf"#));
        assert!(json.contains(r#""level":"payload"#));
    }

    #[test]
    fn without_a_schedule_there_is_no_consistency_section() {
        // Tutarlılık, karşılaştıracak bir tarife olmadan iddia edilemez.
        let json = analyze_feed(&realtime_with_trip("T1"));
        assert!(!json.contains("consistency"));
    }

    #[test]
    fn a_matching_trip_produces_a_clean_consistency_section() {
        let json = analyze_feed_with_schedule(&realtime_with_trip("T1"), &schedule_zip());
        assert!(json.contains(r#""consistency""#), "{json}");
        assert!(json.contains(r#""notices":[]"#), "{json}");
        assert!(json.contains(r#""trips":1"#), "{json}");
    }

    /// Snapshot'ta bir sefer ve onun ilk durağı için güncelleme taşıyan payload.
    pub(super) fn realtime_with_stop_update(trip_id: &str, stop_id: &str, sequence: u8) -> Vec<u8> {
        fn field(number: u8, payload: &[u8]) -> Vec<u8> {
            let mut out = vec![(number << 3) | 2, payload.len() as u8];
            out.extend_from_slice(payload);
            out
        }
        let mut header = vec![0x0a, 3];
        header.extend_from_slice(b"2.0");

        let trip_descriptor = field(1, &field(1, trip_id.as_bytes()));
        // StopTimeUpdate: stop_sequence (1, varint) + stop_id (4, string)
        let mut stop_update = vec![0x08, sequence];
        stop_update.extend_from_slice(&field(4, stop_id.as_bytes()));

        let mut trip_update = trip_descriptor;
        trip_update.extend_from_slice(&field(2, &stop_update));

        let mut entity = field(1, b"e1");
        entity.extend_from_slice(&field(3, &trip_update));

        let mut out = field(1, &header);
        out.extend_from_slice(&field(2, &entity));
        out
    }

    #[test]
    fn a_referenced_trip_keeps_its_stop_times_through_the_filter() {
        // Tarife yalnızca snapshot'ta geçen seferler için indekslenir. Filtre fazla
        // dar olursa geçerli bir durak güncellemesi "statikte yok" diye YANLIŞ
        // raporlanır — bu testin koruduğu regresyon budur.
        let json =
            analyze_feed_with_schedule(&realtime_with_stop_update("T1", "S1", 1), &schedule_zip());
        assert!(json.contains(r#""notices":[]"#), "{json}");
        assert!(json.contains(r#""indexed_stop_times":1"#), "{json}");
        assert!(json.contains(r#""checked_stop_time_updates":1"#), "{json}");
    }

    #[test]
    fn a_trip_missing_from_the_schedule_is_reported() {
        let json = analyze_feed_with_schedule(&realtime_with_trip("GHOST"), &schedule_zip());
        assert!(json.contains("RT_TRIP_NOT_IN_STATIC"), "{json}");
    }

    #[test]
    fn the_cached_and_one_shot_paths_agree() {
        // Önbellekli yol tarifeyi FİLTRESİZ tutar, tek atışlık yol snapshot'a göre
        // filtreler. Filtre bir başarım tercihidir; iki yolun raporu AYNI olmalı,
        // aksi halde hangi yolu seçtiğimiz sonucu değiştiriyor demektir.
        let zip = schedule_zip();
        for rt in [
            realtime_with_trip("T1"),
            realtime_with_trip("GHOST"),
            realtime_with_stop_update("T1", "S1", 1),
            realtime_with_stop_update("T1", "GHOST", 9),
        ] {
            let one_shot = analyze_feed_with_schedule(&rt, &zip);
            let cached = LoadedSchedule::load(&zip).unwrap().analyze(&rt);

            // Tek meşru fark `indexed_stop_times`: önbellek filtresizdir, tek atışlık
            // yol snapshot'a göre daraltır. Bulgular ve denetim sayaçları AYNI olmalı.
            let strip = |json: &str| {
                let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
                if let Some(section) = value.get_mut("consistency") {
                    section
                        .as_object_mut()
                        .unwrap()
                        .remove("indexed_stop_times");
                }
                value
            };
            assert_eq!(strip(&one_shot), strip(&cached), "yollar ayrıştı");
        }
    }

    #[test]
    fn a_cached_schedule_reports_what_it_holds() {
        let schedule = LoadedSchedule::load(&schedule_zip()).unwrap();
        assert_eq!(schedule.trips(), 1);
        // Önbellek filtresizdir: tek atışlık yolun aksine satırları tutar.
        assert_eq!(schedule.stop_times(), 1);
    }

    #[test]
    fn a_broken_archive_is_rejected_by_the_cache() {
        assert!(LoadedSchedule::load(b"not a zip").is_err());
    }

    #[test]
    fn a_broken_schedule_still_yields_the_realtime_report() {
        // Bozuk bir tarife, snapshot hakkında söylenebilecekleri geçersiz kılmaz.
        let json = analyze_feed_with_schedule(&realtime_with_trip("T1"), b"not a zip");
        assert!(json.contains(r#""schedule_error""#), "{json}");
        assert!(!json.contains(r#""consistency""#), "{json}");
        assert!(json.contains(r#""total":1"#), "{json}");
    }
}
