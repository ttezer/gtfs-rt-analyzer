//! Dar kapsamlı GTFS static ↔ realtime tutarlılık kuralları.
//!
//! Bu katman protobuf yapısal anomalilerini tekrar etmez; decoder ve static
//! okuyucu çıktısını alıp yalnızca iki veri kümesi arasındaki ölçülebilir
//! ilişkileri raporlar. Özellikle `ADDED`, `NEW` ve `DUPLICATED` trip'ler için
//! statik `trips.txt` karşılığı zorlanmaz.

use std::collections::BTreeMap;
use gtfs_rt_model::model::enums::TripScheduleRelationship;
use gtfs_rt_model::model::{FeedMessage, StopTimeUpdate, TripDescriptor};
use gtfs_static::StaticFeed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub code: &'static str,
    pub severity: Severity,
    pub entity_id: Option<String>,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsistencyReport {
    pub notices: Vec<Notice>,
    pub checked_entities: usize,
    pub checked_trip_references: usize,
    pub skipped_dynamic_trips: usize,
    pub checked_stop_time_updates: usize,
    pub vehicle_trips: usize,
}

impl ConsistencyReport {
    pub fn is_clean(&self) -> bool {
        self.notices.is_empty()
    }
}

/// Bir realtime snapshot'ını static GTFS indeksleriyle karşılaştırır.
pub fn check_snapshot(static_feed: &StaticFeed, realtime: &FeedMessage) -> ConsistencyReport {
    let mut report = ConsistencyReport::default();
    let mut vehicles_by_trip: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (index, entity) in realtime.entity.iter().enumerate() {
        report.checked_entities += 1;
        let entity_id = entity.id.clone();

        if let Some(trip_update) = &entity.trip_update {
            if let Some(trip) = &trip_update.trip {
                check_trip_reference(
                    static_feed,
                    trip,
                    entity_id.as_deref(),
                    &format!("entity[{index}].trip_update.trip"),
                    &mut report,
                );
                if is_dynamic_trip(trip) {
                    report.skipped_dynamic_trips += 1;
                } else if let Some(static_trip) = trip.trip_id.as_deref().and_then(|id| static_feed.trips().get(id)) {
                    for (stop_index, update) in trip_update.stop_time_update.iter().enumerate() {
                        report.checked_stop_time_updates += 1;
                        check_stop_time_update(
                            static_feed,
                            static_trip,
                            update,
                            entity_id.as_deref(),
                            &format!("entity[{index}].trip_update.stop_time_update[{stop_index}]"),
                            &mut report,
                        );
                    }
                }
            }
        }

        if let Some(vehicle) = &entity.vehicle {
            if let Some(trip) = &vehicle.trip {
                check_trip_reference(
                    static_feed,
                    trip,
                    entity_id.as_deref(),
                    &format!("entity[{index}].vehicle.trip"),
                    &mut report,
                );
                if is_dynamic_trip(trip) {
                    report.skipped_dynamic_trips += 1;
                }
                if let Some(trip_id) = trip.trip_id.as_ref() {
                    if !is_dynamic_trip(trip) {
                        report.vehicle_trips += 1;
                        vehicles_by_trip
                            .entry(trip_id.clone())
                            .or_default()
                            .push(entity_id.clone().unwrap_or_else(|| format!("entity[{index}]")));
                    }
                }
            }
        }
    }

    report.checked_trip_references = count_trip_references(realtime);
    report.notices.extend(multiple_vehicle_notices(vehicles_by_trip));
    report
}

fn count_trip_references(realtime: &FeedMessage) -> usize {
    realtime
        .entity
        .iter()
        .map(|entity| {
            usize::from(entity.trip_update.as_ref().is_some_and(|value| value.trip.is_some()))
                + usize::from(entity.vehicle.as_ref().is_some_and(|value| value.trip.is_some()))
        })
        .sum()
}

fn is_dynamic_trip(trip: &TripDescriptor) -> bool {
    matches!(
        trip.schedule_relationship,
        Some(
            TripScheduleRelationship::Added
                | TripScheduleRelationship::New
                | TripScheduleRelationship::Duplicated
        )
    )
}

