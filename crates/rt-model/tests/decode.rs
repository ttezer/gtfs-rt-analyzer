//! Decode katmanının uçtan uca davranışı.
//!
//! İki yönlü kapı: geçerli payload **sessiz** kalmalı (yanlış pozitif yok) ve her
//! anomali türü gerçekten **üretilebilmeli** (ölü kod yok). `gtfs-analyzer`'ın
//! `spec_conformance` + `emit_proof` ikilisinin karşılığı.

mod common;

use common::{entity_with_trip_update, header, minimal_feed, trip_update, Enc};
use gtfs_rt_model::anomaly::AnomalyKind;
use gtfs_rt_model::model::enums::{Incrementality, TripScheduleRelationship};
use gtfs_rt_model::{decode_feed_message, decode_feed_message_with, DecodeCtx};

/// Anomali türlerinin sabit adlarını verir — hangi anomalilerin çıktığını
/// tek satırda karşılaştırmak için.
fn kinds(bytes: &[u8]) -> Vec<&'static str> {
    decode_feed_message(bytes).anomalies.iter().map(|a| a.kind.stable_name()).collect()
}

// ── Geçerli veri sessiz kalmalı ──────────────────────────────────────────────

#[test]
fn minimal_valid_feed_is_silent() {
    let out = decode_feed_message(&minimal_feed());
    assert!(out.is_clean(), "beklenmeyen anomaliler: {:?}", out.anomalies);
    assert_eq!(out.message.entity.len(), 1);
    let e = &out.message.entity[0];
    assert_eq!(e.id.as_deref(), Some("e1"));
    assert_eq!(
        e.trip_update.as_ref().unwrap().trip.as_ref().unwrap().trip_id.as_deref(),
        Some("T1")
    );
}

#[test]
fn header_fields_round_trip() {
    let bytes = Enc::new()
        .msg_field(
            1,
            Enc::new()
                .string_field(1, "2.0")
                .varint_field(2, 1) // DIFFERENTIAL
                .varint_field(3, 1_757_000_000)
                .string_field(4, "v42"),
        )
        .msg_field(2, entity_with_trip_update("e1", "T1"))
        .into_bytes();

    let out = decode_feed_message(&bytes);
    assert!(out.is_clean(), "{:?}", out.anomalies);
    let h = out.message.header.unwrap();
    assert_eq!(h.gtfs_realtime_version.as_deref(), Some("2.0"));
    assert_eq!(h.incrementality, Some(Incrementality::Differential));
    assert_eq!(h.timestamp, Some(1_757_000_000));
    assert_eq!(h.feed_version.as_deref(), Some("v42"));
}

#[test]
fn all_three_payload_kinds_are_silent() {
    for (field, label) in [(3u32, "trip_update"), (4, "vehicle"), (5, "alert")] {
        let payload = match field {
            3 => trip_update("T1"),
            4 => Enc::new().msg_field(1, Enc::new().string_field(1, "T1")),
            _ => Enc::new().varint_field(6, 3), // Alert.cause = TECHNICAL_PROBLEM
        };
        let bytes = Enc::new()
            .msg_field(1, header(1))
            .msg_field(2, Enc::new().string_field(1, "e1").msg_field(field, payload))
            .into_bytes();
        let out = decode_feed_message(&bytes);
        assert!(out.is_clean(), "{label}: {:?}", out.anomalies);
    }
}

#[test]
fn negative_delay_round_trips() {
    // int32 negatifleri 10 baytlık varint olarak gelir; kısaltmak veri kaybettirir.
    let tu = trip_update("T1").i32_field(5, -120);
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(3, tu))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out.is_clean(), "{:?}", out.anomalies);
    assert_eq!(out.message.entity[0].trip_update.as_ref().unwrap().delay, Some(-120));
}

#[test]
fn position_with_both_required_fields_is_silent() {
    let vp = Enc::new()
        .msg_field(1, Enc::new().string_field(1, "T1"))
        .msg_field(2, Enc::new().f32_field(1, 41.0).f32_field(2, 29.0).f64_field(4, 12345.5));
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(4, vp))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out.is_clean(), "{:?}", out.anomalies);
    let p = out.message.entity[0].vehicle.as_ref().unwrap().position.as_ref().unwrap();
    assert_eq!(p.latitude, Some(41.0));
    assert_eq!(p.odometer, Some(12345.5));
}

// ── proto2 `required` eksiklikleri ───────────────────────────────────────────

