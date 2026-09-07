//! GTFS-Realtime mesaj modeli.
//!
//! `gtfs-realtime.proto` (proto2) birebir yansıtılır: 26 mesaj, 12 enum.
//! Her alan `Option<T>` ya da `Vec<T>`'dir — proto2 `required` alanlar dahil.
//! Zorunluluk, tipte değil `check_required` içinde denetlenir; böylece eksik bir
//! `required` alan decode'u düşürmek yerine **anomali** üretir ve kalan veri korunur.

pub(crate) mod macros;

pub mod alert;
pub mod descriptor;
pub mod enums;
pub mod feed;
pub mod modifications;
pub mod shape_stop;
pub mod trip_update;
pub mod vehicle;

pub use alert::Alert;
pub use descriptor::{
    EntitySelector, LocalizedImage, ModifiedTripSelector, Position, TimeRange, TranslatedImage,
    TranslatedString, Translation, TripDescriptor, VehicleDescriptor,
};
pub use feed::{FeedEntity, FeedHeader, FeedMessage};
pub use modifications::{
    Modification, ReplacementStop, SelectedTrips, StopSelector, TripModifications,
};
pub use shape_stop::{Shape, Stop};
pub use trip_update::{
    StopTimeEvent, StopTimeProperties, StopTimeUpdate, TripProperties, TripUpdate,
};
pub use vehicle::{CarriageDetails, VehiclePosition};
