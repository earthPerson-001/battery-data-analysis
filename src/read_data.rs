use chrono::serde::ts_seconds::deserialize as ts_s;
use chrono::DateTime;
use chrono::Utc;
use csv::ReaderBuilder;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum ChargeState {
    Charging,
    Discharging,
    Unknown,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BatteryHistoryRecord {
    #[serde(deserialize_with = "ts_s")]
    pub date_time: DateTime<Utc>,
    pub capacity: i32,
    pub state: ChargeState,
}

/// Reads the csv from given path and deserializes the data into
/// the type [BatteryHistoryRecord].
///
/// # Parameters
/// path: [&str] of the battery report csv consisting of headers date_time, capacity, state
///
/// The csv in the provided path must contain header corresponding to data types in [BatteryHistoryRecord] otherwise deserialization will panic
///
/// # Returns
/// The unsorted [HashMap] which contains [`DateTime<Utc>`] key and [BatteryHistoryRecord] value pairs.
pub fn get_data(
    path: &str,
) -> Result<HashMap<DateTime<Utc>, BatteryHistoryRecord>, Box<dyn Error>> {
    let mut data_hash_map: HashMap<DateTime<Utc>, BatteryHistoryRecord> = HashMap::new();

    let mut rdr = ReaderBuilder::new().has_headers(true).from_path(path)?;
    for result in rdr.deserialize::<BatteryHistoryRecord>() {
        let record = result?;
        data_hash_map.insert(record.date_time, record);
    }

    Ok(data_hash_map)
}

/// Stores sorted values into x_data, y_data
pub fn sort_hashmap(
    data: &HashMap<DateTime<Utc>, BatteryHistoryRecord>,
    x_data: &mut Vec<DateTime<Utc>>,
    y_data: &mut Vec<i32>,
) {
    for (key, _) in data.iter() {
        x_data.push(*key);
    }

    // since hashmap isn't ordered, ordering x_data
    x_data.sort();

    let mut invalid_values: Vec<usize> = vec![];

    // and selecting y_data based on that data
    for (i, dat) in (0..).zip(x_data.iter()) {
        if let Some(val) = data.get(dat) {
            if *dat == val.date_time {
                y_data.push(val.capacity);
            } else {
                invalid_values.push(i);
            }
        }
    }

    let _: Vec<chrono::DateTime<chrono::Utc>> = invalid_values
        .iter()
        .map(|index| x_data.remove(*index))
        .collect();
}
