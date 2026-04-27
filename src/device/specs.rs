use crate::protocol::{
    CTRL_REPORT_ID, CTRL_REPORT_SIZE, FAN_CTRL_OFFSETS, QUADRO_FLOW_SENSOR_OFFSET,
    QUADRO_NUM_SENSORS, QUADRO_NUM_VIRTUAL_SENSORS, QUADRO_POWER_CYCLES, QUADRO_SENSOR_START,
    QUADRO_VIRTUAL_SENSORS_START, QUADRO_FAN_SENSOR_OFFSETS, SECONDARY_REPORT_ID,
    SECONDARY_REPORT,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Quadro,
    Octo,
}

impl DeviceKind {
    pub fn from_product_id(product_id: u16) -> Option<Self> {
        match product_id {
            0xf00d => Some(DeviceKind::Quadro),
            0xf011 => Some(DeviceKind::Octo),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DeviceKind::Quadro => "QUADRO",
            DeviceKind::Octo => "OCTO",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceSpec {
    pub kind: DeviceKind,
    pub vendor_id: u16,
    pub product_id: u16,
    pub ctrl_report_id: u8,
    pub ctrl_report_size: usize,
    pub secondary_report_id: u8,
    pub secondary_report: &'static [u8],
    pub num_fans: usize,
    pub num_sensors: usize,
    pub num_virtual_sensors: usize,
    pub fan_ctrl_offsets: &'static [usize],
    pub sensor_start: usize,
    pub virtual_sensors_start: usize,
    pub flow_sensor_offset: usize,
    pub fan_sensor_offsets: &'static [usize],
    pub temp_ctrl_offset: usize,
    pub flow_pulses_ctrl_offset: usize,
    pub power_cycles_offset: usize,
}

impl DeviceSpec {
    pub fn for_device(kind: DeviceKind) -> Self {
        match kind {
            DeviceKind::Quadro => Self::quadro(),
            DeviceKind::Octo => Self::octo(),
        }
    }

    pub fn from_product_id(product_id: u16) -> Option<Self> {
        DeviceKind::from_product_id(product_id).map(Self::for_device)
    }

    fn quadro() -> Self {
        DeviceSpec {
            kind: DeviceKind::Quadro,
            vendor_id: 0x0c70,
            product_id: 0xf00d,
            ctrl_report_id: CTRL_REPORT_ID,
            ctrl_report_size: CTRL_REPORT_SIZE,
            secondary_report_id: SECONDARY_REPORT_ID,
            secondary_report: &SECONDARY_REPORT,
            num_fans: 4,
            num_sensors: QUADRO_NUM_SENSORS,
            num_virtual_sensors: QUADRO_NUM_VIRTUAL_SENSORS,
            fan_ctrl_offsets: &FAN_CTRL_OFFSETS,
            sensor_start: QUADRO_SENSOR_START,
            virtual_sensors_start: QUADRO_VIRTUAL_SENSORS_START,
            flow_sensor_offset: QUADRO_FLOW_SENSOR_OFFSET,
            fan_sensor_offsets: &QUADRO_FAN_SENSOR_OFFSETS,
            temp_ctrl_offset: 0x0A,
            flow_pulses_ctrl_offset: 0x06,
            power_cycles_offset: QUADRO_POWER_CYCLES,
        }
    }

    fn octo() -> Self {
        use crate::protocol::{
            OCTO_CTRL_REPORT_SIZE, OCTO_FAN_CTRL_OFFSETS, OCTO_FAN_SENSOR_OFFSETS,
            OCTO_FLOW_SENSOR_OFFSET, OCTO_NUM_SENSORS, OCTO_NUM_VIRTUAL_SENSORS,
            OCTO_POWER_CYCLES, OCTO_SENSOR_START, OCTO_VIRTUAL_SENSORS_START,
        };

        DeviceSpec {
            kind: DeviceKind::Octo,
            vendor_id: 0x0c70,
            product_id: 0xf011,
            ctrl_report_id: CTRL_REPORT_ID,
            ctrl_report_size: OCTO_CTRL_REPORT_SIZE,
            secondary_report_id: SECONDARY_REPORT_ID,
            secondary_report: &SECONDARY_REPORT,
            num_fans: 8,
            num_sensors: OCTO_NUM_SENSORS,
            num_virtual_sensors: OCTO_NUM_VIRTUAL_SENSORS,
            fan_ctrl_offsets: &OCTO_FAN_CTRL_OFFSETS,
            sensor_start: OCTO_SENSOR_START,
            virtual_sensors_start: OCTO_VIRTUAL_SENSORS_START,
            flow_sensor_offset: OCTO_FLOW_SENSOR_OFFSET,
            fan_sensor_offsets: &OCTO_FAN_SENSOR_OFFSETS,
            temp_ctrl_offset: 0x0A,
            flow_pulses_ctrl_offset: 0x06,
            power_cycles_offset: OCTO_POWER_CYCLES,
        }
    }
}
