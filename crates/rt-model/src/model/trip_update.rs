//! `TripUpdate` ve iç mesajları — bir seferin gerçekleşen/tahmin edilen zamanlaması.

use crate::decode::{DecodeCtx, Message};
use crate::model::descriptor::{TripDescriptor, VehicleDescriptor};
use crate::model::enums::{DropOffPickupType, OccupancyStatus, StopTimeScheduleRelationship};
use crate::model::macros::{push_if, set_if};
use crate::wire::Field;

/// Bir sefer için zamanlama güncellemesi. `trip` proto2 `required`'dır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripUpdate {
    pub trip: Option<TripDescriptor>,
    pub vehicle: Option<VehicleDescriptor>,
    pub stop_time_update: Vec<StopTimeUpdate>,
    pub timestamp: Option<u64>,
    pub delay: Option<i32>,
    pub trip_properties: Option<TripProperties>,
}

impl Message for TripUpdate {
    const NAME: &'static str = "TripUpdate";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.trip, ctx.nested(f, "trip")),
            2 => {
                let i = self.stop_time_update.len();
                push_if!(
                    self.stop_time_update,
                    ctx.nested_repeated(f, "stop_time_update", i)
                );
            }
            3 => set_if!(self.vehicle, ctx.nested(f, "vehicle")),
            4 => set_if!(self.timestamp, ctx.u64(f)),
            5 => set_if!(self.delay, ctx.i32(f)),
            6 => set_if!(self.trip_properties, ctx.nested(f, "trip_properties")),
            _ => return false,
        }
        true
    }

    fn check_required(&self, ctx: &mut DecodeCtx) {
        if self.trip.is_none() {
            ctx.missing_required("TripUpdate.trip");
        }
    }
}

/// Tek durak için varış/kalkış güncellemesi.
///
/// `stop_id` ve `stop_sequence`'ın ikisi de opsiyoneldir ama en az biri gerekir —
/// bu kısıt spec düzyazısındadır, alan tablosunda değil, dolayısıyla burada
/// denetlenmez; kural katmanının işidir.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StopTimeUpdate {
    pub stop_sequence: Option<u32>,
    pub stop_id: Option<String>,
    pub arrival: Option<StopTimeEvent>,
    pub departure: Option<StopTimeEvent>,
    pub departure_occupancy_status: Option<OccupancyStatus>,
    pub schedule_relationship: Option<StopTimeScheduleRelationship>,
    pub stop_time_properties: Option<StopTimeProperties>,
}

impl Message for StopTimeUpdate {
    const NAME: &'static str = "TripUpdate.StopTimeUpdate";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.stop_sequence, ctx.u32(f)),
            2 => set_if!(self.arrival, ctx.nested(f, "arrival")),
            3 => set_if!(self.departure, ctx.nested(f, "departure")),
            4 => set_if!(self.stop_id, ctx.string(f)),
            5 => set_if!(self.schedule_relationship, ctx.enum_value(f)),
            6 => set_if!(
                self.stop_time_properties,
                ctx.nested(f, "stop_time_properties")
            ),
            7 => set_if!(self.departure_occupancy_status, ctx.enum_value(f)),
            _ => return false,
        }
        true
    }
}

/// Varış ya da kalkış olayı. `delay` ve `time`'dan en az biri gerekir (spec düzyazısı).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StopTimeEvent {
    pub delay: Option<i32>,
    pub time: Option<i64>,
    pub uncertainty: Option<i32>,
    pub scheduled_time: Option<i64>,
}

impl Message for StopTimeEvent {
    const NAME: &'static str = "TripUpdate.StopTimeEvent";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.delay, ctx.i32(f)),
            2 => set_if!(self.time, ctx.i64(f)),
            3 => set_if!(self.uncertainty, ctx.i32(f)),
            4 => set_if!(self.scheduled_time, ctx.i64(f)),
            _ => return false,
        }
        true
    }
}

/// Durağa özgü, statik feed'i geçersiz kılan özellikler.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StopTimeProperties {
    pub assigned_stop_id: Option<String>,
    pub stop_headsign: Option<String>,
    pub pickup_type: Option<DropOffPickupType>,
    pub drop_off_type: Option<DropOffPickupType>,
}

impl Message for StopTimeProperties {
    const NAME: &'static str = "TripUpdate.StopTimeUpdate.StopTimeProperties";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.assigned_stop_id, ctx.string(f)),
            2 => set_if!(self.stop_headsign, ctx.string(f)),
            3 => set_if!(self.pickup_type, ctx.enum_value(f)),
            4 => set_if!(self.drop_off_type, ctx.enum_value(f)),
            _ => return false,
        }
        true
    }
}

/// Sefere özgü, statik feed'i geçersiz kılan özellikler (`ADDED`/`NEW` seferlerde).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripProperties {
    pub trip_id: Option<String>,
    pub start_date: Option<String>,
    pub start_time: Option<String>,
    pub shape_id: Option<String>,
    pub trip_headsign: Option<String>,
    pub trip_short_name: Option<String>,
}

impl Message for TripProperties {
    const NAME: &'static str = "TripUpdate.TripProperties";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.trip_id, ctx.string(f)),
            2 => set_if!(self.start_date, ctx.string(f)),
            3 => set_if!(self.start_time, ctx.string(f)),
            4 => set_if!(self.shape_id, ctx.string(f)),
            5 => set_if!(self.trip_headsign, ctx.string(f)),
            6 => set_if!(self.trip_short_name, ctx.string(f)),
            _ => return false,
        }
        true
    }
}
