use std::io::{Read, Write};
use std::time::Duration;

use log::warn;
use rusb::{DeviceHandle, GlobalContext};

use crate::protocol::commands::CommandOp;
use crate::protocol::error::{Error, Result};
use crate::protocol::foxdelta::SerialDevice;

const TIMEOUT: Duration = Duration::from_millis(2000);

pub struct SerialHID {
    handle: DeviceHandle<GlobalContext>,
}

impl Read for SerialHID {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.handle.read_interrupt(0x81, buf, TIMEOUT).map_err(std::io::Error::other)
    }
}

impl Write for SerialHID {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.write_interrupt(0x01, buf, TIMEOUT).map_err(std::io::Error::other)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SerialHID {
    pub fn new() -> Result<Self>  {
        let handle = rusb::open_device_with_vid_pid(0x04d8, 0xfe00).ok_or(Error::DeviceNotFound)?;
        handle.set_auto_detach_kernel_driver(true)?;
        handle.claim_interface(0x0)?;
        let this = Self {
            handle
        };
        Ok(this)
    }
}

impl Drop for SerialHID {
    fn drop(&mut self) {
        if let Err(e) = self.send_ack(CommandOp::Exit) {
            warn!("error on drop: {e}");
        }
    }
}