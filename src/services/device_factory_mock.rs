use std::cell::RefCell;
use std::rc::Rc;

use crate::device::{DeviceKind, DeviceSpec, HidrawDevice};
use crate::error::QuadroError;
use crate::protocol::{RawReport, RawStatusReport, RawVirtualSensorsReport, CTRL_REPORT_ID, CTRL_REPORT_SIZE, SECONDARY_REPORT, SECONDARY_REPORT_ID, STATUS_REPORT_SIZE, VIRTUAL_SENSORS_REPORT_ID};

use super::DeviceFactory;

struct SharedMockDevice {
    buffer: Vec<u8>,
    status_buffer: Vec<u8>,
    writes: Rc<RefCell<Vec<(u8, Vec<u8>)>>>,
    spec: DeviceSpec,
}

impl HidrawDevice for SharedMockDevice {
    fn read_feature_report(&mut self) -> Result<RawReport, QuadroError> {
        let len = self.buffer.len().min(CTRL_REPORT_SIZE);
        let mut buf = vec![0u8; CTRL_REPORT_SIZE];
        buf[..len].copy_from_slice(&self.buffer[..len]);
        Ok(RawReport::from_bytes(buf, self.spec.clone()))
    }

    fn write_feature_report(&mut self, report: &RawReport) -> Result<(), QuadroError> {
        self.writes.borrow_mut().push((CTRL_REPORT_ID, report.as_bytes().to_vec()));
        Ok(())
    }

    fn commit(&mut self) -> Result<(), QuadroError> {
        self.writes.borrow_mut().push((SECONDARY_REPORT_ID, SECONDARY_REPORT.to_vec()));
        Ok(())
    }

    fn write_virtual_sensors(&mut self, report: &RawVirtualSensorsReport) -> Result<(), QuadroError> {
        self.writes.borrow_mut().push((VIRTUAL_SENSORS_REPORT_ID, report.as_bytes().to_vec()));
        Ok(())
    }

    fn read_status_report(&mut self) -> Result<RawStatusReport, QuadroError> {
        let len = self.status_buffer.len().min(STATUS_REPORT_SIZE);
        let mut buf = vec![0u8; STATUS_REPORT_SIZE];
        buf[..len].copy_from_slice(&self.status_buffer[..len]);
        Ok(RawStatusReport::from_bytes(buf, self.spec.clone()))
    }
}

pub struct MockDeviceFactory {
    buffer: Vec<u8>,
    status_buffer: Vec<u8>,
    writes: Rc<RefCell<Vec<(u8, Vec<u8>)>>>,
}

impl MockDeviceFactory {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            buffer,
            status_buffer: vec![0u8; STATUS_REPORT_SIZE],
            writes: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn with_status_buffer(mut self, status_buffer: Vec<u8>) -> Self {
        self.status_buffer = status_buffer;
        self
    }

    pub fn writes(&self) -> std::cell::Ref<'_, Vec<(u8, Vec<u8>)>> {
        self.writes.borrow()
    }
}

impl DeviceFactory for MockDeviceFactory {
    fn open(&self, _device_path: Option<&str>) -> Result<Box<dyn HidrawDevice>, QuadroError> {
        Ok(Box::new(SharedMockDevice {
            buffer: self.buffer.clone(),
            status_buffer: self.status_buffer.clone(),
            writes: Rc::clone(&self.writes),
            spec: DeviceSpec::for_device(DeviceKind::Quadro),
        }))
    }
}
