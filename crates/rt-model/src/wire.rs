//! Protobuf wire-format okuyucu — şemadan bağımsız.
//!
//! Bu katman `gtfs-realtime.proto` hakkında hiçbir şey bilmez; yalnızca baytları
//! `(alan numarası, wire type, ham değer)` üçlülerine ayırır. Şema yorumu `decode`
//! katmanının işidir.
//!
//! Tasarım kuralı: **hiçbir bozukluk sessizce yutulmaz.** Tolerant protobuf
//! kütüphaneleri bilinmeyen alanı atlar, yanlış wire type'ı görmezden gelir ve
//! artakalan baytları yok sayar; bu decoder'ın varlık sebebi tam da bunları
//! görünür kılmaktır.

use crate::anomaly::AnomalyKind;

/// Bir varint en fazla 10 bayt olabilir (64 bit / 7 bit-per-byte, yukarı yuvarlanmış).
pub const MAX_VARINT_BYTES: usize = 10;

/// Protobuf wire type kodları.
pub mod wire_type {
    pub const VARINT: u8 = 0;
    pub const FIXED64: u8 = 1;
    pub const LEN: u8 = 2;
    pub const START_GROUP: u8 = 3;
    pub const END_GROUP: u8 = 4;
    pub const FIXED32: u8 = 5;
}

/// Wire okuyucusunu durduran bozukluk.
///
/// Fault üretildiğinde okuyucu **durur** — kalan baytlar güvenilmez sayılır ve
/// ilerleme garantisi kaybolduğu için devam etmek sonsuz döngü riski taşır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireFault {
    pub kind: AnomalyKind,
    pub offset: usize,
}

/// Tek bir wire alanının ham hâli.
#[derive(Debug, Clone, PartialEq)]
pub struct Field<'a> {
    pub number: u32,
    pub wire_type: u8,
    pub value: WireValue<'a>,
    /// Alanın tag baytının payload içindeki konumu.
    pub offset: usize,
}

/// Wire type'a göre çözülmüş ham değer.
#[derive(Debug, Clone, PartialEq)]
pub enum WireValue<'a> {
    Varint(u64),
    Fixed64(u64),
    Len(&'a [u8]),
    Fixed32(u32),
    /// Group başlangıç/bitiş etiketi (proto2, kullanımdan kalkmış). Gövde okunmaz.
    Group,
}

impl WireValue<'_> {
    /// Bu değerin wire type kodu.
    pub fn wire_type(&self) -> u8 {
        match self {
            WireValue::Varint(_) => wire_type::VARINT,
            WireValue::Fixed64(_) => wire_type::FIXED64,
            WireValue::Len(_) => wire_type::LEN,
            WireValue::Fixed32(_) => wire_type::FIXED32,
            WireValue::Group => wire_type::START_GROUP,
        }
    }
}

/// Bayt dilimi üzerinde alan alan ilerleyen okuyucu.
pub struct WireReader<'a> {
    buf: &'a [u8],
    pos: usize,
    stopped: bool,
}

impl<'a> WireReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0, stopped: false }
    }

    /// Okunmamış bayt sayısı.
    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    /// Şu anki bayt konumu.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Okuyucu bir bozukluk yüzünden durdu mu?
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// Sıradaki alanı okur.
    ///
    /// `None` → temiz bitiş. `Some(Err(_))` → bozukluk; okuyucu durur ve sonraki
    /// çağrılar `None` döner.
    pub fn next_field(&mut self) -> Option<Result<Field<'a>, WireFault>> {
        if self.stopped || self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        match self.read_field(start) {
            Ok(field) => {
                // İlerleme güvencesi: her başarılı alan en az bir bayt tüketmeli,
                // aksi halde çağıran sonsuz döngüye girer.
                debug_assert!(self.pos > start, "wire reader ilerlemedi");
                Some(Ok(field))
            }
            Err(fault) => {
                self.stopped = true;
                Some(Err(fault))
            }
        }
    }

    fn read_field(&mut self, start: usize) -> Result<Field<'a>, WireFault> {
        let tag = self.read_varint(start)?;
        let number = (tag >> 3) as u32;
        let wt = (tag & 0x07) as u8;

        if number == 0 {
            return Err(WireFault { kind: AnomalyKind::ZeroFieldNumber, offset: start });
        }

        let value = match wt {
            wire_type::VARINT => WireValue::Varint(self.read_varint(start)?),
            wire_type::FIXED64 => WireValue::Fixed64(self.read_fixed::<8>(start)?),
            wire_type::FIXED32 => WireValue::Fixed32(self.read_fixed::<4>(start)? as u32),
            wire_type::LEN => {
                let len = self.read_varint(start)? as usize;
                let remaining = self.remaining();
                if len > remaining {
                    return Err(WireFault {
                        kind: AnomalyKind::LengthExceedsRemaining { declared: len, remaining },
                        offset: start,
                    });
                }
                let slice = &self.buf[self.pos..self.pos + len];
                self.pos += len;
                WireValue::Len(slice)
            }
            wire_type::START_GROUP | wire_type::END_GROUP => {
                // GTFS-Realtime group kullanmaz. Gövdesini atlamaya çalışmıyoruz:
                // group'lar iç içe olabilir ve doğru atlamak, hiç kullanılmayan bir
                // kodlama için gereksiz karmaşıklık. Okuyucu burada durur.
                return Err(WireFault { kind: AnomalyKind::DeprecatedGroup, offset: start });
            }
            other => {
                return Err(WireFault {
                    kind: AnomalyKind::ReservedWireType { found: other },
                    offset: start,
                });
            }
        };

        Ok(Field { number, wire_type: wt, value, offset: start })
    }

    fn read_varint(&mut self, field_start: usize) -> Result<u64, WireFault> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        let mut consumed = 0usize;

        loop {
            if self.pos >= self.buf.len() {
                return Err(WireFault { kind: AnomalyKind::Truncated, offset: field_start });
            }
            if consumed == MAX_VARINT_BYTES {
                return Err(WireFault { kind: AnomalyKind::VarintTooLong, offset: field_start });
            }
            let byte = self.buf[self.pos];
            self.pos += 1;
            consumed += 1;

            // 10. bayt yalnızca en üst biti taşıyabilir; `shift` 63'ü aşmaz.
            result |= u64::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
    }

    fn read_fixed<const N: usize>(&mut self, field_start: usize) -> Result<u64, WireFault> {
        if self.remaining() < N {
            return Err(WireFault { kind: AnomalyKind::Truncated, offset: field_start });
        }
        let mut bytes = [0u8; 8];
        bytes[..N].copy_from_slice(&self.buf[self.pos..self.pos + N]);
        self.pos += N;
        Ok(u64::from_le_bytes(bytes))
    }
}

