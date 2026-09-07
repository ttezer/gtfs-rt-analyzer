//! Dar kapsamlı statik GTFS okuyucu.
//!
//! Bu crate yalnızca realtime ile eşleştirmede gereken beş tabloyu okur:
//! `routes`, `trips`, `stops`, `stop_times` ve isteğe bağlı `calendar`.
//! `gtfs-pipeline`'a bağlanmaz ve kural çalıştırmaz; deterministik indeksleri
//! üst katmandaki cross-validation kurallarına verir.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{Cursor, Read};
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub route_id: String,
    pub route_short_name: Option<String>,
    pub route_long_name: Option<String>,
    pub route_type: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trip {
    pub route_id: String,
    pub service_id: String,
    pub trip_id: String,
    pub trip_headsign: Option<String>,
    pub direction_id: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stop {
    pub stop_id: String,
    pub stop_name: Option<String>,
    pub stop_lat: Option<f64>,
    pub stop_lon: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopTime {
    pub trip_id: String,
    pub arrival_time: Option<u32>,
    pub departure_time: Option<u32>,
    pub stop_id: String,
    pub stop_sequence: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calendar {
    pub service_id: String,
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticFeedSummary {
    pub routes: usize,
    pub trips: usize,
    pub stops: usize,
    pub stop_times: usize,
    pub calendars: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticFeed {
    routes: BTreeMap<String, Route>,
    trips: BTreeMap<String, Trip>,
    stops: BTreeMap<String, Stop>,
    stop_times: BTreeMap<String, Vec<StopTime>>,
    calendars: BTreeMap<String, Calendar>,
}

impl StaticFeed {
    /// GTFS ZIP arşivini bellekte açar. Gerekli beş tablodan `calendar.txt`
    /// eksik olabilir; bu durumda calendar indeksi boş kalır.
    pub fn from_zip_bytes(bytes: &[u8]) -> Result<Self, StaticFeedError> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|source| StaticFeedError::Zip { detail: source.to_string() })?;

        let routes = parse_routes(&read_required(&mut archive, "routes.txt")?)?;
        let trips = parse_trips(&read_required(&mut archive, "trips.txt")?)?;
        let stops = parse_stops(&read_required(&mut archive, "stops.txt")?)?;
        let stop_times = parse_stop_times(&read_required(&mut archive, "stop_times.txt")?)?;
        let calendars = match read_optional(&mut archive, "calendar.txt")? {
            Some(bytes) => parse_calendars(&bytes)?,
            None => BTreeMap::new(),
        };

        Ok(Self {
            routes,
            trips,
            stops,
            stop_times,
            calendars,
        })
    }

    pub fn routes(&self) -> &BTreeMap<String, Route> {
        &self.routes
    }

    pub fn trips(&self) -> &BTreeMap<String, Trip> {
        &self.trips
    }

    pub fn stops(&self) -> &BTreeMap<String, Stop> {
        &self.stops
    }

    pub fn calendars(&self) -> &BTreeMap<String, Calendar> {
        &self.calendars
    }

    /// Trip kimliğine göre stop-time sıralı görünüm.
    pub fn stop_times_for_trip(&self, trip_id: &str) -> Option<&[StopTime]> {
        self.stop_times.get(trip_id).map(Vec::as_slice)
    }

    pub fn stop_time_count(&self) -> usize {
        self.stop_times.values().map(Vec::len).sum()
    }

    pub fn summary(&self) -> StaticFeedSummary {
        StaticFeedSummary {
            routes: self.routes.len(),
            trips: self.trips.len(),
            stops: self.stops.len(),
            stop_times: self.stop_time_count(),
            calendars: self.calendars.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticFeedError {
    Zip { detail: String },
    MissingFile { file: &'static str },
    Csv { file: &'static str, detail: String },
    MissingColumn { file: &'static str, column: &'static str },
    DuplicateId { file: &'static str, id: String },
    EmptyField { file: &'static str, column: &'static str, row: usize },
    InvalidField {
        file: &'static str,
        column: &'static str,
        value: String,
        row: usize,
    },
}

impl fmt::Display for StaticFeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zip { detail } => write!(f, "GTFS ZIP okunamadı: {detail}"),
            Self::MissingFile { file } => write!(f, "GTFS tablosu eksik: {file}"),
            Self::Csv { file, detail } => write!(f, "{file} CSV okunamadı: {detail}"),
            Self::MissingColumn { file, column } => {
                write!(f, "{file} zorunlu sütunu eksik: {column}")
            }
            Self::DuplicateId { file, id } => write!(f, "{file} yinelenen kimlik: {id}"),
            Self::EmptyField { file, column, row } => {
                write!(f, "{file} satır {row}: {column} boş")
            }
            Self::InvalidField { file, column, value, row } => {
                write!(f, "{file} satır {row}: {column} geçersiz değer: {value}")
            }
        }
    }
}

impl std::error::Error for StaticFeedError {}

fn read_required<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    file: &'static str,
) -> Result<Vec<u8>, StaticFeedError> {
    read_optional(archive, file)?.ok_or(StaticFeedError::MissingFile { file })
}

fn read_optional<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    wanted: &'static str,
) -> Result<Option<Vec<u8>>, StaticFeedError> {
    let mut index = None;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|source| StaticFeedError::Zip { detail: source.to_string() })?;
        if entry.name().rsplit('/').next() == Some(wanted) {
            index = Some(i);
            break;
        }
    }

    let Some(index) = index else { return Ok(None) };
    let mut entry = archive
        .by_index(index)
        .map_err(|source| StaticFeedError::Zip { detail: source.to_string() })?;
    let mut bytes = Vec::new();
    entry
        .read_to_end(&mut bytes)
        .map_err(|source| StaticFeedError::Zip { detail: source.to_string() })?;
    Ok(Some(bytes))
}

struct Table<'a> {
    file: &'static str,
    headers: csv::StringRecord,
    bytes: &'a [u8],
}

impl<'a> Table<'a> {
    fn new(file: &'static str, bytes: &'a [u8]) -> Result<Self, StaticFeedError> {
        let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(bytes);
        let headers = reader
            .headers()
            .map_err(|source| StaticFeedError::Csv { file, detail: source.to_string() })?
            .clone();
        Ok(Self { file, headers, bytes })
    }

    fn column(&self, name: &'static str) -> Result<usize, StaticFeedError> {
        self.headers
            .iter()
            .position(|header| header == name)
            .ok_or(StaticFeedError::MissingColumn { file: self.file, column: name })
    }

    fn records(&self) -> Result<Vec<(usize, csv::StringRecord)>, StaticFeedError> {
        let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(self.bytes);
        let mut rows = Vec::new();
        for (index, result) in reader.records().enumerate() {
            let row = index + 2;
            let record = result.map_err(|source| StaticFeedError::Csv {
                file: self.file,
                detail: format!("satır {row}: {source}"),
            })?;
            rows.push((row, record));
        }
        Ok(rows)
    }

    fn value(
        &self,
        record: &csv::StringRecord,
        index: usize,
        row: usize,
        column: &'static str,
    ) -> Result<String, StaticFeedError> {
        let value = record.get(index).unwrap_or("").trim();
        if value.is_empty() {
            return Err(StaticFeedError::EmptyField {
                file: self.file,
                column,
                row,
            });
        }
        Ok(value.to_owned())
    }

    fn optional(&self, record: &csv::StringRecord, index: usize) -> Option<String> {
        record.get(index).map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned)
    }
}

