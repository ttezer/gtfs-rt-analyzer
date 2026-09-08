//! Testler için minik protobuf encoder.
//!
//! Fixture'lar **kod içinde** kurulur, diske binary dosya yazılmaz: bayt dizisi
//! fixture'ları diff'te okunamaz ve neyin neden değiştiği görünmez olur.
//! (`gtfs-analyzer` aynı sebeple test ZIP'lerini bellekte kuruyor.)

#![allow(dead_code)]

/// Alan alan protobuf payload'ı kuran yardımcı.
#[derive(Debug, Default, Clone)]
pub struct Enc(Vec<u8>);

impl Enc {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    fn varint(&mut self, mut v: u64) {
        loop {
            let byte = (v & 0x7F) as u8;
            v >>= 7;
            if v == 0 {
                self.0.push(byte);
                return;
            }
            self.0.push(byte | 0x80);
        }
    }

    fn tag(&mut self, number: u32, wire_type: u8) {
        self.varint((u64::from(number) << 3) | u64::from(wire_type));
    }

    pub fn varint_field(mut self, number: u32, value: u64) -> Self {
        self.tag(number, 0);
        self.varint(value);
        self
    }

    pub fn i32_field(self, number: u32, value: i32) -> Self {
        // Negatif int32 protobuf'ta 64-bit'e işaret genişletilerek kodlanır.
        self.varint_field(number, value as i64 as u64)
    }

    pub fn bytes_field(mut self, number: u32, value: &[u8]) -> Self {
        self.tag(number, 2);
        self.varint(value.len() as u64);
        self.0.extend_from_slice(value);
        self
    }

    pub fn string_field(self, number: u32, value: &str) -> Self {
        self.bytes_field(number, value.as_bytes())
    }

    pub fn msg_field(self, number: u32, inner: Enc) -> Self {
        self.bytes_field(number, inner.as_slice())
    }

    pub fn f32_field(mut self, number: u32, value: f32) -> Self {
        self.tag(number, 5);
        self.0.extend_from_slice(&value.to_bits().to_le_bytes());
        self
    }

    pub fn f64_field(mut self, number: u32, value: f64) -> Self {
        self.tag(number, 1);
        self.0.extend_from_slice(&value.to_bits().to_le_bytes());
        self
    }

    /// Ham bayt ekler — bozuk payload kurmak için.
    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.0.extend_from_slice(bytes);
        self
    }
}

// ── Sık kullanılan geçerli parçalar ──────────────────────────────────────────

/// Geçerli bir `FeedHeader` (`gtfs_realtime_version` + timestamp).
pub fn header(timestamp: u64) -> Enc {
    Enc::new().string_field(1, "2.0").varint_field(3, timestamp)
}

/// Geçerli bir `TripUpdate` gövdesi: yalnızca `trip.trip_id`.
pub fn trip_update(trip_id: &str) -> Enc {
    Enc::new().msg_field(1, Enc::new().string_field(1, trip_id))
}

/// Tek `TripUpdate` taşıyan geçerli `FeedEntity`.
pub fn entity_with_trip_update(id: &str, trip_id: &str) -> Enc {
    Enc::new()
        .string_field(1, id)
        .msg_field(3, trip_update(trip_id))
}

/// Tek entity'li, tümüyle geçerli bir `FeedMessage`.
pub fn minimal_feed() -> Vec<u8> {
    Enc::new()
        .msg_field(1, header(1_757_000_000))
        .msg_field(2, entity_with_trip_update("e1", "T1"))
        .into_bytes()
}
