//! `Shape` ve `Stop` — RT içinde tanımlanan, statik feed'de bulunmayan varlıklar.
//! (GTFS-Realtime v2.1 ile eklendi, `TripModifications` ile birlikte kullanılır.)

use crate::decode::{DecodeCtx, Message};
use crate::model::descriptor::TranslatedString;
use crate::model::enums::WheelchairBoarding;
use crate::model::macros::set_if;
use crate::wire::Field;

/// RT içinde tanımlanan güzergah geometrisi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    pub shape_id: Option<String>,
    pub encoded_polyline: Option<String>,
}

impl Message for Shape {
    const NAME: &'static str = "Shape";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.shape_id, ctx.string(f)),
            2 => set_if!(self.encoded_polyline, ctx.string(f)),
            _ => return false,
        }
        true
    }
}

/// RT içinde tanımlanan durak.
///
/// ⚠️ Alan numarası **10 kullanılmıyor** (spec'te atlanmış); 11 `parent_station`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stop {
    pub stop_id: Option<String>,
    pub stop_code: Option<TranslatedString>,
    pub stop_name: Option<TranslatedString>,
    pub tts_stop_name: Option<TranslatedString>,
    pub stop_desc: Option<TranslatedString>,
    pub stop_lat: Option<f32>,
    pub stop_lon: Option<f32>,
    pub zone_id: Option<String>,
    pub stop_url: Option<TranslatedString>,
    pub parent_station: Option<String>,
    pub stop_timezone: Option<String>,
    pub wheelchair_boarding: Option<WheelchairBoarding>,
    pub level_id: Option<String>,
    pub platform_code: Option<TranslatedString>,
}

impl Message for Stop {
    const NAME: &'static str = "Stop";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.stop_id, ctx.string(f)),
            2 => set_if!(self.stop_code, ctx.nested(f, "stop_code")),
            3 => set_if!(self.stop_name, ctx.nested(f, "stop_name")),
            4 => set_if!(self.tts_stop_name, ctx.nested(f, "tts_stop_name")),
            5 => set_if!(self.stop_desc, ctx.nested(f, "stop_desc")),
            6 => set_if!(self.stop_lat, ctx.f32(f)),
            7 => set_if!(self.stop_lon, ctx.f32(f)),
            8 => set_if!(self.zone_id, ctx.string(f)),
            9 => set_if!(self.stop_url, ctx.nested(f, "stop_url")),
            11 => set_if!(self.parent_station, ctx.string(f)),
            12 => set_if!(self.stop_timezone, ctx.string(f)),
            13 => set_if!(self.wheelchair_boarding, ctx.enum_value(f)),
            14 => set_if!(self.level_id, ctx.string(f)),
            15 => set_if!(self.platform_code, ctx.nested(f, "platform_code")),
            _ => return false,
        }
        true
    }
}
