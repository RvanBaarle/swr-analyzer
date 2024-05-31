use std::str::Utf8Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("libusb error: {0}")]
    LibUsb(#[from] libusb::Error),
    #[error("device not found")]
    DeviceNotFound,
    #[error("UTF-8 encoding error in reply")]
    Encoding(#[from] Utf8Error),
    #[error("Unexpected response from device")]
    InvalidResponse,
    #[error("Provided parameter is out of range")]
    OutOfRange,
}

pub type Result<T> = std::result::Result<T, Error>;