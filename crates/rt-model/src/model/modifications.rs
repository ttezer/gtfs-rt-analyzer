//! `TripModifications` — mevcut seferlerin güzergahını değiştiren tanım.

use crate::decode::{DecodeCtx, Message};
use crate::model::macros::{push_if, set_if};
use crate::wire::Field;

/// Bir grup sefere uygulanan güzergah değişiklikleri.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripModifications {
    pub selected_trips: Vec<SelectedTrips>,
    pub start_times: Vec<String>,
    pub service_dates: Vec<String>,
    pub modifications: Vec<Modification>,
}

impl Message for TripModifications {
    const NAME: &'static str = "TripModifications";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => {
                let i = self.selected_trips.len();
                push_if!(
                    self.selected_trips,
                    ctx.nested_repeated(f, "selected_trips", i)
                );
            }
            2 => push_if!(self.start_times, ctx.string(f)),
            3 => push_if!(self.service_dates, ctx.string(f)),
            4 => {
                let i = self.modifications.len();
                push_if!(
                    self.modifications,
                    ctx.nested_repeated(f, "modifications", i)
                );
            }
            _ => return false,
        }
        true
    }
}

/// Tek bir güzergah değişikliği.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Modification {
    pub start_stop_selector: Option<StopSelector>,
    pub end_stop_selector: Option<StopSelector>,
    pub propagated_modification_delay: Option<i32>,
    pub replacement_stops: Vec<ReplacementStop>,
    pub service_alert_id: Option<String>,
    pub last_modified_time: Option<u64>,
}

impl Message for Modification {
    const NAME: &'static str = "TripModifications.Modification";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(
                self.start_stop_selector,
                ctx.nested(f, "start_stop_selector")
            ),
            2 => set_if!(self.end_stop_selector, ctx.nested(f, "end_stop_selector")),
            3 => set_if!(self.propagated_modification_delay, ctx.i32(f)),
            4 => {
                let i = self.replacement_stops.len();
                push_if!(
                    self.replacement_stops,
                    ctx.nested_repeated(f, "replacement_stops", i)
                );
            }
            5 => set_if!(self.service_alert_id, ctx.string(f)),
            6 => set_if!(self.last_modified_time, ctx.u64(f)),
            _ => return false,
        }
        true
    }
}

/// Değişikliğin uygulandığı seferler.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelectedTrips {
    pub trip_ids: Vec<String>,
    pub shape_id: Option<String>,
}

impl Message for SelectedTrips {
    const NAME: &'static str = "TripModifications.SelectedTrips";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => push_if!(self.trip_ids, ctx.string(f)),
            2 => set_if!(self.shape_id, ctx.string(f)),
            _ => return false,
        }
        true
    }
}

/// Durağı sırayla ya da kimlikle seçer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StopSelector {
    pub stop_sequence: Option<u32>,
    pub stop_id: Option<String>,
}

impl Message for StopSelector {
    const NAME: &'static str = "StopSelector";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.stop_sequence, ctx.u32(f)),
            2 => set_if!(self.stop_id, ctx.string(f)),
            _ => return false,
        }
        true
    }
}

/// Güzergaha eklenen durak.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplacementStop {
    pub travel_time_to_stop: Option<i32>,
    pub stop_id: Option<String>,
}

impl Message for ReplacementStop {
    const NAME: &'static str = "ReplacementStop";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.travel_time_to_stop, ctx.i32(f)),
            2 => set_if!(self.stop_id, ctx.string(f)),
            _ => return false,
        }
        true
    }
}