#[test]
fn missing_feed_header_is_reported() {
    let bytes = Enc::new().msg_field(2, entity_with_trip_update("e1", "T1")).into_bytes();
    assert_eq!(kinds(&bytes), vec!["missing_required_field"]);
}

#[test]
fn missing_realtime_version_is_reported() {
    let bytes = Enc::new()
        .msg_field(1, Enc::new().varint_field(3, 1)) // yalnız timestamp
        .msg_field(2, entity_with_trip_update("e1", "T1"))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    let names: Vec<_> = out.anomalies.iter().map(|a| a.kind.stable_name()).collect();
    assert_eq!(names, vec!["missing_required_field"]);
    assert_eq!(out.anomalies[0].path, "header");
}

#[test]
fn missing_entity_id_is_reported() {
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().msg_field(3, trip_update("T1")))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out
        .anomalies
        .iter()
        .any(|a| a.kind == AnomalyKind::MissingRequiredField { field: "FeedEntity.id" }));
}

#[test]
fn missing_trip_in_trip_update_is_reported() {
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(
            2,
            Enc::new().string_field(1, "e1").msg_field(3, Enc::new().varint_field(4, 99)),
        )
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out
        .anomalies
        .iter()
        .any(|a| a.kind == AnomalyKind::MissingRequiredField { field: "TripUpdate.trip" }));
}

#[test]
fn missing_position_coordinates_are_reported_individually() {
    // Yalnız longitude verilmiş: latitude eksikliği ayrı bulgudur.
    let vp = Enc::new().msg_field(2, Enc::new().f32_field(2, 29.0));
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(4, vp))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    let missing: Vec<_> = out
        .anomalies
        .iter()
        .filter_map(|a| match a.kind {
            AnomalyKind::MissingRequiredField { field } => Some(field),
            _ => None,
        })
        .collect();
    assert_eq!(missing, vec!["Position.latitude"]);
}

// ── FeedEntity payload sayımı ────────────────────────────────────────────────

#[test]
fn entity_with_no_payload_is_reported() {
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1"))
        .into_bytes();
    assert_eq!(kinds(&bytes), vec!["no_payload"]);
}

#[test]
fn entity_with_two_payloads_is_reported() {
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(
            2,
            Enc::new()
                .string_field(1, "e1")
                .msg_field(3, trip_update("T1"))
                .msg_field(4, Enc::new().msg_field(1, Enc::new().string_field(1, "T1"))),
        )
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out
        .anomalies
        .iter()
        .any(|a| a.kind == AnomalyKind::MultiplePayloads { count: 2 }));
}

// ── Bilinmeyen alanlar ve enum'lar ───────────────────────────────────────────

#[test]
fn unknown_field_is_reported_with_its_number() {
    let bytes = Enc::new()
        .msg_field(1, header(1).varint_field(77, 1))
        .msg_field(2, entity_with_trip_update("e1", "T1"))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out.anomalies.iter().any(|a| a.kind == AnomalyKind::UnknownField { field: 77 }));
}

#[test]
fn extension_range_fields_are_distinguished_from_unknown() {
    // 1000-1999 ve 9000-9999 uzantı aralığıdır; "bilinmeyen alan" ile aynı şey değil.
    for field in [1000u32, 1999, 9000, 9999] {
        let bytes = Enc::new()
            .msg_field(1, header(1).varint_field(field, 1))
            .msg_field(2, entity_with_trip_update("e1", "T1"))
            .into_bytes();
        let out = decode_feed_message(&bytes);
        assert!(
            out.anomalies.iter().any(|a| a.kind == AnomalyKind::ExtensionField { field }),
            "alan {field} uzantı olarak işaretlenmedi: {:?}",
            out.anomalies
        );
    }
    // Aralığın hemen dışı uzantı DEĞİL.
    for field in [999u32, 2000, 8999, 10000] {
        let bytes = Enc::new()
            .msg_field(1, header(1).varint_field(field, 1))
            .msg_field(2, entity_with_trip_update("e1", "T1"))
            .into_bytes();
        let out = decode_feed_message(&bytes);
        assert!(
            out.anomalies.iter().any(|a| a.kind == AnomalyKind::UnknownField { field }),
            "alan {field} yanlışlıkla uzantı sayıldı"
        );
    }
}

