//! `Alert` — servis uyarısı.

use crate::decode::{DecodeCtx, Message};
use crate::model::descriptor::{EntitySelector, TimeRange, TranslatedImage, TranslatedString};
use crate::model::enums::{AlertCause, AlertEffect, AlertSeverityLevel};
use crate::model::macros::{push_if, set_if};
use crate::wire::Field;

/// Ağın bir bölümünü etkileyen uyarı.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Alert {
    /// ⚠️ Spec'te `deprecated` — yerine `communication_period` / `impact_period`.
    /// Yine de yaygın kullanımda olduğu için çözülür; kullanımı kural katmanının konusu.
    pub active_period: Vec<TimeRange>,
    pub communication_period: Vec<TimeRange>,
    pub impact_period: Vec<TimeRange>,
    pub informed_entity: Vec<EntitySelector>,
    pub cause: Option<AlertCause>,
    pub effect: Option<AlertEffect>,
    pub url: Option<TranslatedString>,
    pub header_text: Option<TranslatedString>,
    pub description_text: Option<TranslatedString>,
    pub tts_header_text: Option<TranslatedString>,
    pub tts_description_text: Option<TranslatedString>,
    pub severity_level: Option<AlertSeverityLevel>,
    pub image: Option<TranslatedImage>,
    pub image_alternative_text: Option<TranslatedString>,
    pub cause_detail: Option<TranslatedString>,
    pub effect_detail: Option<TranslatedString>,
}

impl Message for Alert {
    const NAME: &'static str = "Alert";
    fn merge_field(&mut self, f: &Field<'_>, ctx: &mut DecodeCtx) -> bool {
        match f.number {
            1 => {
                let i = self.active_period.len();
                push_if!(
                    self.active_period,
                    ctx.nested_repeated(f, "active_period", i)
                );
            }
            2 => {
                let i = self.communication_period.len();
                push_if!(
                    self.communication_period,
                    ctx.nested_repeated(f, "communication_period", i)
                );
            }
            3 => {
                let i = self.impact_period.len();
                push_if!(
                    self.impact_period,
                    ctx.nested_repeated(f, "impact_period", i)
                );
            }
            5 => {
                let i = self.informed_entity.len();
                push_if!(
                    self.informed_entity,
                    ctx.nested_repeated(f, "informed_entity", i)
                );
            }
            6 => set_if!(self.cause, ctx.enum_value(f)),
            7 => set_if!(self.effect, ctx.enum_value(f)),
            8 => set_if!(self.url, ctx.nested(f, "url")),
            10 => set_if!(self.header_text, ctx.nested(f, "header_text")),
            11 => set_if!(self.description_text, ctx.nested(f, "description_text")),
            12 => set_if!(self.tts_header_text, ctx.nested(f, "tts_header_text")),
            13 => set_if!(
                self.tts_description_text,
                ctx.nested(f, "tts_description_text")
            ),
            14 => set_if!(self.severity_level, ctx.enum_value(f)),
            15 => set_if!(self.image, ctx.nested(f, "image")),
            16 => set_if!(
                self.image_alternative_text,
                ctx.nested(f, "image_alternative_text")
            ),
            17 => set_if!(self.cause_detail, ctx.nested(f, "cause_detail")),
            18 => set_if!(self.effect_detail, ctx.nested(f, "effect_detail")),
            _ => return false,
        }
        true
    }
}
