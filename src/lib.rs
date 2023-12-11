use chrono::DateTime;
use chrono::Utc;
use read_data::BatteryHistoryRecord;

mod plot;
mod read_data;

use crate::plot::start_battery_plot;
use crate::read_data::get_data;
use crate::read_data::sort_hashmap;
pub use plotters_cairo::CairoBackend;

use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;

enum PreviousVal {
    None,
    Increasing,
    Decreasing,
}

pub fn display_error<'a, DB: DrawingBackend + 'a>(
    backend: DB,
    error_message: &str,
    pos: (i32, i32),
) {
    let drawing_area = backend.into_drawing_area();
    drawing_area.fill(&BLACK).unwrap();
    let text_style = ("sans-serif", 20, &RED).into_text_style(&drawing_area);
    let errors = error_message.lines();
    
    for (i, error) in errors.enumerate() {
        drawing_area
            .draw_text(
                error,
                &text_style,
                (
                    pos.0 ,
                    pos.1 + (i as f64 * text_style.font.get_size()) as i32,
                ),
            )
            .unwrap();
    }
}

pub fn battery_plot_pdf<'a, DB: DrawingBackend + 'a>(
    backend: DB,
    data: HashMap<DateTime<Utc>, BatteryHistoryRecord>,
    from_days_before: Option<i64>,
    to_days_before: Option<i64>,
    show_data_points: bool,
) -> Result<(), Box<dyn Error + 'a>> {
    /* reading data from csv */

    // debug
    if data.is_empty() {
        panic!("The provided data is empty.");
    }

    let end_date = data
        .iter()
        .reduce(|max_capacity_record, current| {
            data.get_key_value(max_capacity_record.0.max(current.0))
                .unwrap()
        })
        .unwrap()
        .0
        .to_owned();

    let mut sanitized_data = data;
    let mut x_data: Vec<DateTime<Utc>> = Vec::new();
    let mut y_data: Vec<i32> = Vec::new();

    // removing all the entries before the start days
    if let Some(number_of_days) = from_days_before {
        let start_date = end_date - chrono::Duration::days(number_of_days);
        sanitized_data.retain(|date, _| (date) > (&start_date));
    }

    // removing all the entries after the end days
    if let Some(number_of_days) = to_days_before {
        let end_date = end_date - chrono::Duration::days(number_of_days);
        sanitized_data.retain(|date, _| (date) < (&end_date))
    }

    /* Separating data into charge, discharge and unidenfied portions */

    sort_hashmap(&sanitized_data, &mut x_data, &mut y_data);

    // the data must be sorted up to now, so we can separate into increasing and decreasing trends
    let mut cur_index = 0;
    let max_index = x_data.len() - 1;

    let mut prev_capacity = sanitized_data
        .get(&x_data[cur_index])
        .expect("Couldn't get the capacity")
        .capacity;
    let mut cur_capacity;

    let mut previous_trend = PreviousVal::None;

    let mut x_data_charging: Vec<Vec<DateTime<Utc>>> = Vec::new();
    let mut y_data_charging: Vec<Vec<i32>> = Vec::new();

    let mut x_data_discharging: Vec<Vec<DateTime<Utc>>> = Vec::new();
    let mut y_data_discharging: Vec<Vec<i32>> = Vec::new();

    let mut x_data_none: Vec<Vec<DateTime<Utc>>> = Vec::new();
    let mut y_data_none: Vec<Vec<i32>> = Vec::new();

    // pushing the first value to none
    x_data_none.push(Vec::new());
    y_data_none.push(Vec::new());

    x_data_none.last_mut().unwrap().push(x_data[cur_index]);
    y_data_none
        .last_mut()
        .unwrap()
        .push(sanitized_data.get(&x_data[cur_index]).unwrap().capacity);

    loop {
        cur_index += 1;
        if cur_index > max_index {
            break;
        }
        cur_capacity = sanitized_data.get(&x_data[cur_index]);

        if cur_capacity.is_none() {
            continue;
        }

        match previous_trend {
            PreviousVal::None => {
                if x_data_none.last_mut().is_none() || y_data_none.last_mut().is_none() {
                    x_data_none.push(Vec::new());
                    y_data_none.push(Vec::new());
                }
                x_data_none.last_mut().unwrap().push(x_data[cur_index]);
                y_data_none
                    .last_mut()
                    .unwrap()
                    .push(cur_capacity.unwrap().capacity);

                previous_trend = match cur_capacity.unwrap().capacity.cmp(&prev_capacity) {
                    std::cmp::Ordering::Less => {
                        // creating a new vector for storing the following decreasing values
                        x_data_discharging.push(Vec::new());
                        y_data_discharging.push(Vec::new());

                        // pushing the current value for connecting the dots otherwise the curve will be disconnected
                        x_data_discharging
                            .last_mut()
                            .unwrap()
                            .push(x_data[cur_index]);
                        y_data_discharging
                            .last_mut()
                            .unwrap()
                            .push(cur_capacity.unwrap().capacity);

                        PreviousVal::Decreasing
                    }
                    std::cmp::Ordering::Equal => PreviousVal::None, // already created a vector for storing the following none values
                    std::cmp::Ordering::Greater => {
                        // creating a new vector for storing the following increasing values
                        x_data_charging.push(Vec::new());
                        y_data_charging.push(Vec::new());

                        // pushing the current value for connecting the dots otherwise the curve will be disconnected
                        x_data_charging.last_mut().unwrap().push(x_data[cur_index]);
                        y_data_charging
                            .last_mut()
                            .unwrap()
                            .push(cur_capacity.unwrap().capacity);

                        PreviousVal::Increasing
                    }
                };
            }
            PreviousVal::Increasing => {
                // if this also increases (or is equal), pushing into the last increasing vector
                if cur_capacity.unwrap().capacity >= prev_capacity {
                    previous_trend = PreviousVal::Increasing;
                    x_data_charging.last_mut().unwrap().push(x_data[cur_index]);
                    y_data_charging
                        .last_mut()
                        .unwrap()
                        .push(cur_capacity.unwrap().capacity);
                } else {
                    previous_trend = PreviousVal::Decreasing;
                    // creating a new vectors as this is separate decreasing curve
                    x_data_discharging.push(Vec::new());
                    y_data_discharging.push(Vec::new());

                    if cur_index > 0 {
                        // pushing the previous data to make the graph connected
                        x_data_discharging
                            .last_mut()
                            .unwrap()
                            .push(x_data[cur_index - 1]);
                        y_data_discharging
                            .last_mut()
                            .unwrap()
                            .push(sanitized_data.get(&x_data[cur_index - 1]).unwrap().capacity);
                    }

                    x_data_discharging
                        .last_mut()
                        .unwrap()
                        .push(x_data[cur_index]);
                    y_data_discharging
                        .last_mut()
                        .unwrap()
                        .push(cur_capacity.unwrap().capacity);
                }
            }
            PreviousVal::Decreasing => {
                // if this also decreases (or is equal), pushing into the last decreasing vector
                if cur_capacity.unwrap().capacity <= prev_capacity {
                    previous_trend = PreviousVal::Decreasing;
                    x_data_discharging
                        .last_mut()
                        .unwrap()
                        .push(x_data[cur_index]);
                    y_data_discharging
                        .last_mut()
                        .unwrap()
                        .push(cur_capacity.unwrap().capacity);
                } else {
                    previous_trend = PreviousVal::Increasing;
                    // creating a new vectors as this is separate increasing curve
                    x_data_charging.push(Vec::new());
                    y_data_charging.push(Vec::new());

                    if cur_index > 0 {
                        // pushing the previous data to make the graph connected
                        x_data_charging
                            .last_mut()
                            .unwrap()
                            .push(x_data[cur_index - 1]);
                        y_data_charging
                            .last_mut()
                            .unwrap()
                            .push(sanitized_data.get(&x_data[cur_index - 1]).unwrap().capacity);
                    }

                    x_data_charging.last_mut().unwrap().push(x_data[cur_index]);
                    y_data_charging
                        .last_mut()
                        .unwrap()
                        .push(cur_capacity.unwrap().capacity);
                }
            }
        }
        prev_capacity = cur_capacity.unwrap().capacity;
    }

    /* Visualize the data */

    start_battery_plot(
        (&x_data_charging, &y_data_charging),
        (&x_data_discharging, &y_data_discharging),
        (&x_data_none, &y_data_none),
        backend,
        show_data_points,
    )?;

    Ok(())
}

pub fn get_data_from_csv(
    file_path: &str,
) -> Result<HashMap<DateTime<Utc>, BatteryHistoryRecord>, Box<dyn Error>> {
    get_data(file_path)
}