#[test]
fn unknown_enum_value_is_reported_and_field_is_dropped() {
    // TripDescriptor.ScheduleRelationship'te 4 tanımsızdır.
    let tu = Enc::new().msg_field(1, Enc::new().string_field(1, "T1").varint_field(4, 4));
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(3, tu))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out.anomalies.iter().any(|a| matches!(
        a.kind,
        AnomalyKind::UnknownEnumValue { value: 4, .. }
    )));
    let trip = out.message.entity[0].trip_update.as_ref().unwrap().trip.as_ref().unwrap();
    assert_eq!(trip.schedule_relationship, None, "tanınmayan enum alanı düşmeli");
    assert_eq!(trip.trip_id.as_deref(), Some("T1"), "diğer alanlar korunmalı");
}

#[test]
fn known_enum_value_survives() {
    let tu = Enc::new().msg_field(1, Enc::new().string_field(1, "T1").varint_field(4, 3));
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(3, tu))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out.is_clean(), "{:?}", out.anomalies);
    let trip = out.message.entity[0].trip_update.as_ref().unwrap().trip.as_ref().unwrap();
    assert_eq!(trip.schedule_relationship, Some(TripScheduleRelationship::Canceled));
}

// ── Wire düzeyi bozukluklar ──────────────────────────────────────────────────

#[test]
fn wrong_wire_type_is_reported_and_other_fields_survive() {
    // gtfs_realtime_version string beklenir; varint gönderiliyor.
    let bytes = Enc::new()
        .msg_field(1, Enc::new().varint_field(1, 20).varint_field(3, 99))
        .msg_field(2, entity_with_trip_update("e1", "T1"))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out.anomalies.iter().any(|a| matches!(
        a.kind,
        AnomalyKind::UnexpectedWireType { expected: 2, found: 0 }
    )));
    // Aynı mesajdaki sağlam alan korunmalı.
    assert_eq!(out.message.header.as_ref().unwrap().timestamp, Some(99));
}

#[test]
fn invalid_utf8_is_reported() {
    let bytes = Enc::new()
        .msg_field(1, Enc::new().bytes_field(1, &[0xFF, 0xFE]).varint_field(3, 1))
        .msg_field(2, entity_with_trip_update("e1", "T1"))
        .into_bytes();
    let out = decode_feed_message(&bytes);
    assert!(out.anomalies.iter().any(|a| a.kind == AnomalyKind::InvalidUtf8));
}

#[test]
fn truncated_payload_keeps_what_was_decoded() {
    let full = minimal_feed();
    let cut = &full[..full.len() - 3];
    let out = decode_feed_message(cut);
    assert!(!out.is_clean());
    assert!(out.wire_anomalies().count() > 0);
    // Header tam olarak okunmuştu; kesik olan sonraki entity.
    assert!(out.message.header.is_some(), "kısmi sonuç korunmalı");
}

#[test]
fn html_error_page_does_not_panic() {
    // Yaygın vaka: feed yerine HTML hata sayfası dönmesi.
    let out = decode_feed_message(b"<!DOCTYPE html><html><body>502 Bad Gateway</body></html>");
    assert!(!out.is_clean());
    assert!(out.wire_anomalies().count() > 0);
}

#[test]
fn empty_payload_reports_missing_header_only() {
    assert_eq!(kinds(&[]), vec!["missing_required_field"]);
}

// ── Derinlik kapağı ──────────────────────────────────────────────────────────

#[test]
fn schema_has_no_cycles_so_nesting_cannot_run_away() {
    // ÖLÇÜM: GTFS-Realtime şemasında kendine dönen mesaj YOKTUR. Decoder yalnızca
    // TANIDIĞI alanlar için iç mesaja iner, dolayısıyla saldırgan bir payload
    // derinliği keyfi olarak artıramaz — sarma denemesi ikinci katmanda
    // "bilinmeyen alan"a çarpar ve iniş orada durur.
    //
    // Bu test, derinlik kapağının varsayılan değerinin (32) pratikte ULAŞILAMAZ
    // olduğunu kayda geçirir. Kapak yine de duruyor: şema ileride döngüsel bir alan
    // kazanırsa (ör. iç içe TripModifications) tek savunma o olur.
    let mut inner = Enc::new().string_field(1, "T1");
    for _ in 0..200 {
        inner = Enc::new().msg_field(7, inner); // TripDescriptor.modified_trip
    }
    let tu = Enc::new().msg_field(1, inner);
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(3, tu))
        .into_bytes();

    let out = decode_feed_message(&bytes);
    assert!(
        !out.anomalies.iter().any(|a| matches!(a.kind, AnomalyKind::DepthLimitExceeded { .. })),
        "kapağa ulaşıldı — şema döngüsel hale gelmiş olabilir, varsayımı gözden geçir"
    );
    // ModifiedTripSelector'da 7 numaralı alan yok: iniş burada durur.
    assert!(out.anomalies.iter().any(|a| a.kind == AnomalyKind::UnknownField { field: 7 }));
}

