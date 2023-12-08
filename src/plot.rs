
use std::error::Error;

use chrono::DateTime;
use chrono::Utc;
use itertools::izip;

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
fn plot_battery_data_pdf<'a, DB: DrawingBackend + 'a>(
    charging: (&[DateTime<Utc>], &[i32]),
    discharging: (&[DateTime<Utc>], &[i32]),
    none: (&[DateTime<Utc>], &[i32]),
    id: i32,
    backend: DB,
) -> Result<(), Box<dyn Error + 'a>> {

    let root_area = backend.into_drawing_area();
    root_area.fill(&WHITE)?;

    let mut start_date: DateTime<Utc> = DateTime::<Utc>::MAX_UTC;
    let mut end_date: DateTime<Utc> = DateTime::<Utc>::MIN_UTC;

    let mut min_capacity = i32::MAX;
    let mut max_capacity = i32::MIN;

    let mut set_min = |x: (&[DateTime<Utc>], &[i32])| {
        if !x.0.is_empty() {
            start_date = start_date.min(x.0.first().unwrap().to_owned());
            end_date = end_date.max(x.0.last().unwrap().to_owned());
        }

        if !x.1.is_empty() {
            min_capacity = min_capacity.min(x.1.first().unwrap().to_owned());
            max_capacity = max_capacity.max(x.1.last().unwrap().to_owned());
        }
    };

    set_min(charging);
    set_min(discharging);
    set_min(none);

    let mut ctx = ChartBuilder::on(&root_area)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .caption("Battery Usage History", ("sans-serif", 40))
        .build_cartesian_2d(
            start_date..end_date,
            min_capacity * 0.1 as i32..max_capacity,
        )?;

    ctx.configure_mesh().draw()?;

    let line_colors = [GREEN, RED, BLACK];
    let dot_colors = [BLUE, YELLOW, PURPLE];

    for (i, state) in [charging, discharging, none].iter().enumerate() {
        let (x, y) = state;

        // the line
        ctx.draw_series(LineSeries::new(
            x.iter()
                .zip(y.iter())
                .map(|(date, capacity)| (*date, *capacity)),
            &line_colors[i],
        ))?;

        // the dots
        ctx.draw_series(x.iter().zip(y.iter()).map(|(date, capacity)| {
            Circle::new(
                (*date, *capacity),
                5,
                ShapeStyle {
                    color: dot_colors[i].mix(1.0),
                    filled: true,
                    stroke_width: 2,
                },
            )
        }))?;


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
    charging: (&[DateTime<Utc>], &[i32]),
    discharging: (&[DateTime<Utc>], &[i32]),
    none: (&[DateTime<Utc>], &[i32]),
    backend: DB,
) -> Result<(), Box<dyn Error + 'a>> {
    let x_data_charging = charging.0;
    let y_data_charging = charging.1;

    let x_data_discharging = discharging.0;
    let y_data_discharging = discharging.1;

    let x_data_none = none.0;
    let y_data_none = none.1;

    // the whole graph
    plot_battery_data_pdf(
        (x_data_charging, y_data_charging),
        (x_data_discharging, y_data_discharging),
        (x_data_none, y_data_none),
        0,
        backend
    )?;

    // plotting smaller graphs
    // Takes a lot of time, so commenting it out
    /*
    // number of data in a graph
    const NUM_DATA: usize = 10;

    let x_chunks_charging = x_data_charging.chunks(NUM_DATA);
    let y_chunks_charging = y_data_charging.chunks(NUM_DATA);

    let x_chunks_discharging = x_data_charging.chunks(NUM_DATA);
    let y_chunks_discharging = y_data_charging.chunks(NUM_DATA);

    let x_chunks_none = x_data_charging.chunks(NUM_DATA);
    let y_chunks_none = y_data_charging.chunks(NUM_DATA);


    let mut index: i32 = 1;
    for (x_charging, y_charging, x_discharging, y_discharging, x_none, y_none) in izip!(x_chunks_charging, y_chunks_charging, x_chunks_discharging, y_chunks_discharging, x_chunks_none, y_chunks_none) {
        plot_battery_data(
            (x_charging, y_charging),
            (x_discharging, y_discharging),
            (x_none, y_none),
            index,
        );
        index += 1;
    }
    */
    // other parts

    Ok(())
}