fn parse_routes(bytes: &[u8]) -> Result<BTreeMap<String, Route>, StaticFeedError> {
    let table = Table::new("routes.txt", bytes)?;
    let route_id = table.column("route_id")?;
    let short = table.column("route_short_name").ok();
    let long = table.column("route_long_name").ok();
    let route_type = table.column("route_type").ok();
    let mut routes = BTreeMap::new();

    for (row, record) in table.records()? {
        let id = table.value(&record, route_id, row, "route_id")?;
        if routes.contains_key(&id) {
            return Err(StaticFeedError::DuplicateId { file: "routes.txt", id });
        }
        let route_type = match route_type.and_then(|index| table.optional(&record, index)) {
            Some(value) => Some(parse_number("routes.txt", "route_type", &value, row)?),
            None => None,
        };
        routes.insert(
            id.clone(),
            Route {
                route_id: id,
                route_short_name: short.and_then(|index| table.optional(&record, index)),
                route_long_name: long.and_then(|index| table.optional(&record, index)),
                route_type,
            },
        );
    }
    Ok(routes)
}

fn parse_trips(bytes: &[u8]) -> Result<BTreeMap<String, Trip>, StaticFeedError> {
    let table = Table::new("trips.txt", bytes)?;
    let route_id = table.column("route_id")?;
    let service_id = table.column("service_id")?;
    let trip_id = table.column("trip_id")?;
    let headsign = table.column("trip_headsign").ok();
    let direction = table.column("direction_id").ok();
    let mut trips = BTreeMap::new();

    for (row, record) in table.records()? {
        let id = table.value(&record, trip_id, row, "trip_id")?;
        if trips.contains_key(&id) {
            return Err(StaticFeedError::DuplicateId { file: "trips.txt", id });
        }
        let direction_id = match direction.and_then(|index| table.optional(&record, index)) {
            Some(value) => Some(parse_number("trips.txt", "direction_id", &value, row)?),
            None => None,
        };
        trips.insert(
            id.clone(),
            Trip {
                route_id: table.value(&record, route_id, row, "route_id")?,
                service_id: table.value(&record, service_id, row, "service_id")?,
                trip_id: id,
                trip_headsign: headsign.and_then(|index| table.optional(&record, index)),
                direction_id,
            },
        );
    }
    Ok(trips)
}

