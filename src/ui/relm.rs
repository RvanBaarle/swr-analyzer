use std::fmt::{Debug, Display, Formatter};
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;

use async_oneshot::Sender;
use futures::{FutureExt, select, StreamExt};
use log::{error, info};
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;

use crate::protocol::{AsyncSWRAnalyzer, error::Result, SWRAnalyzer};
use crate::protocol::dummy::Dummy;
use crate::protocol::foxdelta::FoxDeltaAnalyzer;
use crate::protocol::libusb::SerialHID;
use crate::ui::relm::controls::Controls;
use crate::ui::relm::graph::{Graph, Sample};

mod controls;
mod graph;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum State {
    Disconnected,
    Idle,
    Busy,
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Disconnected => write!(f, "Disconnected"),
            State::Idle => write!(f, "Idle"),
            State::Busy => write!(f, "Busy"),
        }
    }
}

pub struct App {
    analyzer: Option<AsyncSWRAnalyzer<Box<dyn SWRAnalyzer + Send>>>,
    canceller: Option<Sender<()>>,
    controls: Controller<Controls>,
    graph: Controller<Graph>,
    state: State,
}

#[derive(Debug)]
pub enum Message {
    #[doc(hidden)]
    Controls(controls::Output)
}

pub enum CommandOutput {
    Sample(graph::Sample),
    Done(AsyncSWRAnalyzer<Box<dyn SWRAnalyzer + Send>>),
}

impl Debug for CommandOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandOutput::Sample { .. } => write!(f, "Sample"),
            CommandOutput::Done(_) => write!(f, "Done"),
        }
    }
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

#[relm4::component(pub, async)]
//noinspection RsSortImplTraitMembers
impl AsyncComponent for App {
    type Input = Message;
    type Output = ();
    type Init = ();
    type CommandOutput = CommandOutput;

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

    async fn update(&mut self, message: Self::Input, sender: AsyncComponentSender<Self>, root: &Self::Root) {
        match message {
            Message::Controls(controls::Output::Connect) => {
                if self.state != State::Disconnected {
                    error!("Already connected");
                    return;
                }
                self.analyzer = get_analyzer(true)
                    .map_err(|x| error!("{}", x)).ok();
                self.state = State::Idle;
                self.controls.model();
                self.controls.emit(controls::Input::StateChange(self.state));
            }
            Message::Controls(controls::Output::Start { continuous, start_freq, stop_freq, step_count, step_time }) => {
                let Some(mut analyzer) = self.analyzer.take() else {
                    error!("no analyzer available");
                    return;
                };
                let (canceller, cancel) = async_oneshot::oneshot();
                self.canceller = Some(canceller);
                
                self.graph.sender().emit(graph::Input::Clear {
                    start_freq: start_freq as f32,
                    stop_freq: stop_freq as f32,
                    y_min: 0.0,
                    y_max: 1000.0,
                });

                let step_freq = (stop_freq - start_freq) / step_count + 1;

                sender.command(move |sender, rcv| async move {
                    {
                        let task = pin!(if continuous {
                            analyzer.start_continuous(600, start_freq, step_freq, step_count, step_time)
                        } else {
                            analyzer.start_oneshot(600, start_freq, step_freq, step_count, step_time)
                        });
                        let mut task = task.fuse();
                        let mut cancel = pin!(cancel.fuse());

                        loop {
                            let Some(x) = select! {
                                x = task.next() => x,
                                _ = cancel => break,
                                complete => break,
                            } else {
                                break;
                            };
                            match x {
                                Ok((index, freq, value)) => {
                                    sender.send(CommandOutput::Sample( graph::Sample {
                                        index: index as usize,
                                        freq: freq as f32,
                                        value: value as f32,
                                    })).unwrap()
                                }
                                Err(e) => {
                                    error!("Sweep error: {}", e);
                                    break;
                                }
                            }
                        }
                        if let Err(e) = task.into_inner().cancel().await {
                            error!("Cancel error: {}", e)
                        };
                    }
                    
                    sender.send(CommandOutput::Done(analyzer)).unwrap();
                });
                self.state = State::Busy;
                self.controls.emit(controls::Input::StateChange(self.state));
            }
            Message::Controls(controls::Output::Cancel) => {
                let Some(mut canceller) = self.canceller.take() else {
                    error!("no task to cancel");
                    return;
                };
                if canceller.send(()).is_err() {
                    error!("no task to cancel")
                }
            }
        }
    }

    async fn update_cmd(&mut self, message: Self::CommandOutput, sender: AsyncComponentSender<Self>, root: &Self::Root) {
        match message {
            CommandOutput::Sample(sample) => {
                self.graph.emit(graph::Input::Sample(sample))
            }
            CommandOutput::Done(analyzer) => {
                self.analyzer = Some(analyzer);
                self.state = State::Idle;
                self.controls.emit(controls::Input::StateChange(self.state))
            }
        }
    }

    async fn init(init: Self::Init, root: Self::Root, sender: AsyncComponentSender<Self>) -> AsyncComponentParts<Self> {
        let controls = Controls::builder()
            .launch(())
            .forward(sender.input_sender(), Message::Controls);
        let graph = Graph::builder()
            .launch(())
            .detach();
        let model = Self {
            state: State::Disconnected,
            analyzer: None,
            canceller: None,
            controls,
            graph,
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }
}
