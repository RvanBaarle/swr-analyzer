use std::fmt::Debug;

use log::error;

use error::Result;

pub mod libusb;
pub mod error;
pub mod foxdelta;
pub mod dummy;
mod commands;

#[derive(Debug)]
pub struct SweepParams {
    pub noise_filter: i32,
    pub start_freq: i32,
    pub step_freq: i32,
    pub step_count: i32,
    pub step_millis: i32
}

pub trait SWRAnalyzer {
    fn version(&mut self) -> Result<String>;
    fn set_led_blink(&mut self, state: LedState) -> Result<()>;
    fn start_sweep(&mut self,
                   continuous: bool,
                   params: SweepParams,
                   f: &mut dyn FnMut(i32, i32, i32) -> std::ops::ControlFlow<()>) -> Result<()>;
}

#[derive(Debug)]
pub enum LedState {
    Off,
    Blink,
}