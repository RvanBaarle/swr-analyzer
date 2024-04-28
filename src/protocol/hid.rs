extern crate hidraw_sys;

use std::fmt::{Display, Formatter};
use std::fs::File;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, AsRawFd};

use hidraw_sys::{hidiocgrawinfo, hidraw_devinfo};

pub struct HIDDevice {
    file: File,
    devinfo: hidraw_devinfo,
}

impl Display for HIDDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f, "BUSTYPE:{} VID:{:04x} PID:{:04x}",
            self.devinfo.bustype, self.devinfo.vendor, self.devinfo.product
        )
    }
}

pub fn device_info(file: &mut File) -> std::io::Result<hidraw_devinfo> {
    let fd = file.as_raw_fd();
    let mut info = MaybeUninit::uninit();
    unsafe {
        if hidiocgrawinfo(fd, info.as_mut_ptr()) == 0 {
            Ok(info.assume_init())
        } else {
            Err(std::io::Error::other("Invalid IOCTL result"))
        }
    }
}

pub fn enumerate_devices() -> std::io::Result<impl Iterator<Item=std::io::Result<HIDDevice>>> {
    let dev_dir = std::fs::read_dir("/dev")?;
    Ok(dev_dir.into_iter().filter_map(|x| {
        let dev = x.ok()?;
        let filename = dev.file_name().into_string().ok()?;
        if !filename.starts_with("hidraw") { return None; };
        let mut file = File::options().read(true).write(true).open(dev.path()).ok()?;
        let devinfo = device_info(&mut file);
        Some(devinfo.map(|devinfo| HIDDevice {
            file,
            devinfo,
        }))
    }))
}

pub fn find_device(vendor: i16, product: i16) -> Option<HIDDevice> {
    for dev in enumerate_devices().ok()? {
        let dev = dev.ok()?;
        if dev.devinfo.vendor == vendor && dev.devinfo.product == product {
            return Some(dev);
        }
    }
    None
}

impl HIDDevice {
    pub fn send(&mut self)
}