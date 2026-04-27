use crate::device::{DeviceKind, DeviceSpec};
use crate::error::QuadroError;
use crate::protocol::{RawReport, RawStatusReport, RawVirtualSensorsReport, CTRL_REPORT_ID, CTRL_REPORT_SIZE, SECONDARY_REPORT, SECONDARY_REPORT_ID, STATUS_REPORT_SIZE, VIRTUAL_SENSORS_REPORT_ID};
use super::HidrawDevice;

pub struct MockHidrawDevice {
    pub buffer: Vec<u8>,
    pub status_buffer: Vec<u8>,
    pub writes: Vec<(u8, Vec<u8>)>,
    pub spec: DeviceSpec,
}

impl Default for MockHidrawDevice {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            status_buffer: Vec::new(),
            writes: Vec::new(),
            spec: DeviceSpec::for_device(DeviceKind::Quadro),
        }
    }
}

impl HidrawDevice for MockHidrawDevice {
    fn read_feature_report(&mut self) -> Result<RawReport, QuadroError> {
        let len = self.buffer.len().min(CTRL_REPORT_SIZE);
        let mut buf = vec![0u8; CTRL_REPORT_SIZE];
        buf[..len].copy_from_slice(&self.buffer[..len]);
        Ok(RawReport::from_bytes(buf, self.spec))
    }

    fn write_feature_report(&mut self, report: &RawReport) -> Result<(), QuadroError> {
        self.writes.push((CTRL_REPORT_ID, report.as_bytes().to_vec()));
        Ok(())
    }

    fn commit(&mut self) -> Result<(), QuadroError> {
        self.writes.push((SECONDARY_REPORT_ID, SECONDARY_REPORT.to_vec()));
        Ok(())
    }

    fn write_virtual_sensors(&mut self, report: &RawVirtualSensorsReport) -> Result<(), QuadroError> {
        self.writes.push((VIRTUAL_SENSORS_REPORT_ID, report.as_bytes().to_vec()));
        Ok(())
    }

    fn read_status_report(&mut self) -> Result<RawStatusReport, QuadroError> {
        let len = self.status_buffer.len().min(STATUS_REPORT_SIZE);
        let mut buf = vec![0u8; STATUS_REPORT_SIZE];
        buf[..len].copy_from_slice(&self.status_buffer[..len]);
        Ok(RawStatusReport::from_bytes(buf, self.spec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device_with_buffer(data: Vec<u8>) -> MockHidrawDevice {
        MockHidrawDevice {
            buffer: data,
            ..Default::default()
        }
    }

    #[test]
    fn read_returns_report_of_correct_size() {
        let mut device = device_with_buffer(vec![0u8; CTRL_REPORT_SIZE]);

        let report = device.read_feature_report().unwrap();

        assert_eq!(report.as_bytes().len(), CTRL_REPORT_SIZE);
    }

    #[test]
    fn read_preserves_buffer_contents() {
        let mut data = vec![0u8; CTRL_REPORT_SIZE];
        data[0] = 0x42;
        data[1] = 0xFF;
        let mut device = device_with_buffer(data);

        let report = device.read_feature_report().unwrap();

        assert_eq!(report.as_bytes()[0], 0x42);
        assert_eq!(report.as_bytes()[1], 0xFF);
    }

    #[test]
    fn write_records_report_data() {
        let mut device = MockHidrawDevice::default();
        let report = RawReport::from_bytes(vec![0xAA; CTRL_REPORT_SIZE], DeviceSpec::for_device(DeviceKind::Quadro));

        device.write_feature_report(&report).unwrap();

        assert_eq!(device.writes.len(), 1);
        assert_eq!(device.writes[0].0, CTRL_REPORT_ID);
    }

    #[test]
    fn write_secondary_records_correct_payload() {
        let mut device = MockHidrawDevice::default();

        device.commit().unwrap();

        assert_eq!(device.writes.len(), 1);
        assert_eq!(device.writes[0].0, SECONDARY_REPORT_ID);
        assert_eq!(device.writes[0].1, SECONDARY_REPORT.to_vec());
    }
}
