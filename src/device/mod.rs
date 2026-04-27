mod mock;
mod specs;

pub use mock::MockHidrawDevice;
pub use specs::{DeviceKind, DeviceSpec};

use crate::error::QuadroError;
use crate::protocol::{RawReport, RawStatusReport, RawVirtualSensorsReport};

pub trait HidrawDevice {
    fn read_feature_report(&mut self) -> Result<RawReport, QuadroError>;
    fn write_feature_report(&mut self, report: &RawReport) -> Result<(), QuadroError>;
    fn commit(&mut self) -> Result<(), QuadroError>;
    fn read_status_report(&mut self) -> Result<RawStatusReport, QuadroError>;
    fn write_virtual_sensors(&mut self, report: &RawVirtualSensorsReport) -> Result<(), QuadroError>;
}

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::{LinuxHidrawDevice, find_device, find_quadro};
