use std::thread;
use std::time::Duration;
use crate::protocol::libusb::SerialHID;
use crate::protocol::{LedState, SWRAnalyzer};

mod protocol;

fn main() {
    let libusb_ctx = libusb::Context::new().unwrap();
    
    let mut dev = SerialHID::new(&libusb_ctx).expect("Opening device");
    
    println!("Version: {}", dev.version().expect("get version"));
    
    dev.set_led_blink(LedState::Blink).expect("set led blink");
    thread::sleep(Duration::from_secs(2));
    dev.set_led_blink(LedState::Off).expect("set led off");
    thread::sleep(Duration::from_secs(2));
    println!("Version: {}", dev.version().expect("get version"));
    println!("Goodbye");
}

