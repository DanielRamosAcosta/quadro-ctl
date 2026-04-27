use std::collections::BTreeMap;

use crate::config::FanLabel;
use crate::device::DeviceSpec;

use super::buffer;
use super::constants::*;
use super::status::{DeviceInfo, FanStatus, Status};

const DISCONNECTED_SENSOR: i16 = 0x7FFF;

pub struct RawStatusReport {
    buffer: Vec<u8>,
    spec: DeviceSpec,
}

impl RawStatusReport {
    pub fn from_bytes(bytes: Vec<u8>, spec: DeviceSpec) -> Self {
        Self { buffer: bytes, spec }
    }

    pub fn to_status(&self) -> Status {
        Status {
            device: self.parse_device_info(),
            temperatures: self.parse_temperatures(),
            fans: self.parse_fans(),
            flow: self.parse_flow(),
        }
    }

    fn parse_device_info(&self) -> DeviceInfo {
        let serial_part1 = buffer::read_be16(&self.buffer, AQC_SERIAL_START);
        let serial_part2 = buffer::read_be16(&self.buffer, AQC_SERIAL_START + 2);
        let serial = format!("{:05}-{:05}", serial_part1, serial_part2);
        let firmware = buffer::read_be16(&self.buffer, AQC_FIRMWARE_VERSION);
        let power_cycles = u32::from_be_bytes([
            self.buffer[self.spec.power_cycles_offset],
            self.buffer[self.spec.power_cycles_offset + 1],
            self.buffer[self.spec.power_cycles_offset + 2],
            self.buffer[self.spec.power_cycles_offset + 3],
        ]);
        DeviceInfo { serial, firmware, power_cycles }
    }

    fn parse_temperatures(&self) -> BTreeMap<String, Option<f64>> {
        let mut temps = BTreeMap::new();
        for i in 0..self.spec.num_sensors {
            let offset = self.spec.sensor_start + i * SENSOR_SIZE;
            let raw = buffer::read_be16(&self.buffer, offset) as i16;
            let value = if raw == DISCONNECTED_SENSOR {
                None
            } else {
                Some(raw as f64 / 100.0)
            };
            temps.insert(format!("sensor{}", i + 1), value);
        }
        for i in 0..self.spec.num_virtual_sensors {
            let offset = self.spec.virtual_sensors_start + i * SENSOR_SIZE;
            let raw = buffer::read_be16(&self.buffer, offset) as i16;
            let value = if raw == DISCONNECTED_SENSOR {
                None
            } else {
                Some(raw as f64 / 100.0)
            };
            temps.insert(format!("virtual{}", i + 1), value);
        }
        temps
    }

    fn parse_fans(&self) -> BTreeMap<FanLabel, FanStatus> {
        let all_labels = [
            FanLabel::Fan1, FanLabel::Fan2, FanLabel::Fan3, FanLabel::Fan4,
            FanLabel::Fan5, FanLabel::Fan6, FanLabel::Fan7, FanLabel::Fan8,
        ];
        all_labels.iter().take(self.spec.num_fans).enumerate().map(|(i, label)| {
            let base = self.spec.fan_sensor_offsets[i];
            let fan = FanStatus {
                pwm: buffer::read_be16(&self.buffer, base),
                voltage: buffer::read_be16(&self.buffer, base + AQC_FAN_VOLTAGE_OFFSET) as f64 / 100.0,
                current: buffer::read_be16(&self.buffer, base + AQC_FAN_CURRENT_OFFSET) as f64 / 100.0,
                power: buffer::read_be16(&self.buffer, base + AQC_FAN_POWER_OFFSET) as f64 / 100.0,
                rpm: buffer::read_be16(&self.buffer, base + AQC_FAN_SPEED_OFFSET),
            };
            (*label, fan)
        }).collect()
    }

    fn parse_flow(&self) -> f64 {
        buffer::read_be16(&self.buffer, self.spec.flow_sensor_offset) as f64 / 10.0
    }
}
