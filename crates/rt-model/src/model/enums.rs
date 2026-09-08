//! `gtfs-realtime.proto`'daki enum'lar.
//!
//! Sayısal değerler spec'ten birebir alınmıştır ve **boşluklar korunur** —
//! ör. `TripDescriptor.ScheduleRelationship`'te 4 tanımlı değildir, dolayısıyla
//! 4 gelirse `UnknownEnumValue` anomalisi doğar. Boşluğu doldurmak, üreticinin
//! gerçek hatasını gizlerdi.

use crate::decode::ProtoEnum;

macro_rules! proto_enum {
    ($(#[$meta:meta])* $name:ident, $proto_name:literal, { $($variant:ident = $value:expr),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),+
        }

        impl ProtoEnum for $name {
            const NAME: &'static str = $proto_name;
            fn from_i32(value: i32) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }

        impl $name {
            /// Spec'teki sayısal değer.
            pub fn as_i32(self) -> i32 {
                match self { $(Self::$variant => $value),+ }
            }
            /// Spec'teki sabit adı.
            pub fn proto_name(self) -> &'static str {
                match self { $(Self::$variant => stringify!($variant)),+ }
            }
        }
    };
}

proto_enum!(
    /// `FeedHeader.incrementality` — varsayılan `FullDataset`.
    Incrementality, "FeedHeader.Incrementality", {
        FullDataset = 0,
        Differential = 1,
    }
);

proto_enum!(
    /// `TripDescriptor.schedule_relationship`.
    ///
    /// ⚠️ 4 **tanımsızdır** (spec'te atlanmış). `Added` = 1 kullanımdan kalkmıştır
    /// ama hâlâ geçerli bir değerdir.
    TripScheduleRelationship, "TripDescriptor.ScheduleRelationship", {
        Scheduled = 0,
        Added = 1,
        Unscheduled = 2,
        Canceled = 3,
        Replacement = 5,
        Duplicated = 6,
        Deleted = 7,
        New = 8,
    }
);

proto_enum!(
    /// `StopTimeUpdate.schedule_relationship` — varsayılan `Scheduled`.
    StopTimeScheduleRelationship, "StopTimeUpdate.ScheduleRelationship", {
        Scheduled = 0,
        Skipped = 1,
        NoData = 2,
        Unscheduled = 3,
    }
);

proto_enum!(
    /// `StopTimeProperties.pickup_type` / `drop_off_type`.
    DropOffPickupType, "StopTimeProperties.DropOffPickupType", {
        Regular = 0,
        None = 1,
        PhoneAgency = 2,
        CoordinateWithDriver = 3,
    }
);

proto_enum!(
    /// `VehiclePosition.current_status` — varsayılan `InTransitTo`.
    VehicleStopStatus, "VehiclePosition.VehicleStopStatus", {
        IncomingAt = 0,
        StoppedAt = 1,
        InTransitTo = 2,
    }
);

proto_enum!(
    /// `VehiclePosition.congestion_level`.
    CongestionLevel, "VehiclePosition.CongestionLevel", {
        UnknownCongestionLevel = 0,
        RunningSmoothly = 1,
        StopAndGo = 2,
        Congestion = 3,
        SevereCongestion = 4,
    }
);

proto_enum!(
    /// `VehiclePosition.occupancy_status` ve `CarriageDetails.occupancy_status`.
    OccupancyStatus, "VehiclePosition.OccupancyStatus", {
        Empty = 0,
        ManySeatsAvailable = 1,
        FewSeatsAvailable = 2,
        StandingRoomOnly = 3,
        CrushedStandingRoomOnly = 4,
        Full = 5,
        NotAcceptingPassengers = 6,
        NoDataAvailable = 7,
        NotBoardable = 8,
    }
);

proto_enum!(
    /// `Alert.cause` — varsayılan `UnknownCause`. **0 tanımsızdır**, numaralandırma 1'den başlar.
    AlertCause, "Alert.Cause", {
        UnknownCause = 1,
        OtherCause = 2,
        TechnicalProblem = 3,
        Strike = 4,
        Demonstration = 5,
        Accident = 6,
        Holiday = 7,
        Weather = 8,
        Maintenance = 9,
        Construction = 10,
        PoliceActivity = 11,
        MedicalEmergency = 12,
        SpecialEvent = 13,
    }
);

proto_enum!(
    /// `Alert.effect` — varsayılan `UnknownEffect`. **0 tanımsızdır**.
    AlertEffect, "Alert.Effect", {
        NoService = 1,
        ReducedService = 2,
        SignificantDelays = 3,
        Detour = 4,
        AdditionalService = 5,
        ModifiedService = 6,
        OtherEffect = 7,
        UnknownEffect = 8,
        StopMoved = 9,
        NoEffect = 10,
        AccessibilityIssue = 11,
    }
);

proto_enum!(
    /// `Alert.severity_level` — varsayılan `UnknownSeverity`. **0 tanımsızdır**.
    AlertSeverityLevel, "Alert.SeverityLevel", {
        UnknownSeverity = 1,
        Info = 2,
        Warning = 3,
        Severe = 4,
    }
);

proto_enum!(
    /// `VehicleDescriptor.wheelchair_accessible` — varsayılan `NoValue`.
    WheelchairAccessible, "VehicleDescriptor.WheelchairAccessible", {
        NoValue = 0,
        Unknown = 1,
        WheelchairAccessible = 2,
        WheelchairInaccessible = 3,
    }
);

proto_enum!(
    /// `Stop.wheelchair_boarding` — varsayılan `Unknown`.
    WheelchairBoarding, "Stop.WheelchairBoarding", {
        Unknown = 0,
        Available = 1,
        NotAvailable = 2,
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_i32() {
        assert_eq!(
            TripScheduleRelationship::from_i32(3),
            Some(TripScheduleRelationship::Canceled)
        );
        assert_eq!(TripScheduleRelationship::Canceled.as_i32(), 3);
        assert_eq!(TripScheduleRelationship::New.proto_name(), "New");
    }

    #[test]
    fn undefined_trip_relationship_four_is_rejected() {
        // Spec'te 4 atlanmıştır; doldurmak üreticinin hatasını gizlerdi.
        assert_eq!(TripScheduleRelationship::from_i32(4), None);
    }

    #[test]
    fn alert_enums_do_not_define_zero() {
        // Cause/Effect/SeverityLevel 1'den başlar. Bir üretici 0 gönderiyorsa
        // muhtemelen proto3 varsayılanı sanıyordur — bu bir bulgudur.
        assert_eq!(AlertCause::from_i32(0), None);
        assert_eq!(AlertEffect::from_i32(0), None);
        assert_eq!(AlertSeverityLevel::from_i32(0), None);
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        assert_eq!(OccupancyStatus::from_i32(9), None);
        assert_eq!(VehicleStopStatus::from_i32(-1), None);
        assert_eq!(Incrementality::from_i32(2), None);
    }
}