#[test]
fn deepest_legitimate_path_decodes_cleanly() {
    // Şemadaki en derin meşru yol 5 seviyedir:
    //   FeedMessage → entity → alert → informed_entity → trip → modified_trip
    // Derinlik kapağı bu sayının çok üstünde olmalı, aksi halde geçerli veriyi keser.
    let modified = Enc::new().string_field(1, "MOD1").string_field(2, "T1");
    let trip = Enc::new().string_field(1, "T1").msg_field(7, modified);
    let selector = Enc::new().string_field(2, "R1").msg_field(4, trip);
    let alert = Enc::new().varint_field(6, 3).msg_field(5, selector);
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(5, alert))
        .into_bytes();

    let out = decode_feed_message(&bytes);
    assert!(out.is_clean(), "en derin meşru yol anomali üretti: {:?}", out.anomalies);
    let sel = &out.message.entity[0].alert.as_ref().unwrap().informed_entity[0];
    let mt = sel.trip.as_ref().unwrap().modified_trip.as_ref().unwrap();
    assert_eq!(mt.modifications_id.as_deref(), Some("MOD1"));
}

#[test]
fn depth_limit_is_configurable() {
    let inner = Enc::new().msg_field(7, Enc::new().string_field(1, "T1"));
    let tu = Enc::new().msg_field(1, inner);
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, Enc::new().string_field(1, "e1").msg_field(3, tu))
        .into_bytes();

    // Varsayılan kapakla temiz geçer.
    assert!(decode_feed_message(&bytes).is_clean());
    // 2 seviyelik kapakla kapağa çarpar.
    let out = decode_feed_message_with(&bytes, DecodeCtx::new().with_max_depth(2));
    assert!(out
        .anomalies
        .iter()
        .any(|a| matches!(a.kind, AnomalyKind::DepthLimitExceeded { limit: 2 })));
}

// ── Yol (path) ve determinizm ────────────────────────────────────────────────

#[test]
fn anomaly_path_locates_the_field() {
    // İkinci entity'nin TripUpdate'inde bilinmeyen alan.
    let tu = trip_update("T1").varint_field(55, 1);
    let bytes = Enc::new()
        .msg_field(1, header(1))
        .msg_field(2, entity_with_trip_update("e1", "T1"))
        .msg_field(2, Enc::new().string_field(1, "e2").msg_field(3, tu))
        .into_bytes();

    let out = decode_feed_message(&bytes);
    let a = out
        .anomalies
        .iter()
        .find(|a| a.kind == AnomalyKind::UnknownField { field: 55 })
        .expect("bilinmeyen alan bulunamadı");
    assert_eq!(a.path, "entity[1].trip_update");
}

#[test]
fn decoding_is_deterministic() {
    // Aynı girdi, farklı koşumlar: model ve anomali listesi birebir aynı olmalı.
    let mut messy = Enc::new()
        .msg_field(1, header(1).varint_field(77, 1))
        .msg_field(2, Enc::new().string_field(1, "e1"))
        .msg_field(2, entity_with_trip_update("e2", "T2"))
        .into_bytes();
    messy.extend_from_slice(&[0x08]); // kesik varint

    let first = decode_feed_message(&messy);
    for _ in 0..20 {
        let again = decode_feed_message(&messy);
        assert_eq!(first, again);
    }
}

#[test]
fn anomalies_are_ordered_by_byte_offset() {
    let bytes = Enc::new()
        .msg_field(1, header(1).varint_field(77, 1))
        .msg_field(2, Enc::new().string_field(1, "e1")) // no_payload
        .into_bytes();
    let out = decode_feed_message(&bytes);
    let offsets: Vec<usize> = out.anomalies.iter().map(|a| a.offset).collect();
    let mut sorted = offsets.clone();
    sorted.sort_unstable();
    assert_eq!(offsets, sorted, "anomaliler bayt sırasında değil: {:?}", out.anomalies);
}