fn parse_stops(bytes: &[u8]) -> Result<BTreeMap<String, Stop>, StaticFeedError> {
    let table = Table::new("stops.txt", bytes)?;
    let stop_id = table.column("stop_id")?;
    let name = table.column("stop_name").ok();
    let lat = table.column("stop_lat").ok();
    let lon = table.column("stop_lon").ok();
    let mut stops = BTreeMap::new();

    for (row, record) in table.records()? {
        let id = table.value(&record, stop_id, row, "stop_id")?;
        if stops.contains_key(&id) {
            return Err(StaticFeedError::DuplicateId { file: "stops.txt", id });
        }
        stops.insert(
            id.clone(),
            Stop {
                stop_id: id,
                stop_name: name.and_then(|index| table.optional(&record, index)),
                stop_lat: parse_optional_float(&table, &record, lat, row, "stop_lat")?,
                stop_lon: parse_optional_float(&table, &record, lon, row, "stop_lon")?,
            },
        );
    }
    Ok(stops)
}

fn parse_stop_times(bytes: &[u8]) -> Result<BTreeMap<String, Vec<StopTime>>, StaticFeedError> {
    let table = Table::new("stop_times.txt", bytes)?;
    let trip_id = table.column("trip_id")?;
    let arrival = table.column("arrival_time").ok();
    let departure = table.column("departure_time").ok();
    let stop_id = table.column("stop_id")?;
    let sequence = table.column("stop_sequence")?;
    let mut by_trip: BTreeMap<String, Vec<StopTime>> = BTreeMap::new();

    for (row, record) in table.records()? {
        let trip = table.value(&record, trip_id, row, "trip_id")?;
        let stop = table.value(&record, stop_id, row, "stop_id")?;
        let sequence_value = table.value(&record, sequence, row, "stop_sequence")?;
        let stop_sequence = parse_number("stop_times.txt", "stop_sequence", &sequence_value, row)?;
        let arrival_time = parse_optional_time(&table, &record, arrival, row, "arrival_time")?;
        let departure_time = parse_optional_time(&table, &record, departure, row, "departure_time")?;
        by_trip.entry(trip.clone()).or_default().push(StopTime {
            trip_id: trip,
            arrival_time,
            departure_time,
            stop_id: stop,
            stop_sequence,
        });
    }

    for times in by_trip.values_mut() {
        times.sort_by_key(|stop_time| stop_time.stop_sequence);
    }
    Ok(by_trip)
}

