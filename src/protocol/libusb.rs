use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use libusb::{Context, DeviceHandle};
use log::warn;

use crate::protocol::commands::CommandOp;
use crate::protocol::error::{Error, Result};
use crate::protocol::foxdelta::SerialDevice;

const TIMEOUT: Duration = Duration::from_millis(2000);

pub struct SerialHID {
    _context: Arc<Context>,
    handle: Option<DeviceHandle<'static>>,
}

impl Read for SerialHID {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.handle.as_ref().unwrap()
            .read_interrupt(0x81, buf, TIMEOUT).map_err(std::io::Error::other)
    }
}

impl Write for SerialHID {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.as_ref().unwrap()
            .write_interrupt(0x01, buf, TIMEOUT).map_err(std::io::Error::other)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SerialHID {
    pub fn new(context: Arc<Context>) -> Result<Self>  {
        let handle = context.open_device_with_vid_pid(0x04d8, 0xfe00).ok_or(Error::DeviceNotFound)?;
        let mut handle: DeviceHandle<'static> = unsafe {
            std::mem::transmute(handle)
        };
        if handle.kernel_driver_active(0x0)? {
            handle.detach_kernel_driver(0x0)?;
        }
        handle.claim_interface(0x0)?;
        let this = Self {
            _context: context,
            handle: Some(handle)
        };
        Ok(this)
    }
}

impl Drop for SerialHID {
    fn drop(&mut self) {
        if let Err(e) = self.send_ack(CommandOp::Exit) {
            warn!("error on drop: {e}");
        }
        let Some(mut handle) = self.handle.take() else {
            warn!("no handle");
            return;
        };
        if let Err(e) = handle.release_interface(0x0) {
            warn!("failed to release interface: {e}");
        }
        if let Err(e) = handle.attach_kernel_driver(0x0) {
            warn!("failed to reattach kernel driver: {e}");
        }
    }
}