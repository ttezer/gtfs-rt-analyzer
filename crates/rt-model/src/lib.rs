//! GTFS-Realtime protobuf decoder — bağımlılıksız, anomali raporlayan.
//!
//! Bu crate bir **doğrulayıcı değildir**: kural, severity ve rapor kavramı yoktur.
//! Yaptığı tek şey, GTFS-Realtime payload'ını okumak ve okurken gördüğü yapısal
//! aykırılıkları [`anomaly::Anomaly`] listesi olarak yanına koymaktır.
//!
//! ## Neden elle yazılmış bir decoder?
//!
//! `prost`, `quick-protobuf` gibi kütüphaneler **hoşgörülüdür** — bilinmeyen alanı
//! atlar, yanlış wire type'ı görmezden gelir, proto2 `required` eksikliğini yutar,
//! artakalan baytları yok sayar. Bir RT doğrulayıcısının işinin önemli bir kısmı
//! tam olarak bu davranışların gizlediği yerde. Rust'taki mevcut iki GTFS-RT crate'i
//! (`gtfs-rt`, `gtfs-realtime`) da `prost` tabanlıdır, dolayısıyla ikisi de bu
//! bilgiyi kaybeder.
//!
//! ## Sözleşmeler
//!
//! - **Panik yok.** Hiçbir girdi — bozuk, kesik, düşmanca — panik üretmez.
//! - **Determinizm.** Aynı bayt dizisi her koşumda aynı modeli ve aynı sıradaki
//!   aynı anomali listesini verir.
//! - **Kısmi sonuç.** Bozuk payload'da bile o ana kadar çözülen kısım döndürülür;
//!   `Err` yalnızca hiçbir şey çözülemediğinde gelir.

#![forbid(unsafe_code)]

pub mod anomaly;
pub mod wire;

pub use anomaly::{Anomaly, AnomalyKind};
