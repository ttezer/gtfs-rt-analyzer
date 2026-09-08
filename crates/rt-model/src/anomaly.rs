//! Decode sırasında toplanan yapısal gözlemler.
//!
//! Bunlar **kural bulgusu (notice) DEĞİLDİR** — bu crate'te kural katmanı yoktur.
//! Burada yalnızca "baytlarda ne gördük" yazar; bunun ihlal olup olmadığına, hangi
//! severity'yi hak ettiğine ve nasıl raporlanacağına üst katman karar verir.
//!
//! Determinizm sözleşmesi: anomaliler decode sırasında, bayt sırasına göre toplanır.
//! Hiçbir yerde HashMap iterasyonu ya da paralellik yoktur, dolayısıyla aynı girdi
//! her koşumda aynı listeyi aynı sırada üretir. (`gtfs-analyzer` bu dersi pahalıya
//! öğrendi: notice id'leri emisyon sırasından geliyordu, o da HashMap'e bağlıydı.)

use core::fmt;

/// Tek bir yapısal gözlem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anomaly {
    pub kind: AnomalyKind,
    /// Proto alan yolu, ör. `entity[12].trip_update.trip.trip_id`.
    /// Kök mesaj için boş string.
    pub path: String,
    /// Gözlemin başladığı bayt konumu (payload başından itibaren).
    pub offset: usize,
}

impl Anomaly {
    pub fn new(kind: AnomalyKind, path: impl Into<String>, offset: usize) -> Self {
        Self {
            kind,
            path: path.into(),
            offset,
        }
    }
}

impl fmt::Display for Anomaly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "@{}: {}", self.offset, self.kind)
        } else {
            write!(f, "@{} {}: {}", self.offset, self.path, self.kind)
        }
    }
}

/// Gözlem türü.
///
/// Üç aile: **payload** düzeyi (içerik protobuf değil), **wire** düzeyi
/// (protobuf çerçevelemesi bozuk) ve **şema** düzeyi (çerçeveleme sağlam ama
/// GTFS-Realtime şemasıyla uyuşmuyor).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnomalyKind {
    // ── wire düzeyi ──────────────────────────────────────────────────────────
    /// Payload alanın ortasında bitti.
    Truncated,
    /// Kök mesaj bittikten sonra artakalan baytlar var.
    TrailingGarbage { remaining: usize },
    /// Varint 10 baytı aştı — hiçbir u64 bu kadar uzun kodlanamaz.
    VarintTooLong,
    /// Alan bu şemada başka bir wire type ile bekleniyordu.
    UnexpectedWireType { expected: u8, found: u8 },
    /// proto2 group kodlaması (wire type 3/4). GTFS-Realtime hiç kullanmaz.
    DeprecatedGroup,
    /// Protobuf'ta tanımsız wire type (6 veya 7).
    ReservedWireType { found: u8 },
    /// Length-delimited alanın uzunluğu kalan baytları aşıyor.
    LengthExceedsRemaining { declared: usize, remaining: usize },
    /// Alan numarası 0 — protobuf'ta geçersiz.
    ZeroFieldNumber,
    /// İç içe mesaj derinliği kapağı aşıldı.
    DepthLimitExceeded { limit: u32 },
    /// Payload protobuf yerine yaygın bir HTTP hata/yanıt gövdesine benziyor.
    ///
    /// Decoder'ın üreteceği daha düşük seviyeli wire belirtilerinden önce
    /// gösterilmesi gereken bir payload teşhisidir.
    NotProtobuf { looks_like: &'static str },

    // ── şema düzeyi ──────────────────────────────────────────────────────────
    /// Bu mesajda tanımlı olmayan alan numarası.
    UnknownField { field: u32 },
    /// Tanınmayan alan, ama uzantı aralığında (1000-1999 / 9000-9999).
    /// Uzantı semantiği bu crate'in kapsamı dışında; yalnızca sayılır.
    ExtensionField { field: u32 },
    /// proto2 `required` alan yok.
    MissingRequiredField { field: &'static str },
    /// `FeedEntity` birden fazla payload taşıyor (spec: en fazla biri).
    MultiplePayloads { count: usize },
    /// `FeedEntity` hiç payload taşımıyor.
    NoPayload,
    /// Enum alanında şemada olmayan sayısal değer.
    UnknownEnumValue { enum_name: &'static str, value: i64 },
    /// String alanı geçerli UTF-8 değil.
    InvalidUtf8,
}

impl fmt::Display for AnomalyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AnomalyKind::*;
        match self {
            Truncated => write!(f, "payload truncated"),
            TrailingGarbage { remaining } => {
                write!(f, "{remaining} trailing byte(s) after root message")
            }
            VarintTooLong => write!(f, "varint longer than 10 bytes"),
            UnexpectedWireType { expected, found } => {
                write!(f, "wire type {found}, expected {expected}")
            }
            DeprecatedGroup => write!(f, "deprecated group wire type (3/4)"),
            ReservedWireType { found } => write!(f, "reserved wire type {found}"),
            LengthExceedsRemaining {
                declared,
                remaining,
            } => {
                write!(
                    f,
                    "declared length {declared} exceeds {remaining} remaining byte(s)"
                )
            }
            ZeroFieldNumber => write!(f, "field number 0 is invalid"),
            DepthLimitExceeded { limit } => write!(f, "nesting deeper than {limit}"),
            NotProtobuf { looks_like } => {
                write!(f, "payload is not protobuf (looks like {looks_like})")
            }
            UnknownField { field } => write!(f, "unknown field number {field}"),
            ExtensionField { field } => write!(f, "extension field number {field} (not decoded)"),
            MissingRequiredField { field } => write!(f, "required field '{field}' is missing"),
            MultiplePayloads { count } => {
                write!(f, "FeedEntity carries {count} payloads, expected at most 1")
            }
            NoPayload => write!(f, "FeedEntity carries no payload"),
            UnknownEnumValue { enum_name, value } => {
                write!(f, "value {value} is not a known {enum_name}")
            }
            InvalidUtf8 => write!(f, "string field is not valid UTF-8"),
        }
    }
}

