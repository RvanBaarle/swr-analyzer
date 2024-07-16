use std::fmt::Debug;

use ::log::error;
use gtk4::glib::Propagation;
use relm4::{Component, ComponentController, Controller, gtk, WorkerController};
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;

use crate::protocol::SweepParams;
use crate::try_install_udev;
use crate::ui::controls::Controls;
use crate::ui::graph::Graph;
use crate::ui::log::LogWindow;
use crate::ui::swr_worker::{State, SwrWorker};

mod controls;
mod graph;
mod log;
mod swr_worker;
mod util;


pub struct App {
    analyzer: WorkerController<SwrWorker>,
    controls: Controller<Controls>,
    graph: Controller<Graph>,
    state: State,
    log_window: Controller<LogWindow>,
}

#[derive(Debug)]
pub enum Input {
    #[doc(hidden)]
    #[allow(private_interfaces)]
    Controls(controls::Output),
    #[doc(hidden)]
    #[allow(private_interfaces)]
    Worker(swr_worker::Output),
    ToggleLog,
    #[doc(hidden)]
    #[allow(private_interfaces)]
    StateChange(State),
}


#[relm4::component(pub)]
//noinspection RsSortImplTraitMembers
impl Component for App {
    type Input = Input;
    type Output = ();
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ApplicationWindow {
            set_title: Some("SWR analyzer"),
            set_default_size: (800, 600),
            set_size_request: (800, 600),

            gtk::Grid {
                attach[0, 0, 1, 1]= model.controls.widget(),
                attach[1, 0, 1, 1]= model.graph.widget() {},
                attach[0, 1, 1, 1]= &gtk::Button {
                    set_label: "Open log window",
                    connect_clicked[sender] => move |_| {
                        sender.input(Input::ToggleLog)
                    }
                },
                attach[1, 1, 1, 1]= &gtk::Label {
                    #[watch]
                    set_label: &model.state.to_string(),
                },
            },
            
            connect_close_request[sender] => move |_| {
                if *swr_worker::STATE.read() == State::Busy {
                    sender.input(Input::Controls(controls::Output::Cancel));
                    return Propagation::Stop
                }
                Propagation::Proceed
            },
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            Input::Controls(controls::Output::Connect { dummy }) => {
                self.analyzer.emit(swr_worker::Input::Connect { dummy });
            }
            Input::Controls(controls::Output::Disconnect) => {
                self.analyzer.emit(swr_worker::Input::Disconnect);
            }
            Input::Controls(controls::Output::Start { continuous, start_freq, stop_freq, step_count, step_millis }) => {
                self.graph.sender().emit(graph::Input::Clear {
                    x_min: start_freq as f32,
                    x_max: stop_freq as f32,
                    y_min: 0.0,
                    y_max: 1000.0,
                });

                let step_freq = (stop_freq - start_freq) / step_count + 1;
                let params = SweepParams {
                    noise_filter: 600,
                    start_freq,
                    step_freq,
                    step_count,
                    step_millis,
                };

                self.analyzer.emit(swr_worker::Input::Start {
                    continuous,
                    params,
                });
            }
            Input::Controls(controls::Output::Cancel) => {
                self.analyzer.emit(swr_worker::Input::Cancel);
            }
            Input::Worker(swr_worker::Output::Sample(sample)) => {
                self.graph.emit(graph::Input::Sample(sample));
            }
            Input::StateChange(state) => { self.state = state; }
            Input::ToggleLog => {
                self.log_window.emit(log::Input::ToggleVisible);
            }
            Input::Controls(controls::Output::Udev) => {
                if let Err(e) = try_install_udev(true) {
                    error!("error installing udev rules: {}", e);
                }
            }
        }
    }

    fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let controls = Controls::builder()
            .launch(())
            .forward(sender.input_sender(), Input::Controls);
        let graph = Graph::builder()
            .launch(root.clone().into())
            .detach();
        let log_window = LogWindow::builder()
            .launch(())
            .detach();

        let analyzer = SwrWorker::builder()
            .detach_worker(())
            .forward(sender.input_sender(), Input::Worker);

        let model = Self {
            state: State::Disconnected,
            analyzer,
            controls,
            graph,
            log_window,
        };

        let widgets = view_output!();

        swr_worker::STATE.subscribe(sender.input_sender(), |state| Input::StateChange(*state));

        ComponentParts { model, widgets }
    }
}
