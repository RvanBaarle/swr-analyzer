use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use error::{Result, Error};
use log::{debug, error, info, warn};
use crate::protocol::commands::CommandOp;

pub mod libusb;
pub mod error;
mod commands;

pub trait SWRAnalyzer {
    fn version(&mut self) -> Result<String>;
    fn set_params(&mut self,
                  noise_filter: i32,
                  start_frequency: i32,
                  step_frequency: i32,
                  max_step_count: i32,
                  step_millis: i32) -> Result<()>;
    fn set_led_blink(&mut self, state: LedState) -> Result<()>;
    fn start_oneshot(&mut self,
                        noise_filter: i32,
                        start_frequency: i32,
                        step_frequency: i32,
                        max_step_count: i32,
                        step_millis: i32,
                        f: &mut dyn FnMut(i32, i32, i32)) -> Result<()>;
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
            return Err(Error::OutOfRange);
        }
        let mut buff = [0; 32];
        write!(&mut buff[..], ":{:02}{:09}\r", cmd as u16, param).unwrap();
        self.write(&buff)?;
        Ok(())
    }
    
    fn recv_sample(&mut self) -> Result<[u8; 32]> {
        let mut buffer = [0; 32];
        self.read(&mut buffer)?;
        let mut send_buff = [0; 32];
        write!(&mut send_buff[..], ":\r").unwrap();
        self.write(&send_buff)?;
        Ok(buffer)
    }
}

impl<T: Read + Write> SerialDevice for T {}

fn decode_sample(sample: [u8; 32]) -> Result<Vec<u16>> {
    if sample[0] != ':' as u8 || sample[1] > 7  || sample[10] != '\r' as u8 {
        error!("Invalid sample {:?}", sample);
        return Err(Error::InvalidResponse)
    }
    let count = u16::from_le_bytes([sample[2], sample[3]]) as usize;
    let parts: Vec<u16> = sample[4..].chunks_exact(2).take(count).map(|x| {
        u16::from_le_bytes([x[0], x[1]])
    }).collect();
    Ok(parts)
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

    fn set_params(&mut self,
                  noise_filter: i32,
                  start_frequency: i32,
                  step_frequency: i32,
                  step_count: i32,
                  step_millis: i32) -> Result<()> {
        self.send_cmd_param(CommandOp::NoiseFilter, noise_filter)?;
        self.send_cmd_param(CommandOp::SetRFGen, 0)?;
        self.send_cmd_param(CommandOp::StartFrequency, start_frequency)?;
        self.send_cmd_param(CommandOp::StepFrequency, step_frequency)?;
        self.send_cmd_param(CommandOp::StepCount, step_count)?;
        self.send_cmd_param(CommandOp::StepTimeMillis, step_millis)?;
        Ok(())
    }

    fn set_led_blink(&mut self, state: LedState) -> Result<()> {
        match state {
            LedState::Off => self.send_cmd(CommandOp::LedOff)?,
            LedState::Blink => self.send_cmd(CommandOp::LedBlink)?,
        }
        Ok(())
    }

    fn start_oneshot(&mut self,
                     noise_filter: i32,
                     start_frequency: i32,
                     step_frequency: i32,
                     max_step_count: i32,
                     step_millis: i32,
                     f: &mut dyn FnMut(i32, i32, i32)) -> Result<()> {
        self.set_led_blink(LedState::Blink)?;
        self.set_params(noise_filter,
                        start_frequency,
                        step_frequency,
                        max_step_count,
                        step_millis)?;
        self.send_cmd(CommandOp::SweepOneshot)?;
        loop {
            let sample = decode_sample(self.recv_sample()?)?;
            if sample.is_empty() {
                break;
            }
            let cur_freq= start_frequency + step_frequency * sample[0] as i32;
            f(sample[0] as i32, cur_freq, sample[1] as i32);
            
            thread::sleep(Duration::from_millis((step_millis / 2) as u64));
        }
        self.send_cmd(CommandOp::SweepDisable)?;
        self.set_led_blink(LedState::Off)?;
        Ok(())
    }
}

pub enum LedState {
    Off,
    Blink,
}