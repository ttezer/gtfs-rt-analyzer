//! `VehiclePosition` ve vagon ayrıntıları.

use crate::decode::{DecodeCtx, Message};
use crate::model::descriptor::{Position, TripDescriptor, VehicleDescriptor};
use crate::model::enums::{CongestionLevel, OccupancyStatus, VehicleStopStatus};
use crate::model::macros::{push_if, set_if};
use crate::wire::Field;

/// Aracın konumu ve durumu.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VehiclePosition {
    pub trip: Option<TripDescriptor>,
    pub vehicle: Option<VehicleDescriptor>,
    pub position: Option<Position>,
    pub current_stop_sequence: Option<u32>,
    pub stop_id: Option<String>,
    pub current_status: Option<VehicleStopStatus>,
    pub timestamp: Option<u64>,
    pub congestion_level: Option<CongestionLevel>,
    pub occupancy_status: Option<OccupancyStatus>,
    pub occupancy_percentage: Option<u32>,
    pub multi_carriage_details: Vec<CarriageDetails>,
}

impl Message for VehiclePosition {
    const NAME: &'static str = "VehiclePosition";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.trip, ctx.nested(f, "trip")),
            2 => set_if!(self.position, ctx.nested(f, "position")),
            3 => set_if!(self.current_stop_sequence, ctx.u32(f)),
            4 => set_if!(self.current_status, ctx.enum_value(f)),
            5 => set_if!(self.timestamp, ctx.u64(f)),
            6 => set_if!(self.congestion_level, ctx.enum_value(f)),
            7 => set_if!(self.stop_id, ctx.string(f)),
            8 => set_if!(self.vehicle, ctx.nested(f, "vehicle")),
            9 => set_if!(self.occupancy_status, ctx.enum_value(f)),
            10 => set_if!(self.occupancy_percentage, ctx.u32(f)),
            11 => {
                let i = self.multi_carriage_details.len();
                push_if!(
                    self.multi_carriage_details,
                    ctx.nested_repeated(f, "multi_carriage_details", i)
                );
            }
            _ => return false,
        }
        true
    }
}

/// Çok vagonlu araçta tek vagonun doluluk ayrıntısı.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarriageDetails {
    pub id: Option<String>,
    pub label: Option<String>,
    pub occupancy_status: Option<OccupancyStatus>,
    pub occupancy_percentage: Option<i32>,
    pub carriage_sequence: Option<u32>,
}

impl Message for CarriageDetails {
    const NAME: &'static str = "VehiclePosition.CarriageDetails";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.id, ctx.string(f)),
            2 => set_if!(self.label, ctx.string(f)),
            3 => set_if!(self.occupancy_status, ctx.enum_value(f)),
            4 => set_if!(self.occupancy_percentage, ctx.i32(f)),
            5 => set_if!(self.carriage_sequence, ctx.u32(f)),
            _ => return false,
        }
        true
    }
}