fn check_trip_reference(
    static_feed: &StaticFeed,
    trip: &TripDescriptor,
    entity_id: Option<&str>,
    path: &str,
    report: &mut ConsistencyReport,
) {
    let Some(trip_id) = trip.trip_id.as_deref() else { return };
    if is_dynamic_trip(trip) {
        return;
    }

    let Some(static_trip) = static_feed.trips().get(trip_id) else {
        report.notices.push(Notice {
            code: "RT_TRIP_NOT_IN_STATIC",
            severity: Severity::Error,
            entity_id: entity_id.map(str::to_owned),
            path: format!("{path}.trip_id"),
            message: format!("trip_id '{trip_id}' realtime snapshot'ında var, static trips.txt içinde yok"),
        });
        return;
    };

    if let (Some(realtime_route), Some(static_route)) = (trip.route_id.as_deref(), Some(static_trip.route_id.as_str())) {
        if realtime_route != static_route {
            report.notices.push(Notice {
                code: "RT_ROUTE_MISMATCH",
                severity: Severity::Error,
                entity_id: entity_id.map(str::to_owned),
                path: format!("{path}.route_id"),
                message: format!("realtime route_id '{realtime_route}' ile static route_id '{static_route}' eşleşmiyor"),
            });
        }
    }
}

fn check_stop_time_update(
    static_feed: &StaticFeed,
    static_trip: &gtfs_static::Trip,
    update: &StopTimeUpdate,
    entity_id: Option<&str>,
    path: &str,
    report: &mut ConsistencyReport,
) {
    let Some(static_times) = static_feed.stop_times_for_trip(&static_trip.trip_id) else {
        return;
    };

    if let Some(stop_id) = update.stop_id.as_deref() {
        if !static_feed.stops().contains_key(stop_id) {
            report.notices.push(Notice {
                code: "RT_STOP_NOT_IN_STATIC",
                severity: Severity::Error,
                entity_id: entity_id.map(str::to_owned),
                path: format!("{path}.stop_id"),
                message: format!("stop_id '{stop_id}' realtime snapshot'ında var, static stops.txt içinde yok"),
            });
        }
    }

    if let Some(sequence) = update.stop_sequence {
        let matching = static_times.iter().find(|time| time.stop_sequence == sequence);
        let Some(matching) = matching else {
            report.notices.push(Notice {
                code: "RT_STOP_SEQUENCE_NOT_IN_STATIC",
                severity: Severity::Error,
                entity_id: entity_id.map(str::to_owned),
                path: format!("{path}.stop_sequence"),
                message: format!("stop_sequence {sequence} trip '{}' static stop_times.txt içinde yok", static_trip.trip_id),
            });
            return;
        };

        if let Some(stop_id) = update.stop_id.as_deref() {
            if stop_id != matching.stop_id {
                report.notices.push(Notice {
                    code: "RT_STOP_SEQUENCE_MISMATCH",
                    severity: Severity::Error,
                    entity_id: entity_id.map(str::to_owned),
                    path: format!("{path}.stop_id"),
                    message: format!("stop_sequence {sequence} static olarak '{}' gösteriyor, realtime '{stop_id}' gönderiyor", matching.stop_id),
                });
            }
        }
    }
}

