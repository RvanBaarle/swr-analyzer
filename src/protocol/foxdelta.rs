use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

use log::info;

use crate::protocol::{error, LedState, SWRAnalyzer};
use crate::protocol::commands::CommandOp;
use crate::protocol::error::Error;

pub trait SerialDevice: Read + Write {
    fn send_ack(&mut self, cmd: CommandOp) -> error::Result<()> {
        let mut buff = [0; 32];
        write!(&mut buff[..], ":{:02}\r", cmd as u16).unwrap();
        for i in 0..3 {
            self.write_all(&buff)?;
            let mut resp = [0; 32];
            self.read_exact(&mut resp)?;
            if std::str::from_utf8(&resp)?.starts_with(":ACK ") {
                return Ok(());
            }
            info!("send ACK retry {}", i + 1);
        }
        Err(Error::InvalidResponse)
    }

    fn send_and_receive(&mut self, msg: &[u8]) -> error::Result<[u8; 32]> {
        self.write_all(msg)?;
        let mut recv_buffer = [0; 32];
        self.read_exact(&mut recv_buffer)?;
        Ok(recv_buffer)
    }

    fn send_cmd(&mut self, cmd: CommandOp) -> error::Result<()> {
        let mut buff = [0; 32];
        write!(&mut buff[..], ":{:02}\r", cmd as u16).unwrap();
        self.write_all(&buff)?;
        Ok(())
    }

    fn send_cmd_param(&mut self, cmd: CommandOp, param: i32) -> error::Result<()> {
        if !(-99999999..=999999999).contains(&param) || cmd as u16 > 99 {
            return Err(Error::OutOfRange);
        }
        let mut buff = [0; 32];
        write!(&mut buff[..], ":{:02}{:09}\r", cmd as u16, param).unwrap();
        self.write_all(&buff)?;
        Ok(())
    }

    fn recv_sample(&mut self) -> error::Result<[u8; 32]> {
        let mut buffer = [0; 32];
        self.read_exact(&mut buffer)?;
        let mut send_buff = [0; 32];
        write!(&mut send_buff[..], ":\r").unwrap();
        self.write_all(&send_buff)?;
        Ok(buffer)
    }
}

impl<T: Read + Write> SerialDevice for T {}

pub struct FoxDeltaAnalyzer<D: SerialDevice> {
    serial_device: D
}

fn decode_sample(sample: [u8; 32]) -> error::Result<Vec<u16>> {
    if sample[0] != b':' || sample[1] > 7  || sample[10] != b'\r' {
        error!("Invalid sample {:?}", sample);
        return Err(Error::InvalidResponse)
    }
    let count = u16::from_le_bytes([sample[2], sample[3]]) as usize;
    let parts: Vec<u16> = sample[4..].chunks_exact(2).take(count).map(|x| {
        u16::from_le_bytes([x[0], x[1]])
    }).collect();
    Ok(parts)
}

impl<T: SerialDevice> From<T> for FoxDeltaAnalyzer<T> {
    fn from(serial_device: T) -> Self {
        Self { serial_device }
    }
}

impl<T: SerialDevice> FoxDeltaAnalyzer<T> {
    fn set_params(&mut self,
                  noise_filter: i32,
                  start_frequency: i32,
                  step_frequency: i32,
                  step_count: i32,
                  step_millis: i32) -> error::Result<()> {
        self.serial_device.send_cmd_param(CommandOp::NoiseFilter, noise_filter)?;
        self.serial_device.send_cmd_param(CommandOp::SetRFGen, 0)?;
        self.serial_device.send_cmd_param(CommandOp::StartFrequency, start_frequency)?;
        self.serial_device.send_cmd_param(CommandOp::StepFrequency, step_frequency)?;
        self.serial_device.send_cmd_param(CommandOp::StepCount, step_count)?;
        self.serial_device.send_cmd_param(CommandOp::StepTimeMillis, step_millis)?;
        Ok(())
    }
}

impl<T: SerialDevice> SWRAnalyzer for FoxDeltaAnalyzer<T> {
    fn version(&mut self) -> error::Result<String> {
        let buffer = self.serial_device.send_and_receive(":99\r".as_bytes())?;
        let mut res = std::str::from_utf8(&buffer)?;
        res = res.trim_end_matches('\0');
        if res.starts_with(":99") && res.ends_with('\r') {
            Ok(res[3..].trim_end_matches('\r').to_string())
        } else {
            Err(Error::InvalidResponse)
        }
    }

    fn set_led_blink(&mut self, state: LedState) -> error::Result<()> {
        match state {
            LedState::Off => self.serial_device.send_cmd(CommandOp::LedOff)?,
            LedState::Blink => self.serial_device.send_cmd(CommandOp::LedBlink)?,
        }
        Ok(())
    }

    fn start_sweep(&mut self,
                   continuous: bool,
                   noise_filter: i32,
                   start_frequency: i32,
                   step_frequency: i32,
                   max_step_count: i32,
                   step_millis: i32,
                   f: &mut dyn FnMut(i32, i32, i32) -> std::ops::ControlFlow<()>) -> error::Result<()> {
        
        self.set_led_blink(LedState::Blink)?;
        self.set_params(noise_filter,
                        start_frequency,
                        step_frequency,
                        max_step_count,
                        step_millis)?;
        if continuous {
            self.serial_device.send_cmd(CommandOp::SweepEnable)?;
        } else {
            self.serial_device.send_cmd(CommandOp::SweepOneshot)?;
        }
        loop {
            let sample = decode_sample(self.serial_device.recv_sample()?)?;
            if sample.is_empty() {
                break;
            }

            let cur_freq= start_frequency + step_frequency * sample[0] as i32;
            if f(sample[0] as i32, cur_freq, sample[1] as i32).is_break() {
                self.serial_device.send_cmd(CommandOp::SweepDisable)?;
            }

            thread::sleep(Duration::from_millis((step_millis / 2) as u64));
        }
        self.serial_device.send_cmd(CommandOp::SweepDisable)?;
        self.set_led_blink(LedState::Off)?;
        Ok(())
    }

    fn start_continuous(&mut self,
                        _noise_filter: i32,
                        _start_frequency: i32,
                        _step_frequency: i32,
                        _max_step_count: i32,
                        _step_millis: i32,
                        _f: &mut dyn FnMut(i32, i32, i32) -> std::ops::ControlFlow<()>
    ) -> error::Result<()> {
        unimplemented!()
    }
}
