//! GTFS-Realtime protobuf decoder — bağımlılıksız, anomali raporlayan.
//!
//! Bu crate bir **doğrulayıcı değildir**: kural, severity ve rapor kavramı yoktur.
//! Yaptığı tek şey, GTFS-Realtime payload'ını okumak ve okurken gördüğü yapısal
//! aykırılıkları [`Anomaly`] listesi olarak yanına koymaktır. Bunların ihlal olup
//! olmadığına üst katman karar verir.
//!
//! ## Neden elle yazılmış bir decoder?
//!
//! `prost`, `quick-protobuf` gibi kütüphaneler **hoşgörülüdür** — bilinmeyen alanı
//! atlar, yanlış wire type'ı görmezden gelir, proto2 `required` eksikliğini yutar,
//! artakalan baytları yok sayar. Bir RT doğrulayıcısının işinin önemli bir kısmı
//! tam olarak bu davranışların gizlediği yerde. Rust'taki mevcut iki GTFS-RT crate'i
//! (`gtfs-rt`, `gtfs-realtime`) da `prost` tabanlıdır, dolayısıyla ikisi de bu
//! bilgiyi kaybeder ve build zamanında `protoc` ister.
//!
//! ## Sözleşmeler
//!
//! - **Panik yok.** Hiçbir girdi — bozuk, kesik, düşmanca — panik üretmez.
//! - **Determinizm.** Aynı bayt dizisi her koşumda aynı modeli ve aynı sıradaki
//!   aynı anomali listesini verir. Hiçbir yerde HashMap iterasyonu ya da paralellik yok.
//! - **Kısmi sonuç.** Bozuk payload'da bile o ana kadar çözülen kısım döndürülür.
//!
//! ```
//! use gtfs_rt_model::decode_feed_message;
//!
//! // Boş payload: header eksik olduğu için bir anomali üretir.
//! let out = decode_feed_message(&[]);
//! assert!(out.message.header.is_none());
//! assert_eq!(out.anomalies.len(), 1);
//! ```

#![forbid(unsafe_code)]

pub mod anomaly;
pub mod decode;
pub mod model;
pub mod wire;

pub use anomaly::{Anomaly, AnomalyKind};
pub use decode::{DecodeCtx, DEFAULT_MAX_DEPTH};
pub use model::{FeedEntity, FeedHeader, FeedMessage};

/// Çözülmüş payload ve okuma sırasında görülen aykırılıklar.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedFeed {
    pub message: FeedMessage,
    /// Bayt sırasına göre, deterministik.
    pub anomalies: Vec<Anomaly>,
}

impl DecodedFeed {
    /// Hiçbir aykırılık görülmedi mi?
    pub fn is_clean(&self) -> bool {
        self.anomalies.is_empty()
    }

    /// Wire çerçevelemesine ait aykırılıklar — payload'ın protobuf olarak
    /// okunabilirliğine dair. Bunlar varsa şema düzeyi bulgular eksik kalmış olabilir.
    pub fn wire_anomalies(&self) -> impl Iterator<Item = &Anomaly> {
        self.anomalies.iter().filter(|a| a.kind.is_wire_level())
    }
}

/// Bir GTFS-Realtime payload'ını çözer.
///
/// Girdi ne olursa olsun döner — hata tipi yoktur, çünkü "çözülemedi" de bir
/// sonuçtur ve anomali listesinde anlatılır.
pub fn decode_feed_message(bytes: &[u8]) -> DecodedFeed {
    decode_feed_message_with(bytes, DecodeCtx::new())
}

/// [`decode_feed_message`] ile aynı, ama hazırlanmış bir bağlam alır
/// (ör. değiştirilmiş derinlik kapağı ile).
pub fn decode_feed_message_with(bytes: &[u8], mut ctx: DecodeCtx) -> DecodedFeed {
    if let Some(looks_like) = decode::looks_like_non_protobuf(bytes) {
        ctx.report(AnomalyKind::NotProtobuf { looks_like }, 0);
        return DecodedFeed {
            message: FeedMessage::default(),
            anomalies: ctx.into_anomalies(),
        };
    }

    let message = decode::decode_body::<FeedMessage>(bytes, &mut ctx);
    DecodedFeed {
        message,
        anomalies: ctx.into_anomalies(),
    }
}
