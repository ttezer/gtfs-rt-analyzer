//! Tarayıcı köprüsü: decoder modelini küçük ve kararlı bir JSON raporuna çevirir.
//!
//! `gtfs-rt-model` tarayıcı API'si bilmez. Bu crate yalnızca WASM sınırında
//! çalışır; doğrulama kuralı eklemez ve ham payload'ı saklamaz.

use gtfs_rt_model::model::{FeedEntity, FeedMessage};
use gtfs_rt_model::{decode_feed_message, Anomaly};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
struct Report {
    schema_version: u8,
    bytes: usize,
    valid_protobuf: bool,
    header: Option<HeaderReport>,
    entities: EntityCounts,
    anomalies: Vec<AnomalyReport>,
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
    let decoded = decode_feed_message(bytes);
    let report = Report {
        schema_version: 1,
        bytes: bytes.len(),
        valid_protobuf: decoded.anomalies.iter().all(|anomaly| {
            !anomaly.kind.is_payload_level() && !anomaly.kind.is_wire_level()
        }),
        header: decoded.message.header.as_ref().map(header_report),
        entities: entity_counts(&decoded.message),
        anomalies: decoded.anomalies.iter().map(anomaly_report).collect(),
    };

    serde_json::to_string(&report)
        .unwrap_or_else(|_| r#"{"schema_version":1,"error":"report_serialization_failed"}"#.to_owned())
}

fn header_report(header: &gtfs_rt_model::model::FeedHeader) -> HeaderReport {
    HeaderReport {
        gtfs_realtime_version: header.gtfs_realtime_version.clone(),
        incrementality: header.incrementality.map(|value| value.proto_name().to_owned()),
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
    use super::analyze_feed;

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
}
