use std::fmt::Debug;

use async_oneshot::Sender;
use relm4::{Component, ComponentController, Controller, gtk, WorkerController};
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;

use crate::ui::controls::Controls;
use crate::ui::graph::Graph;
use crate::ui::swr_worker::{State, SwrWorker};

mod controls;
mod graph;
pub(crate) mod log;
pub(self) mod swr_worker;


pub struct App {
    analyzer: WorkerController<SwrWorker>,
    controls: Controller<Controls>,
    graph: Controller<Graph>,
    state: State,
}

#[derive(Debug)]
pub enum Message {
    #[doc(hidden)]
    #[allow(private_interfaces)]
    Controls(controls::Output),
    #[doc(hidden)]
    #[allow(private_interfaces)]
    Worker(swr_worker::Output),
    StateChange(State),
}


#[relm4::component(pub)]
//noinspection RsSortImplTraitMembers
impl Component for App {
    type Input = Message;
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
                attach[1, 0, 1, 1]= model.graph.widget() {
                    set_hexpand: true,
                    set_vexpand: true,
                },
                attach[0, 1, 1, 1]= &gtk::Button {
                    set_label: "Open log window",
                },
                attach[1, 1, 1, 1]= &gtk::Label {
                    #[watch]
                    set_label: &model.state.to_string(),
                },
            },
        }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            Message::Controls(controls::Output::Connect) => {
                self.analyzer.emit(swr_worker::Input::Connect { dummy: true });
            }
            Message::Controls(controls::Output::Start { continuous, start_freq, stop_freq, step_count, step_time }) => {
                self.graph.sender().emit(graph::Input::Clear {
                    start_freq: start_freq as f32,
                    stop_freq: stop_freq as f32,
                    y_min: 0.0,
                    y_max: 1000.0,
                });

                let step_freq = (stop_freq - start_freq) / step_count + 1;

                self.analyzer.emit(swr_worker::Input::Start {
                    continuous,
                    start_freq,
                    step_freq,
                    step_count,
                    step_time,
                });
            }
            Message::Controls(controls::Output::Cancel) => {
                self.analyzer.emit(swr_worker::Input::Cancel);
            }
            Message::Worker(swr_worker::Output::Sample(sample)) => {
                self.graph.emit(graph::Input::Sample(sample));
            }
            Message::StateChange(state) => { self.state = state; }
        }
    }

    fn init(_: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let controls = Controls::builder()
            .launch(())
            .forward(sender.input_sender(), Message::Controls);
        let graph = Graph::builder()
            .launch(())
            .detach();
        let analyzer = SwrWorker::builder()
            .detach_worker(())
            .forward(sender.input_sender(), Message::Worker);
        let model = Self {
            state: State::Disconnected,
            analyzer,
            controls,
            graph,
        };

        let widgets = view_output!();

        swr_worker::STATE.subscribe(sender.input_sender(), |state| Message::StateChange(*state));

        ComponentParts { model, widgets }
    }
}
