use std::thread;
use std::time::{Duration, Instant};
use log::{debug, info, Metadata, Record};
use crate::protocol::libusb::SerialHID;

use crate::protocol::SWRAnalyzer;
use crate::ui::ui_main;

mod protocol;
mod ui;

fn main() {
    thread::spawn(|| {
        ui_main();
    });
    
    thread::sleep(Duration::from_millis(500));
    let libusb_ctx = libusb::Context::new().unwrap();
    let mut dev = SerialHID::new(&libusb_ctx).expect("Opening device");
    info!("connected to device");
    
    info!("Version: {}", dev.version().expect("get version"));
    
    // dev.set_led_blink(LedState::Blink).expect("set led blink");
    // thread::sleep(Duration::from_secs(2));
    // dev.set_led_blink(LedState::Off).expect("set led off");
    // thread::sleep(Duration::from_secs(2));
    // info!("Version: {}", dev.version().expect("get version"));
    info!("Getting samples");
    
    let start_time = Instant::now();
    
    dev.start_oneshot(
        600, 1000000, 100000, 100, 10,
        |i, freq, level| {
            debug!("{}, {}: {}", i, freq, level);
        }
    ).expect("oneshot sweep");
    
    info!("Goodbye, avg step duration: {:?}", (Instant::now() - start_time) / 100);
    thread::sleep(Duration::from_secs(10));
}

