use std::thread;
use std::time::Duration;

use log::{debug, info};
use rand::{Rng, thread_rng};

use crate::protocol::{error, LedState, SweepParams};
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

    fn start_sweep(&mut self,
                   continuous: bool,
                   params: SweepParams,
                   f: &mut dyn FnMut(i32, i32, i32) -> std::ops::ControlFlow<()>) -> error::Result<()> {
        let SweepParams { noise_filter, start_freq: start_frequency, step_freq: step_frequency, step_count: max_step_count, step_millis } = params;
        debug!("Settings noise: {noise_filter}, startfreq: {start_frequency}, step: {step_frequency}, step count: {max_step_count}, step delay: {step_millis}");
        'a: loop {
            let offset = thread_rng().gen_range(0..1000);
            for i in 0..=max_step_count {
                let cur_freq = start_frequency + step_frequency * i;
                if f(i, cur_freq, i + offset).is_break() {
                    info!("Scan cancelled");
                    break 'a;
                }
                thread::sleep(Duration::from_millis(step_millis as u64));
            }
            if !continuous { break; }
        }
        Ok(())
    }
}