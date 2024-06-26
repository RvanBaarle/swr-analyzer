use log::error;
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;

use crate::ui::swr_worker::{State, STATE};

pub(super) struct Controls {
    start_freq: gtk::EntryBuffer,
    stop_freq: gtk::EntryBuffer,
    step_count: gtk::EntryBuffer,
    step_millis: gtk::EntryBuffer,
    state: State,
}

#[derive(Copy, Clone, Debug)]
pub(super) enum Input {
    Continuous,
    Oneshot,
    StateChange(State),
}

#[derive(Copy, Clone, Debug)]
pub(super) enum Output {
    Connect {
        dummy: bool,
    },
    Disconnect,
    Start {
        continuous: bool,
        start_freq: i32,
        stop_freq: i32,
        step_count: i32,
        step_millis: i32,
    },
    Cancel,
    Udev,
}

#[relm4::component(pub(super))]
//noinspection RsSortImplTraitMembers
impl SimpleComponent for Controls {
    type Input = Input;
    type Output = Output;
    type Init = ();

    view! {
        gtk::Grid {
            attach[0, 0, 1, 1]= &gtk::Label {
                set_label: "Start frequency [Hz]:",
            },
            #[name = "start_freq"]
            attach[1, 0, 1, 1]= &gtk::Entry {
                set_buffer: &model.start_freq,
            },
            attach[0, 1, 1, 1]= &gtk::Label {
                set_label: "Stop frequency [Hz]:",
            },
            #[name = "stop_freq"]
            attach[1, 1, 1, 1]= &gtk::Entry {
                set_buffer: &model.stop_freq,
            },
            attach[0, 2, 1, 1]= &gtk::Label {
                set_label: "Step count:",
            },
            #[name = "step_count"]
            attach[1, 2, 1, 1]= &gtk::Entry {
                set_buffer: &model.step_count,
            },
            attach[0, 3, 1, 1]= &gtk::Label {
                set_label: "Step time [ms]:",
            },
            #[name = "step_time"]
            attach[1, 3, 1, 1]= &gtk::Entry {
                set_buffer: &model.step_millis,
            },
            attach[0, 4, 1, 2]= &gtk::Button {
                set_label: "Stop",
                #[watch]
                set_sensitive: matches!(model.state, State::Busy),
                connect_clicked[sender] => move |_| sender.output_sender().emit(Output::Cancel)
            },
            attach[1, 4, 1, 1]= &gtk::Button {
                set_label: "Continuous",
                #[watch]
                set_sensitive: matches!(model.state, State::Idle),
                connect_clicked => Input::Continuous,
            },
            attach[1, 5, 1, 1]= &gtk::Button {
                set_label: "Oneshot",
                #[watch]
                set_sensitive: matches!(model.state, State::Idle),
                connect_clicked => Input::Oneshot
            },
            attach[1, 6, 2, 1]= &gtk::Button {
                set_label: "Connect dummy",
                #[watch]
                set_visible: matches!(model.state, State::Disconnected),
                connect_clicked[sender] => move |_| sender.output_sender().emit(Output::Connect {
                    dummy: true
                })
            },
            attach[1, 7, 2, 1]= &gtk::Button {
                set_label: "Connect Fox-Delta",
                #[watch]
                set_visible: matches!(model.state, State::Disconnected),
                connect_clicked[sender] => move |_| sender.output_sender().emit(Output::Connect {
                    dummy: false
                })
            },
            attach[1, 8, 2, 1]= &gtk::Button {
                set_label: "Install udev rules",
                #[watch]
                set_visible: matches!(model.state, State::Disconnected),
                connect_clicked[sender] => move |_| sender.output_sender().emit(Output::Udev)
            },
            attach[1, 6, 2, 1]= &gtk::Button {
                set_label: "Disconnect",
                #[watch]
                set_visible: matches!(model.state, State::Idle),
                connect_clicked[sender] => move |_| sender.output_sender().emit(Output::Disconnect)
            },
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            Input::Continuous => {
                match self.parse_parameters(true) {
                    Ok(p) => {
                        sender.output(p).unwrap()
                    },
                    Err(e) => {
                        error!("{}", e)
                    }
                }
            }
            Input::Oneshot => {
                match self.parse_parameters(false) {
                    Ok(p) => {
                        sender.output(p).unwrap()
                    },
                    Err(e) => {
                        error!("{}", e)
                    }
                }
            }
            Input::StateChange(state) =>  {
                self.state = state
            }
        }
    }

    fn init(_init: Self::Init, root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = Self {
            start_freq: gtk::EntryBuffer::new(Some("1000000")),
            stop_freq: gtk::EntryBuffer::new(Some("35000000")),
            step_count: gtk::EntryBuffer::new(Some("100")),
            step_millis: gtk::EntryBuffer::new(Some("10")),
            state: State::Disconnected,
        };
        let widgets = view_output!();
        
        STATE.subscribe(sender.input_sender(), |state| Input::StateChange(*state));

        ComponentParts { model, widgets }
    }
}

impl Controls {
    fn parse_parameters(&self, continuous: bool) -> Result<Output, &str> {
        let start_freq = self.start_freq.text().parse::<i32>().or(Err("Error parsing start frequency"))?;
        let stop_freq = self.stop_freq.text().parse::<i32>().or(Err("Error parsing stop frequency"))?;
        let step_count = self.step_count.text().parse::<i32>().or(Err("Error parsing step count"))?;
        let step_millis = self.step_millis.text().parse::<i32>().or(Err("Error parsing step time"))?;
        Ok(Output::Start {
            continuous,
            start_freq,
            stop_freq,
            step_count,
            step_millis,
        })
    }
}