// ── Değer dönüşüm yardımcıları ───────────────────────────────────────────────

/// Varint'i `bool` olarak yorumlar. Protobuf'ta sıfır dışı her değer `true`.
pub fn as_bool(v: u64) -> bool {
    v != 0
}

/// Varint'i `int32` olarak yorumlar (negatifler 64-bit'e genişletilmiş hâlde gelir).
pub fn as_i32(v: u64) -> i32 {
    v as u32 as i32
}

/// `Fixed32`'yi IEEE-754 `float` olarak yorumlar.
pub fn as_f32(v: u32) -> f32 {
    f32::from_bits(v)
}

/// `Fixed64`'ü IEEE-754 `double` olarak yorumlar.
pub fn as_f64(v: u64) -> f64 {
    f64::from_bits(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tag baytı: `(alan_no << 3) | wire_type`.
    fn tag(number: u32, wt: u8) -> u8 {
        ((number << 3) as u8) | wt
    }

    fn fields(buf: &[u8]) -> Vec<Result<Field<'_>, WireFault>> {
        let mut r = WireReader::new(buf);
        let mut out = Vec::new();
        while let Some(f) = r.next_field() {
            out.push(f);
        }
        out
    }

    #[test]
    fn reads_varint_field() {
        // field 1, varint 150 (0x96 0x01)
        let buf = [tag(1, wire_type::VARINT), 0x96, 0x01];
        let got = fields(&buf);
        assert_eq!(got.len(), 1);
        let f = got[0].as_ref().unwrap();
        assert_eq!(f.number, 1);
        assert_eq!(f.value, WireValue::Varint(150));
        assert_eq!(f.offset, 0);
    }

    #[test]
    fn reads_length_delimited_field() {
        let mut buf = vec![tag(2, wire_type::LEN), 3];
        buf.extend_from_slice(b"abc");
        let got = fields(&buf);
        let f = got[0].as_ref().unwrap();
        assert_eq!(f.number, 2);
        assert_eq!(f.value, WireValue::Len(b"abc"));
    }

    #[test]
    fn reads_fixed32_and_fixed64() {
        let mut buf = vec![tag(3, wire_type::FIXED32)];
        buf.extend_from_slice(&1.5f32.to_bits().to_le_bytes());
        buf.push(tag(4, wire_type::FIXED64));
        buf.extend_from_slice(&2.5f64.to_bits().to_le_bytes());

        let got = fields(&buf);
        assert_eq!(got.len(), 2);
        match got[0].as_ref().unwrap().value {
            WireValue::Fixed32(v) => assert_eq!(as_f32(v), 1.5),
            ref other => panic!("beklenmeyen: {other:?}"),
        }
        match got[1].as_ref().unwrap().value {
            WireValue::Fixed64(v) => assert_eq!(as_f64(v), 2.5),
            ref other => panic!("beklenmeyen: {other:?}"),
        }
    }

    #[test]
    fn reads_several_fields_in_order() {
        let mut buf = vec![tag(1, wire_type::VARINT), 1, tag(2, wire_type::LEN), 2];
        buf.extend_from_slice(b"hi");
        let got = fields(&buf);
        let numbers: Vec<u32> = got.iter().map(|f| f.as_ref().unwrap().number).collect();
        assert_eq!(numbers, vec![1, 2]);
    }

    // ── bozukluklar ──────────────────────────────────────────────────────────

    #[test]
    fn truncated_varint_is_reported() {
        // continuation biti set ama devamı yok
        let buf = [tag(1, wire_type::VARINT), 0x80];
        let got = fields(&buf);
        assert_eq!(got[0].as_ref().unwrap_err().kind, AnomalyKind::Truncated);
    }

    #[test]
    fn truncated_fixed_is_reported() {
        let buf = [tag(3, wire_type::FIXED32), 0x01, 0x02];
        let got = fields(&buf);
        assert_eq!(got[0].as_ref().unwrap_err().kind, AnomalyKind::Truncated);
    }

    #[test]
    fn varint_longer_than_ten_bytes_is_reported() {
        let mut buf = vec![tag(1, wire_type::VARINT)];
        buf.extend(std::iter::repeat_n(0xFF, MAX_VARINT_BYTES));
        buf.push(0x01);
        let got = fields(&buf);
        assert_eq!(got[0].as_ref().unwrap_err().kind, AnomalyKind::VarintTooLong);
    }

    #[test]
    fn ten_byte_varint_is_accepted() {
        // u64::MAX = 10 baytlık varint; sınırın kendisi geçerli olmalı.
        let mut buf = vec![tag(1, wire_type::VARINT)];
        buf.extend(std::iter::repeat_n(0xFF, MAX_VARINT_BYTES - 1));
        buf.push(0x01);
        let got = fields(&buf);
        assert_eq!(got[0].as_ref().unwrap().value, WireValue::Varint(u64::MAX));
    }

    #[test]
    fn zero_field_number_is_reported() {
        let buf = [0x00, 0x00];
        let got = fields(&buf);
        assert_eq!(got[0].as_ref().unwrap_err().kind, AnomalyKind::ZeroFieldNumber);
    }

    #[test]
    fn reserved_wire_types_are_reported() {
        for wt in [6u8, 7u8] {
            let buf = [tag(1, wt)];
            let got = fields(&buf);
            assert_eq!(
                got[0].as_ref().unwrap_err().kind,
                AnomalyKind::ReservedWireType { found: wt },
                "wire type {wt}"
            );
        }
    }

    #[test]
    fn group_wire_types_are_reported() {
        for wt in [wire_type::START_GROUP, wire_type::END_GROUP] {
            let buf = [tag(1, wt)];
            let got = fields(&buf);
            assert_eq!(got[0].as_ref().unwrap_err().kind, AnomalyKind::DeprecatedGroup);
        }
    }

    #[test]
    fn length_exceeding_buffer_is_reported() {
        // 9 bayt vaat ediyor, 2 bayt var
        let buf = [tag(2, wire_type::LEN), 9, 0x61, 0x62];
        let got = fields(&buf);
        assert_eq!(
            got[0].as_ref().unwrap_err().kind,
            AnomalyKind::LengthExceedsRemaining { declared: 9, remaining: 2 }
        );
    }

    #[test]
    fn zero_length_field_is_valid() {
        let buf = [tag(2, wire_type::LEN), 0];
        let got = fields(&buf);
        assert_eq!(got[0].as_ref().unwrap().value, WireValue::Len(b""));
    }

    // ── okuyucu sözleşmeleri ─────────────────────────────────────────────────

    #[test]
    fn reader_stops_after_fault() {
        // Bozuk alandan sonra geçerli bir alan gelse bile okunmamalı: fault sonrası
        // hizalama güvenilmez, devam etmek uydurma alan üretir.
        let buf = [tag(1, 7), tag(1, wire_type::VARINT), 1];
        let mut r = WireReader::new(&buf);
        assert!(r.next_field().unwrap().is_err());
        assert!(r.next_field().is_none());
        assert!(r.is_stopped());
    }

    #[test]
    fn empty_input_yields_nothing() {
        let mut r = WireReader::new(&[]);
        assert!(r.next_field().is_none());
        assert!(!r.is_stopped());
    }

    #[test]
    fn reader_always_advances_or_stops() {
        // Fuzz güvencesinin birim testi: her girdi için döngü sonlanmalı.
        // 3 baytlık tüm diziler (16.7M değil, 24 bitlik uzay = 16.7M çok; 2 bayt = 65k)
        for a in 0u16..=0xFF {
            for b in 0u16..=0xFF {
                let buf = [a as u8, b as u8];
                let mut r = WireReader::new(&buf);
                let mut guard = 0;
                while r.next_field().is_some() {
                    guard += 1;
                    assert!(guard <= buf.len(), "okuyucu ilerlemedi: {buf:?}");
                }
            }
        }
    }

    #[test]
    fn position_and_remaining_track_progress() {
        let buf = [tag(1, wire_type::VARINT), 1, tag(1, wire_type::VARINT), 2];
        let mut r = WireReader::new(&buf);
        assert_eq!(r.remaining(), 4);
        r.next_field().unwrap().unwrap();
        assert_eq!(r.position(), 2);
        assert_eq!(r.remaining(), 2);
        r.next_field().unwrap().unwrap();
        assert_eq!(r.remaining(), 0);
        assert!(r.next_field().is_none());
    }
}
