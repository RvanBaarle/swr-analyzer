use log::error;
use relm4::prelude::*;
use relm4::prelude::gtk::prelude::*;
use crate::ui::swr_worker;

use crate::ui::swr_worker::{State, STATE};

pub(super) struct Controls {
    start_freq: gtk::EntryBuffer,
    stop_freq: gtk::EntryBuffer,
    step_count: gtk::EntryBuffer,
    step_time: gtk::EntryBuffer,
    state: State,
}

#[derive(Copy, Clone, Debug)]
pub(super) enum Input {
    Connect,
    Continuous,
    Oneshot,
    Cancel,
    StateChange(State),
}

#[derive(Copy, Clone, Debug)]
pub(super) enum Output {
    Connect,
    Start {
        continuous: bool,
        start_freq: i32,
        stop_freq: i32,
        step_count: i32,
        step_time: i32,
    },
    Cancel,
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
                set_buffer: &model.step_time,
            },
            attach[0, 4, 1, 2]= &gtk::Button {
                set_label: "Stop",
                #[watch]
                set_sensitive: matches!(model.state, State::Busy),
                connect_clicked[sender] => move |_| sender.input(Input::Cancel)
            },
            attach[1, 4, 1, 1]= &gtk::Button {
                set_label: "Continuous",
                #[watch]
                set_sensitive: matches!(model.state, State::Idle),
                connect_clicked[sender] => move |_| sender.input(Input::Continuous)
            },
            attach[1, 5, 1, 1]= &gtk::Button {
                set_label: "Oneshot",
                #[watch]
                set_sensitive: matches!(model.state, State::Idle),
                connect_clicked[sender] => move |_| sender.input(Input::Oneshot)
            },
            attach[1, 6, 2, 1]= &gtk::Button {
                set_label: "Connect dummy",
                #[watch]
                set_visible: matches!(model.state, State::Disconnected),
                connect_clicked[sender] => move |_| sender.input(Input::Connect)
            },
        }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            Input::Connect => {
                sender.output(Output::Connect).unwrap()
            }
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
            Input::Cancel => {
                sender.output(Output::Cancel).unwrap()
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
            step_time: gtk::EntryBuffer::new(Some("10")),
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
        let step_time = self.step_time.text().parse::<i32>().or(Err("Error parsing step time"))?;
        Ok(Output::Start {
            continuous,
            start_freq,
            stop_freq,
            step_count,
            step_time,
        })
    }
}