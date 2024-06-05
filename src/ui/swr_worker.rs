use std::fmt::{Debug, Display, Formatter};
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use log::{error, info};
use relm4::{Component, ComponentParts, ComponentSender, Sender, SharedState};

use crate::protocol::{error, SweepParams, SWRAnalyzer};
use crate::protocol::dummy::Dummy;
use crate::protocol::foxdelta::FoxDeltaAnalyzer;
use crate::protocol::libusb::SerialHID;

pub(super) static STATE: SharedState<State> = SharedState::new();

#[derive(Debug)]
pub(super) enum Input {
    Connect { dummy: bool },
    Disconnect,
    Start {
        continuous: bool,
        params: SweepParams,
    },
    Cancel,
}

#[derive(Debug)]
pub(super) enum Output {
    Sample(Sample),
}

pub(super) struct SwrWorker {
    device: InternalState<Box<dyn SWRAnalyzer + Send>>,
}

pub(super) enum CommandOutput {
    Sample(Sample),
    Done(Box<dyn SWRAnalyzer + Send>),
}

impl Debug for CommandOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandOutput::Done(_) => write!(f, "Done"),
            CommandOutput::Sample(sample) => write!(f, "Sample({:?})", sample),
        }
    }
}

impl Component for SwrWorker {
    type CommandOutput = CommandOutput;
    type Input = Input;
    type Output = Output;

    type Init = ();
    type Root = ();
    type Widgets = ();

    fn init_root() -> Self::Root {}

    fn init(_init: Self::Init, _root: Self::Root, _sender: ComponentSender<Self>) -> relm4::ComponentParts<SwrWorker> {
        *STATE.write() = State::Disconnected;
        let model = Self {
            device: InternalState::Disconnected,
        };

        ComponentParts {
            model,
            widgets: (),
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            Input::Connect { dummy } => {
                if !matches!(self.device, InternalState::Disconnected) {
                    error!("already connected");
                    return;
                }
                match get_analyzer(dummy) {
                    Ok(device) => {
                        self.device = InternalState::Idle(device)
                    }
                    Err(e) => {
                        error!("connecting: {}", e);
                        return;
                    }
                }
                *STATE.write() = State::Idle;
            }
            Input::Disconnect => {
                if !matches!(self.device, InternalState::Idle(..)) {
                    error!("device busy or not connected");
                    return;
                }
                self.device = InternalState::Disconnected;
                *STATE.write() = State::Disconnected;
            }
            Input::Start { continuous, params } => {
                *STATE.write() = State::Busy;
                let cancel = Arc::new(AtomicBool::new(false));
                let Some(mut device) = self.device.take(cancel.clone()) else {
                    error!("device not available");
                    return;
                };
                sender.spawn_command(move |sender| {
                    let mut handler = |i, freq, sample| {
                        sender.send(CommandOutput::Sample(Sample {
                            index: i as usize,
                            freq: freq as f32,
                            value: sample as f32,
                        })).expect("output hung up");

                        if cancel.load(Ordering::Relaxed) {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    };
                    if let Err(e) =
                        device.start_sweep(
                            continuous,
                            params,
                            &mut handler,
                        )
                    {
                        error!("error during sweep: {}", e);
                    }

                    sender.send(CommandOutput::Done(device)).unwrap()
                });
            }
            Input::Cancel => {
                let InternalState::Busy { cancel } = &mut self.device else {
                    error!("device not busy");
                    return;
                };
                cancel.store(true, Ordering::Relaxed);
            }
        }
    }

    fn update_cmd(&mut self, message: Self::CommandOutput, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            CommandOutput::Sample(s) => {
                sender.output(Output::Sample(s)).unwrap()
            }
            CommandOutput::Done(device) => {
                self.device = InternalState::Idle(device);
                *STATE.write() = State::Idle;
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        if let InternalState::Busy { cancel } = &self.device {
            cancel.store(true, Ordering::Relaxed);
            // Wait a bit to make sure the thread has exited
            thread::sleep(Duration::from_millis(500));
        }
        self.device = InternalState::Disconnected;
    }
}

fn get_analyzer(use_dummy: bool) -> error::Result<Box<dyn SWRAnalyzer + Send>> {
    let mut device: Box<dyn SWRAnalyzer + Send> = if use_dummy {
        Box::new(Dummy)
    } else {
        let context = libusb::Context::new()?;
        Box::new(FoxDeltaAnalyzer::from(SerialHID::new(Arc::new(context))?))
    };
    info!("version: {}", device.version()?);
    Ok(device)
}

#[derive(Debug)]
pub struct Sample {
    pub index: usize,
    pub freq: f32,
    pub value: f32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum State {
    Disconnected,
    Idle,
    Busy,
}

impl Default for State {
    fn default() -> Self {
        Self::Disconnected
    }
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

enum InternalState<T> {
    Disconnected,
    Idle(T),
    Busy { cancel: Arc<AtomicBool> },
}

impl<T> InternalState<T> {
    fn take(&mut self, cancel: Arc<AtomicBool>) -> Option<T> {
        match std::mem::replace(self, InternalState::Busy { cancel }) {
            InternalState::Idle(device) => Some(device),
            state => {
                *self = state;
                None
            }
        }
    }
}