impl AnomalyKind {
    /// Gözlemin wire çerçevelemesine mi yoksa şemaya mı ait olduğu.
    pub fn is_wire_level(&self) -> bool {
        use AnomalyKind::*;
        matches!(
            self,
            Truncated
                | TrailingGarbage { .. }
                | VarintTooLong
                | UnexpectedWireType { .. }
                | DeprecatedGroup
                | ReservedWireType { .. }
                | LengthExceedsRemaining { .. }
                | ZeroFieldNumber
                | DepthLimitExceeded { .. }
        )
    }

    /// Girdi protobuf yerine yaygın bir metin hata/yanıt gövdesine mi benziyor?
    pub fn is_payload_level(&self) -> bool {
        matches!(self, Self::NotProtobuf { .. })
    }

    /// Sabit, yeniden adlandırmaya dayanıklı kısa ad. Dedup/gruplama anahtarı olarak
    /// kullanılabilir; `Debug` çıktısının aksine varyant adı değişse bile sabit kalır.
    pub fn stable_name(&self) -> &'static str {
        use AnomalyKind::*;
        match self {
            Truncated => "truncated",
            TrailingGarbage { .. } => "trailing_garbage",
            VarintTooLong => "varint_too_long",
            UnexpectedWireType { .. } => "unexpected_wire_type",
            DeprecatedGroup => "deprecated_group",
            ReservedWireType { .. } => "reserved_wire_type",
            LengthExceedsRemaining { .. } => "length_exceeds_remaining",
            ZeroFieldNumber => "zero_field_number",
            DepthLimitExceeded { .. } => "depth_limit_exceeded",
            NotProtobuf { .. } => "not_protobuf",
            UnknownField { .. } => "unknown_field",
            ExtensionField { .. } => "extension_field",
            MissingRequiredField { .. } => "missing_required_field",
            MultiplePayloads { .. } => "multiple_payloads",
            NoPayload => "no_payload",
            UnknownEnumValue { .. } => "unknown_enum_value",
            InvalidUtf8 => "invalid_utf8",
        }
    }
}
