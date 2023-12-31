use chrono::{DateTime, Duration, Utc};
pub use read_data::BatteryHistoryRecord;
use read_data::ChargeState;

mod plot;
mod read_data;

use crate::plot::start_battery_plot;
use crate::read_data::get_data;
use crate::read_data::sort_hashmap;
pub use plotters_cairo::CairoBackend;

use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;

use makima_spline::Spline;

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
                    pos.0,
                    pos.1 + (i as f64 * text_style.font.get_size()) as i32,
                ),
            )
            .unwrap();
    }
}

pub fn battery_plot_pdf<'a, DB: DrawingBackend + 'a>(
    backend: DB,
    predicted_data: HashMap<DateTime<Utc>, BatteryHistoryRecord>,
    data: HashMap<DateTime<Utc>, BatteryHistoryRecord>,
    from_days_before: Option<i64>,
    to_days_before: Option<i64>,
    show_data_points: bool,
    interpolate: bool,
    show_prediction: bool,
) -> Result<(), Box<dyn Error + 'a>> {
    /* reading data from csv */

    // debug
    if data.is_empty() {
        panic!("The provided data is empty.");
    }

    // discard predicted data when show_prediction is false 
    // and when the final date-time is past date-time
    let all_data: HashMap<DateTime<Utc>, BatteryHistoryRecord> = match show_prediction {
        true => match to_days_before {
            Some(0) => {
                HashMap::from_iter(data.into_iter().chain(predicted_data.clone().into_iter()))
            }
            _ => data,
        },
        false => data,
    };

    let end_date = all_data
        .iter()
        .reduce(|max_capacity_record, current| {
            all_data
                .get_key_value(max_capacity_record.0.max(current.0))
                .unwrap()
        })
        .unwrap()
        .0
        .to_owned();

    // all the data after the current date is prediction
    let current_date_time = chrono::Utc::now();

    let mut sanitized_data: HashMap<DateTime<Utc>, BatteryHistoryRecord> =
        HashMap::from_iter(all_data);
    let mut original_x_data: Vec<DateTime<Utc>> = Vec::new();
    let mut original_y_data: Vec<i32> = Vec::new();

    // removing all the entries before the start days
    if let Some(number_of_days) = from_days_before {
        let mut actual_number_of_days = number_of_days;

        // if the prediction needs to be shown and to_days_before is zero
        // i.e. showing only if the graph up to current is shown
        if show_prediction && to_days_before.is_some() && to_days_before.unwrap() == 0 {
            actual_number_of_days += 1;
        }

        let start_date = end_date - chrono::Duration::days(actual_number_of_days);
        sanitized_data.retain(|date, _| (date) > (&start_date));
    }

    // removing all the entries after the end days
    if let Some(number_of_days) = to_days_before {
        // show prediction of future if to_days_before is 0 and prediction is true
        // i.e. showing only if the graph up to current is shown
        if !show_prediction || (to_days_before.is_some() && to_days_before.unwrap() != 0) {
            let end_date = end_date - chrono::Duration::days(number_of_days);
            sanitized_data.retain(|date, _| (date) < (&end_date))
        }
    }

    /* Separating data into charge, discharge and unidentified portions */

    sort_hashmap(&sanitized_data, &mut original_x_data, &mut original_y_data);
    let x_data;
    let y_data;
    if interpolate {
        let converted_datetimes = original_x_data
            .iter()
            .map(|dt| dt.timestamp() as f64)
            .collect::<Vec<f64>>();
        let converted_capacities = original_y_data
            .iter()
            .map(|capacity| *capacity as f64)
            .collect::<Vec<f64>>();

        let points = makima_spline::vec_to_points(&converted_datetimes, &converted_capacities);

        let spline = Spline::from_vec(points);

        // interpolating for each minute
        let mut current_date = original_x_data.first().unwrap().to_owned();
        let last_date = original_x_data.last().unwrap().to_owned();
        let increment_by_minutes = 1;

        // new vectors for x and y date
        let mut interpolated_x_data: Vec<DateTime<Utc>> = Vec::new();
        let mut interpolated_y_data: Vec<i32> = Vec::new();

        while current_date <= current_date_time.min(last_date) {
            let y_data: f64;

            y_data = spline.sample(current_date.timestamp() as f64);

            interpolated_x_data.push(current_date);

            interpolated_y_data.push(y_data as i32);

            current_date = current_date
                .checked_add_signed(Duration::minutes(increment_by_minutes))
                .unwrap();

            let bat_record = BatteryHistoryRecord {
                date_time: current_date,
                capacity: y_data as i32,
                state: match sanitized_data.get(&current_date) {
                    Some(valid_record) => valid_record.state,
                    None => ChargeState::Unknown,
                },
            };

            // changing the key of the dictionary
            sanitized_data.insert(current_date, bat_record);
        }

        x_data = interpolated_x_data;
        y_data = interpolated_y_data;
    } else {
        x_data = original_x_data.clone();
        y_data = original_y_data.clone();
    }

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

        // add to predicted vectors if the data is of the future
        if x_data[cur_index] > current_date_time {
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
    let mut x_data_predicted: Vec<DateTime<Utc>> = Vec::new();
    let mut y_data_predicted: Vec<i32> = Vec::new();

    /* Visualize the data */
    if show_prediction && to_days_before.is_some() && to_days_before.unwrap() == 0 {
        // sorting the predication
        sort_hashmap(
            &predicted_data,
            &mut x_data_predicted,
            &mut y_data_predicted,
        )
    }

    if interpolate {
        start_battery_plot(
            (&original_x_data, &original_y_data),
            (&x_data_charging, &y_data_charging),
            (&x_data_discharging, &y_data_discharging),
            (&x_data_predicted, &y_data_predicted),
            (&x_data_none, &y_data_none),
            backend,
            show_data_points,
        )
        .unwrap();
    } else {
        start_battery_plot(
            (&x_data, &y_data),
            (&x_data_charging, &y_data_charging),
            (&x_data_discharging, &y_data_discharging),
            (&x_data_predicted, &y_data_predicted),
            (&x_data_none, &y_data_none),
            backend,
            show_data_points,
        )
        .unwrap();
    }

    Ok(())
}

pub fn get_data_from_csv(
    file_path: &str,
) -> Result<HashMap<DateTime<Utc>, BatteryHistoryRecord>, Box<dyn Error>> {
    get_data(file_path)
}
