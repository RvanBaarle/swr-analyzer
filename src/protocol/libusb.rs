use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use libusb::{Context, DeviceHandle};

use crate::protocol::error::{Error, Result};
use crate::protocol::{LedState, SerialDevice, SWRAnalyzer};
use crate::protocol::commands::CommandOp;

const TIMEOUT: Duration = Duration::from_millis(2000);

pub struct SerialHID {
    context: Arc<Context>,
    handle: Option<DeviceHandle<'static>>,
}

impl Read for SerialHID {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.handle.as_ref().unwrap()
            .read_interrupt(0x81, buf, TIMEOUT).map_err(|e| std::io::Error::other(e))
    }
}

impl Write for SerialHID {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.handle.as_ref().unwrap()
            .write_interrupt(0x01, buf, TIMEOUT).map_err(|e| std::io::Error::other(e))
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
        let mut this = Self {
            context,
            handle: Some(handle)
        };
        this.set_led_blink(LedState::Off)?;
        Ok(this)
    }
}

impl Drop for SerialHID {
    fn drop(&mut self) {
        self.send_ack(CommandOp::Exit).unwrap();
        let mut handle = self.handle.take().unwrap();
        handle.release_interface(0x0).unwrap();
        handle.attach_kernel_driver(0x0).unwrap();
    }
}