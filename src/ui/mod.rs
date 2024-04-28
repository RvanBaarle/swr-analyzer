mod viewmodel;
mod log;

use ::log::{error, warn};
use gtk::{Application, ApplicationWindow, Button, cairo, DrawingArea, glib, Label, TreeView, Window};
use gtk::gio::ApplicationFlags;
use gtk::glib::{ControlFlow, Propagation};
use gtk::prelude::*;
use plotters::prelude::*;
use plotters_cairo::CairoBackend;
use crate::ui::log::Logger;

pub fn test_graph(drawing_area: &gtk::cairo::Context, (w, h): (u32, u32)) {
    let root = CairoBackend::new(drawing_area, (w, h)).unwrap().into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Test", ("sans-serif", 30).into_font())
        .margin(5)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(1000000f32..35000000f32, 0f32..1024f32).unwrap();

    chart.configure_mesh()
        .x_desc("Frequency [MHz]")
        .x_label_formatter(&|x| format!("{:.2}", x / 1000000.0))
        .y_desc("Voltage [dB]")
        .draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            (1000000..=35000000).step_by(100).map(|x| (x as f32, x as f32 / 35000000.0)).map(|(x, y)| (x, y * y)),
            &RED,
        )).unwrap();

    root.present().unwrap();
}

struct GraphData {
    start_freq: f32,
    stop_freq: f32,
    y_min: f32,
    y_max: f32,
    samples: Vec<(f32, f32)>
}

pub fn draw_graph(context: &cairo::Context, area: (u32, u32), data: &GraphData) {
    let root = CairoBackend::new(context, area).unwrap().into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Oneshot", ("sans-serif", 30).into_font())
        .margin(5)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(data.start_freq..data.stop_freq, data.y_min..data.y_max).unwrap();

    chart.configure_mesh()
        .x_desc("Frequency [MHz]")
        .x_label_formatter(&|x| format!("{:.2}", x / 1000000.0))
        .y_desc("Voltage [dB]")
        .draw().unwrap();

    // chart
    //     .draw_series(LineSeries::new(
    //         data.samples.iter(),
    //         &RED,
    //     )).unwrap();

    root.present().unwrap();
}

pub fn ui_main() {
    let app = Application::builder()
        .application_id("nl.actuallyruben.ruben.SwrAnalyzer")
        .build();

    let glade_src = include_str!("ui.glade");

    app.connect_activate(|app| {
        let builder = gtk::Builder::from_string(glade_src);

        let list_log: TreeView = builder.object("list_log").unwrap();
        Logger::init(list_log).expect("init logger");

        let win: ApplicationWindow = builder.object("window_main").unwrap();
        win.set_application(Some(app));
        let log_win: Window = builder.object("window_log").unwrap();

        let button_oneshot: Button = builder.object("button_oneshot").unwrap();
        let label_oneshot: Label = builder.object("label_oneshot").unwrap();
        button_oneshot.connect_clicked(move |button| {
            warn!("BUTTON PUSH");
            error!("WEEWOOWEEWOO");
            label_oneshot.set_label("DO NOT THE BUTTON >:(");
        });

        let drawing_area: DrawingArea = builder.object("canvas_graph").unwrap();

        drawing_area.connect_draw(|area, context| {
            test_graph(context, (area.allocated_width() as u32, area.allocated_height() as u32));
            Propagation::Stop
        });

        win.show_all();
        log_win.show_all();
    });

    app.run();
}