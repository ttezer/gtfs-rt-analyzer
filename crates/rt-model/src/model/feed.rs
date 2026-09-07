//! `FeedMessage`, `FeedHeader`, `FeedEntity` — payload'ın kök yapısı.

use crate::anomaly::AnomalyKind;
use crate::decode::{DecodeCtx, Message};
use crate::model::alert::Alert;
use crate::model::enums::Incrementality;
use crate::model::macros::{push_if, set_if};
use crate::model::modifications::TripModifications;
use crate::model::shape_stop::{Shape, Stop};
use crate::model::trip_update::TripUpdate;
use crate::model::vehicle::VehiclePosition;
use crate::wire::Field;

/// Payload'ın kökü. `header` proto2 `required`'dır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedMessage {
    pub header: Option<FeedHeader>,
    pub entity: Vec<FeedEntity>,
}

impl Message for FeedMessage {
    const NAME: &'static str = "FeedMessage";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.header, ctx.nested(f, "header")),
            2 => {
                let i = self.entity.len();
                push_if!(self.entity, ctx.nested_repeated(f, "entity", i));
            }
            _ => return false,
        }
        true
    }

    fn check_required(&self, ctx: &mut DecodeCtx) {
        if self.header.is_none() {
            ctx.missing_required("FeedMessage.header");
        }
    }
}

/// Feed üstverisi. `gtfs_realtime_version` proto2 `required`'dır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedHeader {
    pub gtfs_realtime_version: Option<String>,
    pub incrementality: Option<Incrementality>,
    /// Feed'in üretildiği an (POSIX saniye). Tazelik ölçümünün dayanağı.
    pub timestamp: Option<u64>,
    pub feed_version: Option<String>,
}

impl Message for FeedHeader {
    const NAME: &'static str = "FeedHeader";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.gtfs_realtime_version, ctx.string(f)),
            2 => set_if!(self.incrementality, ctx.enum_value(f)),
            3 => set_if!(self.timestamp, ctx.u64(f)),
            4 => set_if!(self.feed_version, ctx.string(f)),
            _ => return false,
        }
        true
    }

    fn check_required(&self, ctx: &mut DecodeCtx) {
        if self.gtfs_realtime_version.is_none() {
            ctx.missing_required("FeedHeader.gtfs_realtime_version");
        }
    }
}

/// Tek bir varlık. `id` proto2 `required`'dır ve payload alanlarından **en fazla biri**
/// dolu olmalıdır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FeedEntity {
    pub id: Option<String>,
    pub is_deleted: Option<bool>,
    pub trip_update: Option<TripUpdate>,
    pub vehicle: Option<VehiclePosition>,
    pub alert: Option<Alert>,
    pub shape: Option<Shape>,
    pub stop: Option<Stop>,
    pub trip_modifications: Option<TripModifications>,
}

impl FeedEntity {
    /// Dolu payload alanlarının sayısı.
    pub fn payload_count(&self) -> usize {
        [
            self.trip_update.is_some(),
            self.vehicle.is_some(),
            self.alert.is_some(),
            self.shape.is_some(),
            self.stop.is_some(),
            self.trip_modifications.is_some(),
        ]
        .iter()
        .filter(|present| **present)
        .count()
    }
}

impl Message for FeedEntity {
    const NAME: &'static str = "FeedEntity";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.id, ctx.string(f)),
            2 => set_if!(self.is_deleted, ctx.bool(f)),
            3 => set_if!(self.trip_update, ctx.nested(f, "trip_update")),
            4 => set_if!(self.vehicle, ctx.nested(f, "vehicle")),
            5 => set_if!(self.alert, ctx.nested(f, "alert")),
            6 => set_if!(self.shape, ctx.nested(f, "shape")),
            7 => set_if!(self.stop, ctx.nested(f, "stop")),
            8 => set_if!(self.trip_modifications, ctx.nested(f, "trip_modifications")),
            _ => return false,
        }
        true
    }

    fn check_required(&self, ctx: &mut DecodeCtx) {
        if self.id.is_none() {
            ctx.missing_required("FeedEntity.id");
        }
        // Payload sayısı: spec `FeedEntity`'nin tam olarak bir şey taşımasını bekler.
        // `is_deleted` DIFFERENTIAL feed'lerde payload'sız silme bildirimi olarak
        // kullanılabildiği için, bu iki durum farklı sayılır ve ayrı raporlanır —
        // "hiç payload yok" her zaman hata değildir, kural katmanı incrementality'ye
        // bakarak karar verir.
        match self.payload_count() {
            0 => ctx.report(AnomalyKind::NoPayload, 0),
            1 => {}
            n => ctx.report(AnomalyKind::MultiplePayloads { count: n }, 0),
        }
    }
}
