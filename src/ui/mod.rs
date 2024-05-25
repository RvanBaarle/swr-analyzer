use std::cell::RefCell;
use std::default::Default;
use std::pin::pin;
use std::rc::Rc;
use std::sync::Arc;

use ::log::{error, info};
use futures::{FutureExt, select, StreamExt};
use gtk::{Application, ApplicationWindow, Button, cairo, DrawingArea, Entry, glib, TreeView, Window};
use gtk::glib::{clone, Propagation};
use gtk::prelude::*;
use plotters::prelude::*;
use plotters_cairo::CairoBackend;

use crate::protocol::{AsyncSWRAnalyzer, SWRAnalyzer};
use crate::protocol::dummy::Dummy;
use crate::protocol::error::Result;
use crate::protocol::foxdelta::FoxDeltaAnalyzer;
use crate::protocol::libusb::SerialHID;
use crate::ui::log::Logger;

mod log;

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

fn get_analyzer(use_dummy: bool) -> Result<AsyncSWRAnalyzer<Box<dyn SWRAnalyzer + Send>>> {
    let mut device: Box<dyn SWRAnalyzer + Send> = if use_dummy {
        Box::new(Dummy)
    } else {
        let context = libusb::Context::new()?;
        Box::new(FoxDeltaAnalyzer::from(SerialHID::new(Arc::new(context))?))
    };
    info!("version: {}", device.version()?);
    Ok(AsyncSWRAnalyzer::new(device))
}

pub fn ui_main() {
    let app = Application::builder()
        .application_id("nl.actuallyruben.ruben.SwrAnalyzer")
        .build();

    let glade_src = include_str!("ui.glade");

    app.connect_activate(|app| {
        let builder = gtk::Builder::from_string(glade_src);

        let win: ApplicationWindow = builder.object("window_main").unwrap();
        win.set_application(Some(app));
        let log_win: Window = builder.object("window_log").unwrap();

        let list_log: TreeView = builder.object("list_log").unwrap();
        Logger::init(win.clone(), list_log).expect("init logger");

        let analyzer: Rc<RefCell<Option<AsyncSWRAnalyzer<Box<dyn SWRAnalyzer + Send>>>>> = Default::default();

        match get_analyzer(false) {
            Ok(dev) => {
                *analyzer.borrow_mut() = Some(dev);
            }
            Err(e) => {
                error!("connection error: {}", e);
                *analyzer.borrow_mut() = Some(get_analyzer(true).expect("infallible dummy"));
            }
        }

        let button_oneshot: Button = builder.object("button_oneshot").unwrap();
        let button_sweep: Button = builder.object("button_sweep").unwrap();
        let button_stop: Button = builder.object("button_stop").unwrap();
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

        let scan_closure = |continuous: bool| clone!(
            @strong graph_data, @weak drawing_area, @strong analyzer,
            @weak input_start_freq, @weak input_stop_freq, @weak input_step_count, @weak input_step_time,
            @weak button_stop, @weak button_sweep, @weak button_oneshot
            => move |_: &Button| {
            button_sweep.set_sensitive(false);
            button_oneshot.set_sensitive(false);
            button_stop.set_sensitive(true);
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

            let step_freq = (stop_freq - start_freq) / step_count + 1;
            graph_data_locked.start_freq = start_freq as f32;
            graph_data_locked.stop_freq = stop_freq as f32;
            graph_data_locked.samples.clear();

            drawing_area.queue_draw();

            glib::spawn_future_local(clone!(@strong graph_data, @strong analyzer,
                @weak button_stop, @weak button_sweep, @weak button_oneshot
            => async move {
                let Ok(mut analyzer) = analyzer.try_borrow_mut() else {
                    error!("Analyzer busy");
                    return;
                };
                let Some(analyzer) = analyzer.as_mut() else {
                    error!("No analyzer connected");
                    return;
                };

                let iter = pin!(if continuous {
                    analyzer.start_continuous(600, start_freq, step_freq, step_count, step_time)
                } else {
                    analyzer.start_oneshot(600, start_freq, step_freq, step_count, step_time)
                });
                let mut iter = iter.fuse();
                let (cancel_trigger, cancel) = async_oneshot::oneshot();
                let cancel_trigger = RefCell::new(cancel_trigger);
                let mut cancel = cancel.fuse();

                button_stop.connect_clicked(clone!(
                    @weak button_stop, @weak button_sweep, @weak button_oneshot
                =>move |_| {
                    let _ = cancel_trigger.borrow_mut().send(());
                    button_sweep.set_sensitive(true);
                    button_oneshot.set_sensitive(true);
                    button_stop.set_sensitive(false);
                    button_stop.connect_clicked(|_| {});
                }));

                loop {
                    let Some(x) = select! {
                        x = iter.next() => x,
                        _ = cancel => break,
                        complete => break,
                    } else {
                        break
                    };
                    match x {
                        Ok((i, freq, sample)) => {
                            let mut graph_data = graph_data.borrow_mut();
                            let i = i as usize;
                            if graph_data.samples.len() <= i {
                                graph_data.samples.resize(i + 1, (0.0, 0.0));
                            }
                            graph_data.samples[i] = (freq as f32, sample as f32);
                            drawing_area.queue_draw();
                        }
                        Err(e) => {
                            error!("Sweep error: {}", e);
                            break;
                        }
                    }
                }

                let _ = iter.into_inner().cancel().await.map_err(|e| {
                    error!("Cancel error: {}", e)
                });

                button_sweep.set_sensitive(true);
                button_oneshot.set_sensitive(true);
                button_stop.set_sensitive(false);
                button_stop.connect_clicked(|_|{});
            }));
        });

        button_oneshot.connect_clicked(scan_closure(false));

        button_sweep.connect_clicked(scan_closure(true));

        log_win.connect_delete_event(clone!(@strong button_show_logs => move |log_win, _| {
            log_win.set_visible(false);
            button_show_logs.set_label("Open log window");
            Propagation::Stop
        }));
        button_show_logs.connect_clicked(clone!(@strong log_win => move |button| {
            if log_win.get_visible() {
                log_win.set_visible(false);
                button.set_label("Open log window");
            } else {
                log_win.set_visible(true);
                button.set_label("Close log window");
            }
        }));

        win.show_all();
    });

    app.run();
}