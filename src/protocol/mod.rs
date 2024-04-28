use std::io::{Read, Write};

use error::{Result, Error};
use log::info;
use crate::protocol::commands::CommandOp;

pub mod libusb;
pub mod error;
mod commands;

pub trait SWRAnalyzer {
    fn version(&mut self) -> Result<String>;
    fn set_params(&mut self, noise_filter: u32, start_frequency: u32, stop_frequency: u32, step_time: u32) -> Result<()>;
    fn set_led_blink(&mut self, state: LedState) -> Result<()>;
    fn start_rx(&mut self, step_time: u32) -> Result<()>;
}

pub trait SerialDevice: Read + Write {
    fn send_ack(&mut self, cmd: CommandOp) -> Result<()> {
        let mut buff = [0; 32];
        write!(&mut buff[..], ":{:02}\r", cmd as u16).unwrap();
        for i in 0..3 {
            self.write(&buff)?;
            let mut resp = [0; 32];
            self.read(&mut resp)?;
            if std::str::from_utf8(&resp)?.starts_with(":ACK ") {
                return Ok(());
            }
            info!("send ACK retry {}", i + 1);
        }
        Err(Error::InvalidResponse)
    }

    fn send_and_receive(&mut self, msg: &[u8], recv_buffer: &mut [u8]) -> Result<()> {
        self.write(msg)?;
        self.read(recv_buffer)?;
        Ok(())
    }

    fn send_cmd(&mut self, cmd: CommandOp) -> Result<()> {
        let mut buff = [0; 32];
        write!(&mut buff[..], ":{:02}\r", cmd as u16).unwrap();
        self.write(&buff)?;
        Ok(())
    }

    fn send_cmd_param(&mut self, cmd: CommandOp, param: i32) -> Result<()> {
        if param > 999999999 || param < -99999999 || cmd as u16 > 99 {
            return Err(Error::OutOfRange)
        }
        let mut buff = [0; 32];
        write!(&mut buff[..], ":{:02}{:09}\r", cmd as u16, param).unwrap();
        self.write(&buff)?;
        Ok(())
    }
}

impl<T: Read + Write> SerialDevice for T {}

impl dyn SerialDevice {
    
}

impl<T: SerialDevice> SWRAnalyzer for T {
    fn version(&mut self) -> Result<String> {
        let mut buffer = [0; 32];
        self.send_and_receive(":99\r".as_bytes(), &mut buffer)?;
        let mut res = std::str::from_utf8(&buffer)?;
        res = res.trim_end_matches('\0');
        if res.starts_with(":99") && res.ends_with('\r') {
            Ok(res[3..].trim_end_matches('\r').to_string())
        } else {
            Err(Error::InvalidResponse)
        }
    }

    fn set_params(&mut self, noise_filter: u32, start_frequency: u32, stop_frequency: u32, step_time: u32) -> Result<()> {
        todo!()
    }

    fn set_led_blink(&mut self, state: LedState) -> Result<()> {
        match state {
            LedState::Off => self.send_cmd(CommandOp::LedOff)?,
            LedState::Blink => self.send_cmd(CommandOp::LedBlink)?,
        }
        Ok(())
    }

    fn start_rx(&mut self, step_time: u32) -> Result<()> {
        todo!()
    }
}

pub enum LedState {
    Off, Blink
}