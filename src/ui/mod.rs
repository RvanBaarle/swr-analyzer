use std::cell::{Cell, RefCell};
use std::default::Default;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;

use ::log::{debug, error, info, warn};
use gtk::{Application, ApplicationWindow, Button, cairo, DrawingArea, Entry, glib, Label, TreeView, Window};
use gtk::gio::ApplicationFlags;
use gtk::glib::{clone, ControlFlow, Propagation};
use gtk::prelude::*;
use plotters::prelude::*;
use plotters_cairo::CairoBackend;

use crate::protocol::error::Result;
use crate::protocol::libusb::SerialHID;
use crate::protocol::SWRAnalyzer;
use crate::ui::log::Logger;

mod log;

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
    samples: Vec<(f32, f32)>,
}

fn draw_graph(context: &cairo::Context, area: (u32, u32), data: &GraphData) {
    let root = CairoBackend::new(context, area).unwrap().into_drawing_area();
    root.fill(&WHITE).unwrap();

    let mut chart = ChartBuilder::on(&root)
        .caption("Oneshot", ("sans-serif", 30).into_font())
        .margin(5)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(data.start_freq..data.stop_freq, data.y_min..data.y_max).unwrap();

    chart.configure_mesh()
        .x_desc("Frequency [MHz]")
        .x_label_formatter(&|x| format!("{:.2}", x / 1000000.0))
        .y_desc("Voltage [dB]")
        .draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            data.samples.iter().copied(),
            &RED,
        )).unwrap();

    root.present().unwrap();
}

fn get_analyzer() -> Result<Box<dyn SWRAnalyzer + Send>> {
    let context = libusb::Context::new()?;
    let mut dev = SerialHID::new(Arc::new(context))?;
    info!("version: {}", dev.version()?);
    Ok(Box::new(dev))
}

enum DataSample {
    Sample(i32, i32, i32),
    Done(Box<dyn SWRAnalyzer + Send>),
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

        let analyzer: Rc<RefCell<Option<Box<dyn SWRAnalyzer + Send>>>> = Default::default();

        match get_analyzer() {
            Ok(dev) => {
                *analyzer.borrow_mut() = Some(dev);
            }
            Err(e) => {
                error!("connection error: {}", e)
            }
        }

        let win: ApplicationWindow = builder.object("window_main").unwrap();
        win.set_application(Some(app));
        let log_win: Window = builder.object("window_log").unwrap();

        let button_oneshot: Button = builder.object("button_oneshot").unwrap();
        let button_show_logs: Button = builder.object("button_show_logs").unwrap();

        let graph_data = Rc::new(RefCell::new(GraphData {
            start_freq: 1000000.0,
            stop_freq: 35000000.0,
            y_min: 0.0,
            y_max: 1024.0,
            samples: vec![],
        }));

        let drawing_area: DrawingArea = builder.object("canvas_graph").unwrap();

        drawing_area.connect_draw(clone!(@strong graph_data => move |area, context| {
            let graph_data = graph_data.borrow();
            draw_graph(context, (area.allocated_width() as u32, area.allocated_height() as u32), &graph_data);
            Propagation::Stop
        }));

        let input_start_freq: Entry = builder.object("input_start_freq").unwrap();
        let input_stop_freq: Entry = builder.object("input_stop_freq").unwrap();
        let input_step_count: Entry = builder.object("input_step_count").unwrap();
        let input_step_time: Entry = builder.object("input_step_time").unwrap();

        button_oneshot.connect_clicked(clone!(@strong graph_data, @weak drawing_area => move |button| {
            let mut graph_data_locked = graph_data.borrow_mut();
            let Ok(start_freq) = input_start_freq.text().parse::<i32>() else {
                error!("Invalid start frequency");
                return;
            };
            let Ok(stop_freq) = input_stop_freq.text().parse::<i32>() else {
                error!("Invalid stop frequency");
                return;
            };
            let Ok(step_count) = input_step_count.text().parse::<i32>() else {
                error!("Invalid step count");
                return;
            };
            let Ok(step_time) = input_step_time.text().parse::<i32>() else {
                error!("Invalid step time");
                return;
            };
            let (send, recv) = channel();
            let Some(mut analyzer_taken) = analyzer.borrow_mut().take() else {
                error!("No analyzer"); return;
            };
            let step_freq = (stop_freq - start_freq) / step_count + 1;
            graph_data_locked.start_freq = start_freq as f32;
            graph_data_locked.stop_freq = stop_freq as f32;
            graph_data_locked.samples.clear();
            
            drawing_area.queue_draw();
            
            thread::spawn(move || {
                if let Err(e) = analyzer_taken.start_oneshot(600, start_freq, step_freq, step_count, step_time, &mut |i, freq, sample| {
                    send.send(DataSample::Sample(i, freq, sample)).unwrap();
                    debug!("{} - {}", freq, sample);
                }) {
                    error!("oneshot {}", e)
                }
                send.send(DataSample::Done(analyzer_taken))
            });
            glib::idle_add_local(clone!(@weak graph_data, @weak analyzer, @weak drawing_area => @default-return ControlFlow::Break, move || {
                let mut graph_data = graph_data.borrow_mut();
                match recv.try_recv() {
                    Ok(DataSample::Sample(i, freq, sample)) => {
                        let i = i as usize;
                        if graph_data.samples.len() <= i {
                            graph_data.samples.resize(i + 1, (0.0, 0.0));
                        }
                        graph_data.samples[i] = (freq as f32, sample as f32);
                        drawing_area.queue_draw();
                        ControlFlow::Continue
                    },
                    Ok(DataSample::Done(analyzer_new)) => {
                        *analyzer.borrow_mut() = Some(analyzer_new);
                        ControlFlow::Break
                    }
                    Err(TryRecvError::Empty) => {
                        ControlFlow::Continue
                    },
                    Err(TryRecvError::Disconnected) => {
                        
                        ControlFlow::Break
                    },
                }
            }));
        }));
        
        let button_sweep: Button = builder.object("button_sweep").unwrap();
        button_sweep.connect_clicked(clone!(@strong graph_data => move |_| {
            debug!("{:?}", graph_data.borrow().samples);
        }));
        
        log_win.connect_delete_event(|log_win, ev| {
            log_win.set_visible(false);
            Propagation::Stop
        });
        button_show_logs.connect_clicked(clone!(@strong log_win => move |button| {
            log_win.set_visible(true);
        }));

        win.show_all();
        log_win.show_all();
    });

    app.run();
}