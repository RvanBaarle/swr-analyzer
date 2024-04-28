use std::io::{Read, Write};
use std::time::Duration;

use libusb::{Context, DeviceHandle};

use crate::protocol::error::{Error, Result};
use crate::protocol::{LedState, SerialDevice, SWRAnalyzer};
use crate::protocol::commands::CommandOp;

const TIMEOUT: Duration = Duration::from_millis(2000);

pub struct SerialHID<'a> {
    handle: DeviceHandle<'a>,
}

impl Read for SerialHID<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.handle.read_interrupt(0x81, buf, TIMEOUT).map_err(|e| std::io::Error::other(e))
    }
}

impl Write for SerialHID<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.write_interrupt(0x01, buf, TIMEOUT).map_err(|e| std::io::Error::other(e))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> SerialHID<'a> {
    pub fn new(ctx: &'a Context) -> Result<Self>  {
        let mut handle = ctx.open_device_with_vid_pid(0x04d8, 0xfe00).ok_or(Error::DeviceNotFound)?;
        handle.detach_kernel_driver(0x0)?;
        handle.claim_interface(0x0)?;
        let mut this = Self {
            handle
        };
        this.set_led_blink(LedState::Off)?;
        Ok(this)
    }
}

impl Drop for SerialHID<'_> {
    fn drop(&mut self) {
        self.send_ack(CommandOp::Exit).unwrap();
        self.handle.release_interface(0x0).unwrap();
        self.handle.attach_kernel_driver(0x0).unwrap();
    }
}