fn multiple_vehicle_notices(vehicles_by_trip: BTreeMap<String, Vec<String>>) -> Vec<Notice> {
    vehicles_by_trip
        .into_iter()
        .filter_map(|(trip_id, entities)| {
            (entities.len() > 1).then(|| Notice {
                code: "RT_MULTIPLE_VEHICLES_FOR_TRIP",
                severity: Severity::Warning,
                entity_id: None,
                path: "vehicle.trip.trip_id".to_owned(),
                message: format!("trip_id '{trip_id}' için aynı snapshot'ta {} araç bildiriyor", entities.len()),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtfs_rt_model::model::feed::{FeedEntity, FeedMessage};
    use gtfs_rt_model::model::trip_update::{StopTimeUpdate, TripUpdate};
    use gtfs_rt_model::model::vehicle::VehiclePosition;
    use gtfs_rt_model::model::descriptor::TripDescriptor;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn static_feed() -> StaticFeed {
        let files = [
            ("routes.txt", "route_id\nR1\n"),
            ("trips.txt", "route_id,service_id,trip_id\nR1,S1,T1\n"),
            ("stops.txt", "stop_id\nS1\nS2\n"),
            ("stop_times.txt", "trip_id,stop_id,stop_sequence\nT1,S1,1\nT1,S2,2\n"),
        ];
        let mut output = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut output);
        for (name, content) in files {
            writer.start_file(name, SimpleFileOptions::default()).unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        StaticFeed::from_zip_bytes(&output.into_inner()).unwrap()
    }

    fn trip(trip_id: &str, route_id: Option<&str>) -> TripDescriptor {
        TripDescriptor {
            trip_id: Some(trip_id.to_owned()),
            route_id: route_id.map(str::to_owned),
            ..TripDescriptor::default()
        }
    }

    fn realtime(entities: Vec<FeedEntity>) -> FeedMessage {
        FeedMessage { header: None, entity: entities }
    }

    #[test]
    fn unknown_trip_and_route_are_reported() {
        let entity = FeedEntity {
            id: Some("e1".to_owned()),
            trip_update: Some(TripUpdate {
                trip: Some(trip("ghost", Some("R9"))),
                ..TripUpdate::default()
            }),
            ..FeedEntity::default()
        };
        let report = check_snapshot(&static_feed(), &realtime(vec![entity]));
        assert_eq!(report.notices[0].code, "RT_TRIP_NOT_IN_STATIC");
        assert_eq!(report.notices.len(), 1);
    }

    #[test]
    fn route_mismatch_and_stop_sequence_mismatch_are_reported() {
        let entity = FeedEntity {
            id: Some("e1".to_owned()),
            trip_update: Some(TripUpdate {
                trip: Some(trip("T1", Some("R9"))),
                stop_time_update: vec![StopTimeUpdate {
                    stop_id: Some("S2".to_owned()),
                    stop_sequence: Some(1),
                    ..StopTimeUpdate::default()
                }],
                ..TripUpdate::default()
            }),
            ..FeedEntity::default()
        };
        let report = check_snapshot(&static_feed(), &realtime(vec![entity]));
        assert!(report.notices.iter().any(|notice| notice.code == "RT_ROUTE_MISMATCH"));
        assert!(report.notices.iter().any(|notice| notice.code == "RT_STOP_SEQUENCE_MISMATCH"));
    }

    #[test]
    fn dynamic_relationship_skips_static_trip_requirement() {
        let entity = FeedEntity {
            id: Some("e1".to_owned()),
            vehicle: Some(VehiclePosition {
                trip: Some(TripDescriptor {
                    trip_id: Some("added-trip".to_owned()),
                    schedule_relationship: Some(TripScheduleRelationship::Added),
                    ..TripDescriptor::default()
                }),
                ..VehiclePosition::default()
            }),
            ..FeedEntity::default()
        };
        let report = check_snapshot(&static_feed(), &realtime(vec![entity]));
        assert!(report.is_clean());
        assert_eq!(report.skipped_dynamic_trips, 1);
    }

    #[test]
    fn multiple_vehicles_for_same_trip_are_a_warning() {
        let entities = (1..=2)
            .map(|number| FeedEntity {
                id: Some(format!("v{number}")),
                vehicle: Some(VehiclePosition {
                    trip: Some(trip("T1", None)),
                    ..VehiclePosition::default()
                }),
                ..FeedEntity::default()
            })
            .collect();
        let report = check_snapshot(&static_feed(), &realtime(entities));
        assert_eq!(report.notices.len(), 1);
        assert_eq!(report.notices[0].code, "RT_MULTIPLE_VEHICLES_FOR_TRIP");
        assert_eq!(report.notices[0].severity, Severity::Warning);
    }
}
