pub const CTRL_REPORT_ID: u8 = 0x03;
pub const CTRL_REPORT_SIZE: usize = 0x3c1;
pub const SECONDARY_REPORT_ID: u8 = 0x02;
pub const SECONDARY_REPORT: [u8; 11] = [
    0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x34, 0xC6,
];

pub const FAN_CTRL_OFFSETS: [usize; 4] = [0x36, 0x8b, 0xe0, 0x135];

pub const OCTO_CTRL_REPORT_SIZE: usize = 0x65f;
pub const OCTO_FAN_CTRL_OFFSETS: [usize; 8] = [0x5B, 0xB0, 0x105, 0x15A, 0x1AF, 0x204, 0x259, 0x2AE];

pub const FAN_MODE_OFFSET: usize = 0x00;
pub const FAN_PWM_OFFSET: usize = 0x01;
pub const FAN_TEMP_SELECT_OFFSET: usize = 0x03;
pub const FAN_TEMP_CURVE_START: usize = 0x15;
pub const FAN_PWM_CURVE_START: usize = 0x35;

pub const CURVE_NUM_POINTS: usize = 16;
pub const SENSOR_SIZE: usize = 0x02;
pub const CHECKSUM_START: usize = 0x01;
pub const CHECKSUM_OFFSET: usize = 0x3bf;

pub const STATUS_REPORT_SIZE: usize = 0x3c1;
pub const QUADRO_STATUS_REPORT_SIZE: usize = 0xDC;
pub const OCTO_STATUS_REPORT_SIZE: usize = 0x147;
pub const AQC_SERIAL_START: usize = 0x03;
pub const AQC_FIRMWARE_VERSION: usize = 0x0D;
pub const AQC_POWER_CYCLES: usize = 0x18;
pub const QUADRO_SENSOR_START: usize = 0x34;
pub const QUADRO_NUM_SENSORS: usize = 4;
pub const QUADRO_VIRTUAL_SENSORS_START: usize = 0x3C;
pub const QUADRO_NUM_VIRTUAL_SENSORS: usize = 16;
pub const QUADRO_FLOW_SENSOR_OFFSET: usize = 0x6E;
pub const QUADRO_FAN_SENSOR_OFFSETS: [usize; 4] = [0x70, 0x7D, 0x8A, 0x97];
pub const QUADRO_POWER_CYCLES: usize = AQC_POWER_CYCLES;
pub const AQC_FAN_VOLTAGE_OFFSET: usize = 0x02;
pub const AQC_FAN_CURRENT_OFFSET: usize = 0x04;
pub const AQC_FAN_POWER_OFFSET: usize = 0x06;
pub const AQC_FAN_SPEED_OFFSET: usize = 0x08;

pub const OCTO_NUM_SENSORS: usize = 4;
pub const OCTO_NUM_VIRTUAL_SENSORS: usize = 16;
pub const OCTO_POWER_CYCLES: usize = 0x18;
pub const OCTO_SENSOR_START: usize = 0x3D;
pub const OCTO_VIRTUAL_SENSORS_START: usize = 0x45;
pub const OCTO_FLOW_SENSOR_OFFSET: usize = 0x7B;
pub const OCTO_FAN_SENSOR_OFFSETS: [usize; 8] = [0x7D, 0x8A, 0x97, 0xA4, 0xB1, 0xBE, 0xCB, 0xD8];

pub const VIRTUAL_SENSORS_REPORT_ID: u8 = 0x04;
pub const VIRTUAL_SENSORS_REPORT_SIZE: usize = 0x43;
pub const VIRTUAL_SENSORS_VALUES_OFFSET: usize = 0x01;
pub const VIRTUAL_SENSORS_TYPES_OFFSET: usize = 0x21;
pub const VIRTUAL_SENSORS_UNKNOWN_OFFSET: usize = 0x31;
pub const VIRTUAL_SENSORS_UNKNOWN_BYTE: u8 = 0x64;
pub const VIRTUAL_SENSOR_DISABLED_VALUE: u16 = 0x7FFF;
pub const VIRTUAL_SENSOR_TYPE_DISABLED: u8 = 0;
pub const VIRTUAL_SENSOR_TYPE_TEMPERATURE: u8 = 3;
