use std::f64::consts::PI;
use std::fs;

use nalgebra::DVector;
use plotters::prelude::*;

extern crate siras;
use siras::filter_design::FilterType;
use siras::lti::DiscreteTransferFunction;

fn plot(
    x: &DVector<f64>,
    y: &DVector<f64>,
    (w, h): (u32, u32),
    path: &str,
    title: &str,
    x_label: &str,
    y_label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(path, (w, h)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 30, FontStyle::Normal).into_font())
        .margin(20)
        .x_label_area_size(60)
        .y_label_area_size(70)
        .build_cartesian_2d(x.min()..x.max(), -2.0..2.0)?;

    let label_font_x = ("sans-serif", 25, FontStyle::Normal).into_font();
    let label_font_y = ("sans-serif", 25, FontStyle::Normal).into_font();
    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .x_label_style(label_font_x)
        .y_label_style(label_font_y)
        .draw()?;

    chart
        .draw_series(LineSeries::new(
            x.iter().copied().zip(y.iter().copied()),
            Palette99::pick(3).stroke_width(2),
        ))?
        .label(title)
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], Palette99::pick(3)));

    root.present()?;

    Ok(())
}

fn main() {
    let sample_rate = 32000;
    let t = DVector::from_fn(sample_rate, |i, _| i as f64 / sample_rate as f64);

    let freq1 = 10.0;
    let freq2 = 100.0;
    let signal = (2.0 * PI * freq1 * t.clone()).map(|e| e.sin())
        + (2.0 * PI * freq2 * t.clone()).map(|e| e.sin());

    let order = 4;
    let dt = 1.0 / sample_rate as f64;
    let cutoff_freq_high_pass = 50.0;
    let cutoff_freq_low_pass = 15.0;

    let signal_with_high_pass_filter =
        DiscreteTransferFunction::bessel(order, cutoff_freq_high_pass, dt, FilterType::HighPass)
            .filtfilt(&signal, &t);
    let signal_with_low_pass_filter =
        DiscreteTransferFunction::bessel(order, cutoff_freq_low_pass, dt, FilterType::LowPass)
            .filtfilt(&signal, &t);

    let plot_dir = "examples/plots";
    if !std::path::Path::new(plot_dir).exists() {
        fs::create_dir_all(plot_dir).unwrap();
    }

    plot(
        &t,
        &signal,
        (1200, 600),
        &format!("{}/bessel_without_filter.png", plot_dir),
        "without filter",
        "time",
        "amplitude",
    )
    .unwrap();
    plot(
        &t,
        &signal_with_low_pass_filter,
        (1200, 600),
        &format!("{}/bessel_with_low_pass_filter.png", plot_dir),
        "with low pass filter",
        "time",
        "amplitude",
    )
    .unwrap();
    plot(
        &t,
        &signal_with_high_pass_filter,
        (1200, 600),
        &format!("{}/bessel_with_high_pass_filter.png", plot_dir),
        "with high pass filter",
        "time",
        "amplitude",
    )
    .unwrap();
}