fn parse_calendars(bytes: &[u8]) -> Result<BTreeMap<String, Calendar>, StaticFeedError> {
    let table = Table::new("calendar.txt", bytes)?;
    let service_id = table.column("service_id")?;
    let days = [
        ("monday", 1),
        ("tuesday", 2),
        ("wednesday", 3),
        ("thursday", 4),
        ("friday", 5),
        ("saturday", 6),
        ("sunday", 7),
    ];
    let day_columns: Vec<_> = days
        .iter()
        .map(|(name, _)| table.column(name).map(|index| (*name, index)))
        .collect::<Result<_, _>>()?;
    let start_date = table.column("start_date")?;
    let end_date = table.column("end_date")?;
    let mut calendars = BTreeMap::new();

    for (row, record) in table.records()? {
        let id = table.value(&record, service_id, row, "service_id")?;
        if calendars.contains_key(&id) {
            return Err(StaticFeedError::DuplicateId { file: "calendar.txt", id });
        }
        let mut values = [false; 7];
        for (position, (name, index)) in day_columns.iter().enumerate() {
            let value = table.value(&record, *index, row, name)?;
            values[position] = parse_flag("calendar.txt", name, &value, row)?;
        }
        let start = table.value(&record, start_date, row, "start_date")?;
        let end = table.value(&record, end_date, row, "end_date")?;
        validate_date("calendar.txt", "start_date", &start, row)?;
        validate_date("calendar.txt", "end_date", &end, row)?;
        calendars.insert(
            id.clone(),
            Calendar {
                service_id: id,
                monday: values[0],
                tuesday: values[1],
                wednesday: values[2],
                thursday: values[3],
                friday: values[4],
                saturday: values[5],
                sunday: values[6],
                start_date: start,
                end_date: end,
            },
        );
    }
    Ok(calendars)
}

fn parse_optional_float(
    table: &Table<'_>,
    record: &csv::StringRecord,
    index: Option<usize>,
    row: usize,
    column: &'static str,
) -> Result<Option<f64>, StaticFeedError> {
    let Some(index) = index else { return Ok(None) };
    let Some(value) = table.optional(record, index) else { return Ok(None) };
    value.parse().map(Some).map_err(|_| StaticFeedError::InvalidField {
        file: table.file,
        column,
        value,
        row,
    })
}

fn parse_optional_time(
    table: &Table<'_>,
    record: &csv::StringRecord,
    index: Option<usize>,
    row: usize,
    column: &'static str,
) -> Result<Option<u32>, StaticFeedError> {
    let Some(index) = index else { return Ok(None) };
    let Some(value) = table.optional(record, index) else { return Ok(None) };
    parse_time(&value).map(Some).map_err(|_| StaticFeedError::InvalidField {
        file: table.file,
        column,
        value,
        row,
    })
}

fn parse_number<T: std::str::FromStr>(
    file: &'static str,
    column: &'static str,
    value: &str,
    row: usize,
) -> Result<T, StaticFeedError> {
    value.parse().map_err(|_| StaticFeedError::InvalidField {
        file,
        column,
        value: value.to_owned(),
        row,
    })
}

fn parse_time(value: &str) -> Result<u32, ()> {
    let mut pieces = value.split(':');
    let hour: u32 = pieces.next().ok_or(())?.parse().map_err(|_| ())?;
    let minute: u32 = pieces.next().ok_or(())?.parse().map_err(|_| ())?;
    let second: u32 = pieces.next().ok_or(())?.parse().map_err(|_| ())?;
    if pieces.next().is_some() || minute > 59 || second > 59 {
        return Err(());
    }
    hour.checked_mul(3600)
        .and_then(|value| value.checked_add(minute * 60))
        .and_then(|value| value.checked_add(second))
        .ok_or(())
}

