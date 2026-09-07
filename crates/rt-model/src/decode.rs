//! Decode altyapısı: wire alanlarını şema mesajlarına bağlayan ortak makine.
//!
//! Her mesaj tipi yalnızca `merge_field` yazar — "şu alan numarası bende şuna
//! karşılık gelir". Bilinmeyen alan tespiti, wire type denetimi, `required`
//! kontrolü, derinlik kapağı, yol (path) takibi ve anomali üretimi burada, **tek
//! yerde** yaşar.
//!
//! Bu tekillik bilinçli: `gtfs-analyzer`'da notice üretimi altı katmana kopyalanmıştı
//! ve kopyalardan biri sessizce sapmıştı (id önekini atlıyordu). Aynı hatayı baştan
//! yapmamak için burada tek giriş noktası var.

use crate::anomaly::{Anomaly, AnomalyKind};
use crate::wire::{wire_type, Field, WireReader, WireValue};

/// İç içe mesaj derinliği kapağı.
///
/// ⚠️ **Bugün ulaşılamaz — ve bu ölçülmüştür.** GTFS-Realtime şemasında kendine dönen
/// mesaj yoktur; decoder yalnızca tanıdığı alanlar için iç mesaja indiği için saldırgan
/// bir payload derinliği keyfi olarak artıramaz (sarma denemesi ikinci katmanda
/// "bilinmeyen alan"a çarpar). En derin meşru yol **5** seviyedir:
/// `FeedMessage → entity → alert → informed_entity → trip → modified_trip`.
///
/// Kapak yine de duruyor, çünkü ucuz ve şema ileride döngüsel bir alan kazanırsa
/// (ör. iç içe `TripModifications`) tek savunma o olur. `schema_has_no_cycles_...`
/// testi, bu varsayım bozulduğunda uyarır.
pub const DEFAULT_MAX_DEPTH: u32 = 32;

/// Uzantı alan aralıkları — `gtfs-realtime.proto` her mesajda bu ikisini ayırır.
const EXTENSION_RANGES: [(u32, u32); 2] = [(1000, 1999), (9000, 9999)];

fn in_extension_range(field: u32) -> bool {
    EXTENSION_RANGES.iter().any(|&(lo, hi)| field >= lo && field <= hi)
}

/// Bir mesaj tipinin decode sözleşmesi.
pub trait Message: Default {
    /// Anomali yollarında ve hata metinlerinde kullanılan sabit ad.
    const NAME: &'static str;

    /// Tanınan bir alanı mesaja işler.
    ///
    /// `false` dönerse alan bu şemada tanımlı değildir; çağıran `UnknownField`
    /// ya da `ExtensionField` anomalisi üretir.
    fn merge_field(&mut self, field: &Field<'_>, ctx: &mut DecodeCtx) -> bool;

    /// proto2 `required` alanlarını denetler. Varsayılan: hiçbir zorunluluk yok.
    fn check_required(&self, _ctx: &mut DecodeCtx) {}
}

/// Decode boyunca taşınan durum: anomali listesi, yol yığını, derinlik.
pub struct DecodeCtx {
    anomalies: Vec<Anomaly>,
    /// Yol parçaları; yalnızca anomali üretilirken birleştirilir.
    path: Vec<String>,
    depth: u32,
    max_depth: u32,
    /// İç içe mesajlarda, slice'ın kök payload içindeki başlangıcı.
    /// Anomali offset'leri her zaman **kök payload**'a görelidir.
    offset_base: usize,
}

impl Default for DecodeCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl DecodeCtx {
    pub fn new() -> Self {
        Self {
            anomalies: Vec::new(),
            path: Vec::new(),
            depth: 0,
            max_depth: DEFAULT_MAX_DEPTH,
            offset_base: 0,
        }
    }

    /// Derinlik kapağını değiştirir (fuzz ve test için).
    pub fn with_max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Toplanan anomaliler; decode sırasına göre, bayt sırasıyla.
    pub fn anomalies(&self) -> &[Anomaly] {
        &self.anomalies
    }

    pub fn into_anomalies(self) -> Vec<Anomaly> {
        self.anomalies
    }

    /// Anomali kaydeder. Offset, kök payload'a göre normalize edilir.
    pub fn report(&mut self, kind: AnomalyKind, local_offset: usize) {
        let path = self.path.join(".");
        self.anomalies.push(Anomaly::new(kind, path, self.offset_base + local_offset));
    }

    fn push_path(&mut self, seg: impl Into<String>) {
        self.path.push(seg.into());
    }

    fn pop_path(&mut self) {
        self.path.pop();
    }

    // ── Skaler okuyucular ────────────────────────────────────────────────────
    //
    // Hepsi aynı deseni izler: wire type beklenenle uyuşmazsa anomali üret ve
    // `None` dön. Uyuşmazlıkta değeri "kurtarmaya" çalışmıyoruz — yanlış tipte
    // gelen bir alan, üreticide gerçek bir hata demektir ve sessizce yorumlamak
    // o hatayı gizler.

    pub fn u64(&mut self, f: &Field<'_>) -> Option<u64> {
        match f.value {
            WireValue::Varint(v) => Some(v),
            _ => {
                self.wrong_type(f, wire_type::VARINT);
                None
            }
        }
    }

    pub fn u32(&mut self, f: &Field<'_>) -> Option<u32> {
        self.u64(f).map(|v| v as u32)
    }

    pub fn i32(&mut self, f: &Field<'_>) -> Option<i32> {
        self.u64(f).map(crate::wire::as_i32)
    }

    pub fn i64(&mut self, f: &Field<'_>) -> Option<i64> {
        self.u64(f).map(|v| v as i64)
    }

