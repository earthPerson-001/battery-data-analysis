use chrono::DateTime;
use chrono::Utc;

mod plot;
mod read_data;

use crate::plot::start_battery_plot;
use crate::read_data::get_data;
use crate::read_data::sort_hashmap;
use crate::read_data::ChargeState;

fn main() {
    /* reading data from csv */

    let data =
        get_data("./assets/battery-history-csvs/batteryreport.csv").expect("Cannot load csv data");

    /* Separating data into charge, discharge and unidenfied portions */
    // todo: separate into smaller portions so that proper visualizations can be done in graph

    let mut x_data_charging: Vec<DateTime<Utc>> = vec![];
    let mut y_data_charging: Vec<i32> = vec![];

    let mut x_data_discharging: Vec<DateTime<Utc>> = vec![];
    let mut y_data_discharging: Vec<i32> = vec![];

    let mut x_data_none: Vec<DateTime<Utc>> = vec![];
    let mut y_data_none: Vec<i32> = vec![];

    for (_, record) in data.iter() {
        println!("{:?}", record);
        match record.state {
            ChargeState::Charging => x_data_charging.push(record.date_time),
            ChargeState::Discharging => x_data_discharging.push(record.date_time),
            ChargeState::Unknown => x_data_none.push(record.date_time),
        }
    }

    /* Sorting the data based on timestamp, as hashmap doesn't stored ordered data. */

    sort_hashmap(&data, &mut x_data_charging, &mut y_data_charging);
    sort_hashmap(&data, &mut x_data_discharging, &mut y_data_discharging);
    sort_hashmap(&data, &mut x_data_none, &mut y_data_none);

    /* Visualize the data */
    
    start_battery_plot(
        (&x_data_charging, &y_data_charging),
        (&x_data_discharging, &y_data_discharging),
        (&x_data_none, &y_data_none)
    );
}
