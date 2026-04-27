use crate::device::HidrawDevice;
use crate::error::QuadroError;

use super::{DeviceFactory, StandardLogger};

pub struct LinuxDeviceFactory;

impl DeviceFactory for LinuxDeviceFactory {
    fn open(&self, device_path: Option<&str>) -> Result<Box<dyn HidrawDevice>, QuadroError> {
        #[cfg(target_os = "linux")]
        {
            match device_path {
                Some(_p) => {
                    let (device, _spec) = crate::device::find_device(Box::new(StandardLogger))?;
                    Ok(Box::new(device))
                }
                None => {
                    let (device, _spec) = crate::device::find_device(Box::new(StandardLogger))?;
                    Ok(Box::new(device))
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = device_path;
            Err(QuadroError::UnsupportedPlatform)
        }
    }
}
