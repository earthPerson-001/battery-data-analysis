use std::error::Error;

use chrono::DateTime;
use chrono::Utc;

use crate::read_data::BatteryHistoryRecord;
use plotters::prelude::*;
use plotters::style::full_palette::PURPLE;

///
/// Plot the battery graph consisting of charging, discharging and unindentified portions.
/// If proper separation is provided in each portions, visual distinction can be made otherwise
/// ever curves will be spread across whole graph making only the last one visible
///
/// # Paramaters
/// id: unique graph id
///
/// backend: the backend for plotting e.g. CairoBackend, SVGBackend, etc
///
fn plot_battery_data_pdf<'a, DB: DrawingBackend + 'a>(
    original_sorted_data: (&Vec<DateTime<Utc>>, &Vec<i32>),
    charging: (&Vec<Vec<DateTime<Utc>>>, &Vec<Vec<i32>>),
    discharging: (&Vec<Vec<DateTime<Utc>>>, &Vec<Vec<i32>>),
    none: (&Vec<Vec<DateTime<Utc>>>, &Vec<Vec<i32>>),
    backend: DB,
    show_data_points: bool,
) -> Result<(), Box<dyn Error + 'a>> {
    let root_area = backend.into_drawing_area();
    root_area.fill(&TRANSPARENT)?;

    let mut start_date: DateTime<Utc> = DateTime::<Utc>::MAX_UTC;
    let mut end_date: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;

    let mut min_capacity = i32::MAX;
    let mut max_capacity = i32::MIN;

    let mut set_min_and_max = |x: (&Vec<DateTime<Utc>>, &Vec<i32>)| {
        if !x.0.is_empty() {
            start_date = start_date.min(x.0.first().unwrap().to_owned());
            end_date = end_date.max(x.0.last().unwrap().to_owned());
        }

        if !x.1.is_empty() {
            min_capacity = min_capacity.min(x.1.iter().min().unwrap().to_owned());
            max_capacity = max_capacity.max(x.1.iter().max().unwrap().to_owned());
        }
    };
    charging
        .0
        .iter()
        .zip(charging.1.iter())
        .for_each(&mut set_min_and_max);
    discharging
        .0
        .iter()
        .zip(discharging.1.iter())
        .for_each(&mut set_min_and_max);
    none.0
        .iter()
        .zip(none.1.iter())
        .for_each(&mut set_min_and_max);

    // if the start_date or end_date are still MAX_UTC and MIN_UTC respectively, there was something wrong
    // debug
    // todo: return proper error when this happens
    assert_ne!(start_date, DateTime::<Utc>::MAX_UTC);
    assert_ne!(end_date, DateTime::<Utc>::MIN_UTC);

    let mut ctx = ChartBuilder::on(&root_area)
        .y_label_area_size(70)
        .x_label_area_size(100)
        // .caption("Battery Usage History", ("sans-serif", 40))
        .build_cartesian_2d(
            start_date..end_date,
            (min_capacity as f64 - min_capacity as f64 * 0.5) as i32
                ..(max_capacity as f64 + min_capacity as f64 * 0.5) as i32,
        )?;

    ctx.configure_mesh()
        .x_label_formatter(&|x| {
            format!("{} hrs", (Utc::now().signed_duration_since(x).num_hours()))
        })
        .disable_mesh()
        .light_line_style(WHITE)
        .draw()?;

    let line_colors = [GREEN, RED, BLACK];
    let dot_color =  BLUE;

    // draw the dots only on the original data, not on the interpolated data
    if show_data_points {
        ctx.draw_series(original_sorted_data.0.iter().zip(original_sorted_data.1.iter()).map(
            |(date, capacity)| {
                Circle::new(
                    (*date, *capacity),
                    5,
                    ShapeStyle {
                        color: dot_color.mix(1.0),
                        filled: true,
                        stroke_width: 1,
                    },
                )
            },
        ))?;
    }

    for (i, state) in [charging, discharging, none].iter().enumerate() {
        for (trend_charge, trend_state) in state.0.iter().zip(state.1.iter()) {
            // the line
            ctx.draw_series(LineSeries::new(
                trend_charge
                    .iter()
                    .zip(trend_state.iter())
                    .map(|(date, capacity)| (*date, *capacity)),
                &line_colors[i],
            ))?;
        }
    }
    root_area.present()?;
    Ok(())
}

/// Plot single graph of the whole data
/// and plot smaller graphs of various sections (turned off because it required long time)
///
/// todo: provide interface to control the size of each small graph
///
pub fn start_battery_plot<'a, DB: DrawingBackend + 'a>(
    original_sorted_data: (&Vec<DateTime<Utc>>, &Vec<i32>),
    charging: (&Vec<Vec<DateTime<Utc>>>, &Vec<Vec<i32>>),
    discharging: (&Vec<Vec<DateTime<Utc>>>, &Vec<Vec<i32>>),
    none: (&Vec<Vec<DateTime<Utc>>>, &Vec<Vec<i32>>),
    backend: DB,
    show_data_points: bool,
) -> Result<(), Box<dyn Error + 'a>> {
    let x_data_charging = charging.0;
    let y_data_charging = charging.1;

    let x_data_discharging = discharging.0;
    let y_data_discharging = discharging.1;

    let x_data_none = none.0;
    let y_data_none = none.1;

    // the whole graph
    plot_battery_data_pdf(
        original_sorted_data,
        (x_data_charging, y_data_charging),
        (x_data_discharging, y_data_discharging),
        (x_data_none, y_data_none),
        backend,
        show_data_points,
    )?;

    Ok(())
}
