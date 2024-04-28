mod protocol;

fn main() {
    for dev in protocol::hid::enumerate_devices().unwrap() {
        println!("{}", dev.unwrap());
    }
    // VID: 0x04d8 PID: 0xfe00
}
