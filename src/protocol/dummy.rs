use std::thread;
use std::time::Duration;
use log::debug;
use crate::protocol::LedState;
use crate::protocol::SWRAnalyzer;

pub struct Dummy;

impl SWRAnalyzer for Dummy {
    fn version(&mut self) -> crate::protocol::error::Result<String> {
        Ok("Dummy device".to_string())
    }

    fn set_led_blink(&mut self, state: LedState) -> crate::protocol::error::Result<()> {
        debug!("Leds set to {state:?}");
        thread::sleep(Duration::from_millis(1000));
        Ok(())
    }

    fn start_oneshot(&mut self, noise_filter: i32, start_frequency: i32, step_frequency: i32, max_step_count: i32, step_millis: i32, f: &mut dyn FnMut(i32, i32, i32)) -> crate::protocol::error::Result<()> {
        debug!("Settings noise: {noise_filter}, startfreq: {start_frequency}, step: {step_frequency}, step count: {max_step_count}, step delay: {step_millis}");
        for i in 0..=max_step_count {
            let cur_freq= start_frequency + step_frequency * i;
            f(i, cur_freq, i);
            thread::sleep(Duration::from_millis(step_millis as u64));
        }
        Ok(())
    }
}