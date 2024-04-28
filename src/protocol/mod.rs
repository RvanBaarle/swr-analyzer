use std::io::{Read, Write};

pub mod hid;

pub trait SWRAnalyzer {
    fn set_params(noise_filter: u32, start_frequency: u32, stop_frequency: u32, step_time: u32);
    fn start_rx(step_time: u32);
}

pub trait SerialDevice: Read + Write {
    fn send_ack(&mut self, msg: &[u8]) {

    }
}