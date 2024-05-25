use std::thread;
use std::time::Duration;

use gtk::glib::random_double;
use log::{debug, info};

use crate::protocol::{error, LedState};
use crate::protocol::error::Error;
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

    fn start_oneshot(&mut self,
                     noise_filter: i32,
                     start_frequency: i32,
                     step_frequency: i32,
                     max_step_count: i32,
                     step_millis: i32,
                     f: &mut dyn FnMut(i32, i32, i32) -> bool) -> error::Result<()> {
        debug!("Settings noise: {noise_filter}, startfreq: {start_frequency}, step: {step_frequency}, step count: {max_step_count}, step delay: {step_millis}");
        for i in 0..=max_step_count {
            let cur_freq= start_frequency + step_frequency * i;
            if !f(i, cur_freq, i) {
                info!("Scan cancelled");
                break
            }
            if i == 101 {
                return Err(Error::InvalidResponse)
            }
            thread::sleep(Duration::from_millis(step_millis as u64));
        }
        Ok(())
    }

    fn start_continuous(&mut self, noise_filter: i32, start_frequency: i32, step_frequency: i32, max_step_count: i32, step_millis: i32, f: &mut dyn FnMut(i32, i32, i32) -> bool) -> error::Result<()> {
        debug!("Settings noise: {noise_filter}, startfreq: {start_frequency}, step: {step_frequency}, step count: {max_step_count}, step delay: {step_millis}");
        'a: loop {
            let offset = (1000.0 * random_double()).floor() as i32;
            for i in 0..=max_step_count {
                let cur_freq = start_frequency + step_frequency * i;
                if !f(i, cur_freq, i + offset) {
                    info!("Scan cancelled");
                    break 'a;
                }
                if i == 101 {
                    return Err(Error::InvalidResponse);
                }
                thread::sleep(Duration::from_millis(step_millis as u64));
            }
        }
        Ok(())
    }
}