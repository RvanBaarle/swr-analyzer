use log::error;

use error::Result;

pub mod libusb;
pub mod error;
pub mod foxdelta;
pub mod dummy;
mod commands;

pub trait SWRAnalyzer {
    fn version(&mut self) -> Result<String>;
    fn set_led_blink(&mut self, state: LedState) -> Result<()>;
    fn start_oneshot(&mut self,
                        noise_filter: i32,
                        start_frequency: i32,
                        step_frequency: i32,
                        max_step_count: i32,
                        step_millis: i32,
                        f: &mut dyn FnMut(i32, i32, i32)) -> Result<()>;
}

#[derive(Debug)]
pub enum LedState {
    Off,
    Blink,
}