fn parse_flag(file: &'static str, column: &'static str, value: &str, row: usize) -> Result<bool, StaticFeedError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(StaticFeedError::InvalidField {
            file,
            column,
            value: value.to_owned(),
            row,
        }),
    }
}

fn validate_date(file: &'static str, column: &'static str, value: &str, row: usize) -> Result<(), StaticFeedError> {
    if value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(StaticFeedError::InvalidField {
            file,
            column,
            value: value.to_owned(),
            row,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn archive(files: &[(&str, &str)]) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(&mut output);
        let options = SimpleFileOptions::default();
        for (name, content) in files {
            writer.start_file(*name, options).unwrap();
            writer.write_all(content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        output.into_inner()
    }

    fn required_files() -> [(&'static str, &'static str); 4] {
        [
            (
                "routes.txt",
                "route_id,route_short_name,route_long_name,route_type\nR1,1,\"Main Street, Local\",3\n",
            ),
            (
                "trips.txt",
                "route_id,service_id,trip_id,trip_headsign,direction_id\nR1,S1,T1,Town,0\n",
            ),
            (
                "stops.txt",
                "stop_id,stop_name,stop_lat,stop_lon\nS1,First,41.0,29.0\nS2,Second,41.1,29.1\n",
            ),
            (
                "stop_times.txt",
                "trip_id,arrival_time,departure_time,stop_id,stop_sequence\nT1,25:01:02,25:01:10,S2,2\nT1,24:59:00,24:59:10,S1,1\n",
            ),
        ]
    }

    #[test]
    fn reads_required_tables_and_sorts_stop_times() {
        let feed = StaticFeed::from_zip_bytes(&archive(&required_files())).unwrap();
        assert_eq!(feed.summary().routes, 1);
        assert_eq!(feed.summary().trips, 1);
        assert_eq!(feed.summary().stops, 2);
        assert_eq!(feed.summary().stop_times, 2);
        assert_eq!(feed.summary().calendars, 0);
        assert_eq!(feed.routes()["R1"].route_long_name.as_deref(), Some("Main Street, Local"));
        let times = feed.stop_times_for_trip("T1").unwrap();
        assert_eq!(times[0].stop_sequence, 1);
        assert_eq!(times[0].arrival_time, Some(89_940));
        assert_eq!(times[1].arrival_time, Some(90_062));
    }

    #[test]
    fn reads_calendar_and_boolean_fields() {
        let mut files = required_files().to_vec();
        files.push((
            "calendar.txt",
            "service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\nS1,1,1,1,1,1,0,0,20260101,20261231\n",
        ));
        let feed = StaticFeed::from_zip_bytes(&archive(&files)).unwrap();
        let calendar = &feed.calendars()["S1"];
        assert!(calendar.monday);
        assert!(!calendar.saturday);
        assert_eq!(calendar.end_date, "20261231");
    }

    #[test]
    fn missing_required_table_is_reported() {
        let files = required_files();
        let archive = archive(&files[..3]);
        assert_eq!(
            StaticFeed::from_zip_bytes(&archive),
            Err(StaticFeedError::MissingFile { file: "stop_times.txt" })
        );
    }

    #[test]
    fn duplicate_trip_id_is_rejected() {
        let mut files = required_files();
        files[1].1 = "route_id,service_id,trip_id\nR1,S1,T1\nR1,S1,T1\n";
        assert!(matches!(
            StaticFeed::from_zip_bytes(&archive(&files)),
            Err(StaticFeedError::DuplicateId { file: "trips.txt", .. })
        ));
    }

    #[test]
    fn invalid_time_is_rejected() {
        let mut files = required_files();
        files[3].1 = "trip_id,arrival_time,departure_time,stop_id,stop_sequence\nT1,25:60:00,,S1,1\n";
        assert!(matches!(
            StaticFeed::from_zip_bytes(&archive(&files)),
            Err(StaticFeedError::InvalidField { file: "stop_times.txt", column: "arrival_time", .. })
        ));
    }
}
