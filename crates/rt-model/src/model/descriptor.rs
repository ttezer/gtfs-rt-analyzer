//! Sefer, araç ve konum tanımlayıcıları — birden çok mesajın paylaştığı yapı taşları.

use crate::decode::{DecodeCtx, Message};
use crate::model::enums::{TripScheduleRelationship, WheelchairAccessible};
use crate::model::macros::{push_if, set_if};
use crate::wire::Field;

/// Bir seferi tanımlar. `trip_id` **opsiyoneldir**: `ADDED`/`NEW` seferlerde statik
/// feed'de karşılığı yoktur, `modified_trip` ile tanımlananlarda ise başka bir
/// seferden türetilir.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TripDescriptor {
    pub trip_id: Option<String>,
    pub route_id: Option<String>,
    pub direction_id: Option<u32>,
    pub start_time: Option<String>,
    pub start_date: Option<String>,
    pub schedule_relationship: Option<TripScheduleRelationship>,
    pub modified_trip: Option<ModifiedTripSelector>,
}

impl Message for TripDescriptor {
    const NAME: &'static str = "TripDescriptor";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.trip_id, ctx.string(f)),
            2 => set_if!(self.start_time, ctx.string(f)),
            3 => set_if!(self.start_date, ctx.string(f)),
            4 => set_if!(self.schedule_relationship, ctx.enum_value(f)),
            5 => set_if!(self.route_id, ctx.string(f)),
            6 => set_if!(self.direction_id, ctx.u32(f)),
            7 => set_if!(self.modified_trip, ctx.nested(f, "modified_trip")),
            _ => return false,
        }
        true
    }
}

/// `TripDescriptor.modified_trip` — bir `TripModifications` kaydına bağlanan seçici.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModifiedTripSelector {
    pub modifications_id: Option<String>,
    pub affected_trip_id: Option<String>,
    pub start_time: Option<String>,
    pub start_date: Option<String>,
}

impl Message for ModifiedTripSelector {
    const NAME: &'static str = "TripDescriptor.ModifiedTripSelector";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.modifications_id, ctx.string(f)),
            2 => set_if!(self.affected_trip_id, ctx.string(f)),
            3 => set_if!(self.start_time, ctx.string(f)),
            4 => set_if!(self.start_date, ctx.string(f)),
            _ => return false,
        }
        true
    }
}

/// Aracı tanımlar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VehicleDescriptor {
    pub id: Option<String>,
    pub label: Option<String>,
    pub license_plate: Option<String>,
    pub wheelchair_accessible: Option<WheelchairAccessible>,
}

impl Message for VehicleDescriptor {
    const NAME: &'static str = "VehicleDescriptor";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.id, ctx.string(f)),
            2 => set_if!(self.label, ctx.string(f)),
            3 => set_if!(self.license_plate, ctx.string(f)),
            4 => set_if!(self.wheelchair_accessible, ctx.enum_value(f)),
            _ => return false,
        }
        true
    }
}

/// Coğrafi konum. `latitude` ve `longitude` proto2 `required`'dır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Position {
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub bearing: Option<f32>,
    pub odometer: Option<f64>,
    pub speed: Option<f32>,
}

impl Message for Position {
    const NAME: &'static str = "Position";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.latitude, ctx.f32(f)),
            2 => set_if!(self.longitude, ctx.f32(f)),
            3 => set_if!(self.bearing, ctx.f32(f)),
            4 => set_if!(self.odometer, ctx.f64(f)),
            5 => set_if!(self.speed, ctx.f32(f)),
            _ => return false,
        }
        true
    }

    fn check_required(&self, ctx: &mut DecodeCtx) {
        if self.latitude.is_none() {
            ctx.missing_required("Position.latitude");
        }
        if self.longitude.is_none() {
            ctx.missing_required("Position.longitude");
        }
    }
}

/// Kapalı olmayabilen zaman aralığı: `start` yoksa "her zamandan beri",
/// `end` yoksa "sonsuza kadar".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeRange {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl Message for TimeRange {
    const NAME: &'static str = "TimeRange";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.start, ctx.u64(f)),
            2 => set_if!(self.end, ctx.u64(f)),
            _ => return false,
        }
        true
    }
}

/// `Alert.informed_entity` — uyarının etkilediği varlıkları seçer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntitySelector {
    pub agency_id: Option<String>,
    pub route_id: Option<String>,
    pub route_type: Option<i32>,
    pub trip: Option<TripDescriptor>,
    pub stop_id: Option<String>,
    pub direction_id: Option<u32>,
}

impl Message for EntitySelector {
    const NAME: &'static str = "EntitySelector";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.agency_id, ctx.string(f)),
            2 => set_if!(self.route_id, ctx.string(f)),
            3 => set_if!(self.route_type, ctx.i32(f)),
            4 => set_if!(self.trip, ctx.nested(f, "trip")),
            5 => set_if!(self.stop_id, ctx.string(f)),
            6 => set_if!(self.direction_id, ctx.u32(f)),
            _ => return false,
        }
        true
    }
}

// ── Çevrilmiş metin ve görsel ────────────────────────────────────────────────

/// Dil etiketli metin kümesi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TranslatedString {
    pub translation: Vec<Translation>,
}

impl Message for TranslatedString {
    const NAME: &'static str = "TranslatedString";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => {
                let i = self.translation.len();
                push_if!(self.translation, ctx.nested_repeated(f, "translation", i));
            }
            _ => return false,
        }
        true
    }
}

/// Tek dildeki metin. `text` proto2 `required`'dır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Translation {
    pub text: Option<String>,
    pub language: Option<String>,
}

impl Message for Translation {
    const NAME: &'static str = "TranslatedString.Translation";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.text, ctx.string(f)),
            2 => set_if!(self.language, ctx.string(f)),
            _ => return false,
        }
        true
    }

    fn check_required(&self, ctx: &mut DecodeCtx) {
        if self.text.is_none() {
            ctx.missing_required("Translation.text");
        }
    }
}

/// Dil etiketli görsel kümesi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TranslatedImage {
    pub localized_image: Vec<LocalizedImage>,
}

impl Message for TranslatedImage {
    const NAME: &'static str = "TranslatedImage";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => {
                let i = self.localized_image.len();
                push_if!(
                    self.localized_image,
                    ctx.nested_repeated(f, "localized_image", i)
                );
            }
            _ => return false,
        }
        true
    }
}

/// Tek dildeki görsel. `url` ve `media_type` proto2 `required`'dır.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocalizedImage {
    pub url: Option<String>,
    pub media_type: Option<String>,
    pub language: Option<String>,
}

impl Message for LocalizedImage {
    const NAME: &'static str = "TranslatedImage.LocalizedImage";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => set_if!(self.url, ctx.string(f)),
            2 => set_if!(self.media_type, ctx.string(f)),
            3 => set_if!(self.language, ctx.string(f)),
            _ => return false,
        }
        true
    }

    fn check_required(&self, ctx: &mut DecodeCtx) {
        if self.url.is_none() {
            ctx.missing_required("LocalizedImage.url");
        }
        if self.media_type.is_none() {
            ctx.missing_required("LocalizedImage.media_type");
        }
    }
}