    pub fn bool(&mut self, f: &Field<'_>) -> Option<bool> {
        self.u64(f).map(crate::wire::as_bool)
    }

    pub fn f32(&mut self, f: &Field<'_>) -> Option<f32> {
        match f.value {
            WireValue::Fixed32(v) => Some(crate::wire::as_f32(v)),
            _ => {
                self.wrong_type(f, wire_type::FIXED32);
                None
            }
        }
    }

    pub fn f64(&mut self, f: &Field<'_>) -> Option<f64> {
        match f.value {
            WireValue::Fixed64(v) => Some(crate::wire::as_f64(v)),
            _ => {
                self.wrong_type(f, wire_type::FIXED64);
                None
            }
        }
    }

    /// UTF-8 string. Geçersiz baytlar anomali üretir ve alan düşer.
    pub fn string(&mut self, f: &Field<'_>) -> Option<String> {
        let bytes = self.bytes(f)?;
        match core::str::from_utf8(bytes) {
            Ok(s) => Some(s.to_owned()),
            Err(_) => {
                self.report(AnomalyKind::InvalidUtf8, f.offset);
                None
            }
        }
    }

    pub fn bytes<'a>(&mut self, f: &Field<'a>) -> Option<&'a [u8]> {
        match f.value {
            WireValue::Len(b) => Some(b),
            _ => {
                self.wrong_type(f, wire_type::LEN);
                None
            }
        }
    }

    /// Enum alanı. Şemada olmayan sayısal değer anomali üretir ve alan düşer;
    /// ham değer kaybolmaz çünkü çağıran isterse `u64` ile de okuyabilir.
    pub fn enum_value<E: ProtoEnum>(&mut self, f: &Field<'_>) -> Option<E> {
        let raw = self.u64(f)?;
        let signed = raw as i64;
        match E::from_i32(signed as i32) {
            Some(v) => Some(v),
            None => {
                self.report(
                    AnomalyKind::UnknownEnumValue { enum_name: E::NAME, value: signed },
                    f.offset,
                );
                None
            }
        }
    }

    /// İç içe mesaj. Derinlik kapağı burada uygulanır.
    pub fn nested<M: Message>(&mut self, f: &Field<'_>, field_name: &str) -> Option<M> {
        let bytes = self.bytes(f)?;
        // `bytes` slice'ının kök payload içindeki konumu: alanın tag'inden sonra
        // varint uzunluk gelir, gövde onun ardındandır. Doğrudan hesaplamak yerine
        // slice'ın adresinden türetmek kırılgan olurdu; tag offset'i yeterince
        // yakın bir çapa ve her anomalide aynı referansı verir.
        let base = self.offset_base + f.offset;
        self.push_path(field_name);
        let out = self.decode_nested::<M>(bytes, base);
        self.pop_path();
        out
    }

    fn decode_nested<M: Message>(&mut self, bytes: &[u8], base: usize) -> Option<M> {
        if self.depth + 1 > self.max_depth {
            self.report(AnomalyKind::DepthLimitExceeded { limit: self.max_depth }, 0);
            return None;
        }
        let saved_base = self.offset_base;
        self.offset_base = base;
        self.depth += 1;

        let msg = decode_body::<M>(bytes, self);

        self.depth -= 1;
        self.offset_base = saved_base;
        Some(msg)
    }

    /// Tekrarlı (`repeated`) iç içe mesaj; yol parçasına indeks ekler.
    pub fn nested_repeated<M: Message>(
        &mut self,
        f: &Field<'_>,
        field_name: &str,
        index: usize,
    ) -> Option<M> {
        let bytes = self.bytes(f)?;
        let base = self.offset_base + f.offset;
        self.push_path(format!("{field_name}[{index}]"));
        let out = self.decode_nested::<M>(bytes, base);
        self.pop_path();
        out
    }

    /// `required` alan eksikliğini bildirir.
    pub fn missing_required(&mut self, field: &'static str) {
        self.report(AnomalyKind::MissingRequiredField { field }, 0);
    }

    fn wrong_type(&mut self, f: &Field<'_>, expected: u8) {
        self.report(
            AnomalyKind::UnexpectedWireType { expected, found: f.wire_type },
            f.offset,
        );
    }
}

/// Sayısal değerden enum varyantına çeviren tip.
pub trait ProtoEnum: Sized {
    const NAME: &'static str;
    fn from_i32(value: i32) -> Option<Self>;
}

/// Bir mesaj gövdesini (uzunluk öneki olmadan, saf alanlar dizisi) çözer.
///
/// Wire bozukluğunda okuyucu durur; o ana kadar çözülmüş alanlar korunur. Bu,
/// "kısmi sonuç" sözleşmesinin uygulandığı yer.
pub fn decode_body<M: Message>(bytes: &[u8], ctx: &mut DecodeCtx) -> M {
    let mut msg = M::default();
    let mut reader = WireReader::new(bytes);

    while let Some(next) = reader.next_field() {
        match next {
            Ok(field) => {
                if !msg.merge_field(&field, ctx) {
                    let kind = if in_extension_range(field.number) {
                        AnomalyKind::ExtensionField { field: field.number }
                    } else {
                        AnomalyKind::UnknownField { field: field.number }
                    };
                    ctx.report(kind, field.offset);
                }
            }
            Err(fault) => {
                ctx.report(fault.kind, fault.offset);
                // Fault sonrası okunamayan baytları ayrıca bildir: "payload bozuk"
                // ile "payload'ın son 3 KB'ı bozuk" farklı teşhislerdir.
                let left = reader.remaining();
                if left > 0 {
                    ctx.report(AnomalyKind::TrailingGarbage { remaining: left }, reader.position());
                }
                break;
            }
        }
    }

    msg.check_required(ctx);
    msg
}
