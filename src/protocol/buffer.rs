use crc::{Crc, CRC_16_USB};

use super::centi_percent::CentiPercent;
use super::constants::*;
use super::curve_data::CurveData;
use super::fan::{FanId, FanMode};
use super::sensor_index::SensorIndex;
use super::temperature::Temperature;

const CRC_ALGO: Crc<u16> = Crc::<u16>::new(&CRC_16_USB);

pub fn read_be16(buffer: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([buffer[offset], buffer[offset + 1]])
}

pub fn write_be16(buffer: &mut [u8], offset: usize, value: u16) {
    let bytes = value.to_be_bytes();
    buffer[offset] = bytes[0];
    buffer[offset + 1] = bytes[1];
}

pub fn apply_manual(buffer: &mut [u8], fan: FanId, percentage: CentiPercent, offsets: &[usize]) {
    let base = fan.offset(offsets);
    buffer[base + FAN_MODE_OFFSET] = 0x00;
    write_be16(buffer, base + FAN_PWM_OFFSET, percentage.0);
}

pub fn apply_curve(buffer: &mut [u8], fan: FanId, curve_data: &CurveData, offsets: &[usize]) {
    let base = fan.offset(offsets);
    buffer[base + FAN_MODE_OFFSET] = 0x02;
    write_be16(buffer, base + FAN_TEMP_SELECT_OFFSET, curve_data.sensor.value() as u16);
    for (i, t) in curve_data.temps.iter().enumerate() {
        write_be16(buffer, base + FAN_TEMP_CURVE_START + i * SENSOR_SIZE, t.to_centi_degrees());
    }
    for (i, p) in curve_data.pwms.iter().enumerate() {
        write_be16(buffer, base + FAN_PWM_CURVE_START + i * SENSOR_SIZE, p.0);
    }
}

pub fn read_fan_mode(buffer: &[u8], fan: FanId, offsets: &[usize]) -> FanMode {
    let base = fan.offset(offsets);
    match buffer[base + FAN_MODE_OFFSET] {
        0x00 => FanMode::Manual,
        0x02 => FanMode::Curve,
        _ => FanMode::Manual,
    }
}

pub fn read_manual_pwm(buffer: &[u8], fan: FanId, offsets: &[usize]) -> CentiPercent {
    let base = fan.offset(offsets);
    CentiPercent(read_be16(buffer, base + FAN_PWM_OFFSET))
}

pub fn read_curve(buffer: &[u8], fan: FanId, offsets: &[usize]) -> CurveData {
    let base = fan.offset(offsets);
    let sensor_raw = read_be16(buffer, base + FAN_TEMP_SELECT_OFFSET) as u8;
    let sensor = SensorIndex::new(sensor_raw).unwrap_or_else(|_| {
        SensorIndex::new(0).expect("0 is always valid")
    });
    let mut temps = [Temperature::from_centi_degrees(0); 16];
    let mut pwms = [CentiPercent(0); 16];
    for i in 0..CURVE_NUM_POINTS {
        temps[i] = Temperature::from_centi_degrees(read_be16(buffer, base + FAN_TEMP_CURVE_START + i * SENSOR_SIZE));
        pwms[i] = CentiPercent(read_be16(buffer, base + FAN_PWM_CURVE_START + i * SENSOR_SIZE));
    }
    CurveData { sensor, temps, pwms }
}

pub fn compute_checksum(buffer: &[u8]) -> u16 {
    let checksum_length = buffer.len() - 3;
    CRC_ALGO.checksum(&buffer[CHECKSUM_START..CHECKSUM_START + checksum_length])
}

pub fn finalize(buffer: &mut [u8]) {
    let checksum = compute_checksum(buffer);
    let offset = buffer.len() - 2;
    write_be16(buffer, offset, checksum);
}

pub fn verify_checksum(buffer: &[u8]) -> bool {
    let offset = buffer.len() - 2;
    let stored = read_be16(buffer, offset);
    let computed = compute_checksum(buffer);
    stored == computed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Percentage;

    fn make_curve_test_data() -> CurveData {
        let mut temps = [Temperature::from_centi_degrees(0); 16];
        let mut pwms = [CentiPercent(0); 16];
        for i in 0..16 {
            temps[i] = Temperature::from_celsius((20 + i) as f64).unwrap();
            pwms[i] = CentiPercent::from_percentage(Percentage::new((i * 6) as u8).unwrap());
        }
        CurveData {
            sensor: SensorIndex::new(2).unwrap(),
            temps,
            pwms,
        }
    }

    #[test]
    fn write_be16_stores_high_byte_first() {
        let mut buf = [0u8; 4];

        write_be16(&mut buf, 1, 0xABCD);

        assert_eq!(buf[1], 0xAB);
    }

    #[test]
    fn write_be16_stores_low_byte_second() {
        let mut buf = [0u8; 4];

        write_be16(&mut buf, 1, 0xABCD);

        assert_eq!(buf[2], 0xCD);
    }

    #[test]
    fn read_be16_reconstructs_written_value() {
        let mut buf = [0u8; 4];

        write_be16(&mut buf, 1, 0xABCD);

        assert_eq!(read_be16(&buf, 1), 0xABCD);
    }

    #[test]
    fn manual_mode_sets_mode_byte_to_zero() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];

        apply_manual(&mut buf, FanId::Fan2, CentiPercent::from_percentage(Percentage::new(50).unwrap()));

        assert_eq!(buf[0x8b + 0x00], 0);
    }

    #[test]
    fn manual_mode_writes_pwm_value() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];

        apply_manual(&mut buf, FanId::Fan2, CentiPercent::from_percentage(Percentage::new(50).unwrap()));

        assert_eq!(read_be16(&buf, 0x8b + 0x01), 5000);
    }

    #[test]
    fn curve_mode_sets_mode_byte_to_one() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let curve_data = make_curve_test_data();

        apply_curve(&mut buf, FanId::Fan1, &curve_data);

        assert_eq!(buf[0x36 + 0x00], 2);
    }

    #[test]
    fn curve_mode_writes_sensor_id() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let curve_data = make_curve_test_data();

        apply_curve(&mut buf, FanId::Fan1, &curve_data);

        assert_eq!(read_be16(&buf, 0x36 + 0x03), 2);
    }

    #[test]
    fn curve_mode_writes_all_sixteen_temperature_and_pwm_points() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let curve_data = make_curve_test_data();

        apply_curve(&mut buf, FanId::Fan1, &curve_data);

        let base = 0x36;
        for i in 0..16 {
            assert_eq!(
                read_be16(&buf, base + FAN_TEMP_CURVE_START + i * 2),
                curve_data.temps[i].to_centi_degrees()
            );
            assert_eq!(
                read_be16(&buf, base + FAN_PWM_CURVE_START + i * 2),
                curve_data.pwms[i].0
            );
        }
    }

    #[test]
    fn zero_mode_byte_reads_as_manual() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let base = FanId::Fan1.offset();
        buf[base] = 0;

        assert_eq!(read_fan_mode(&buf, FanId::Fan1), FanMode::Manual);
    }

    #[test]
    fn mode_byte_two_reads_as_curve() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let base = FanId::Fan1.offset();
        buf[base] = 2;

        assert_eq!(read_fan_mode(&buf, FanId::Fan1), FanMode::Curve);
    }

    #[test]
    fn written_manual_config_reads_back_as_manual_mode() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];

        apply_manual(&mut buf, FanId::Fan3, CentiPercent::from_percentage(Percentage::new(75).unwrap()));

        assert_eq!(read_fan_mode(&buf, FanId::Fan3), FanMode::Manual);
    }

    #[test]
    fn written_manual_config_reads_back_same_pwm() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let pct = CentiPercent::from_percentage(Percentage::new(75).unwrap());

        apply_manual(&mut buf, FanId::Fan3, pct);

        assert_eq!(read_manual_pwm(&buf, FanId::Fan3), pct);
    }

    #[test]
    fn written_curve_config_reads_back_as_curve_mode() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let mut temps = [Temperature::from_centi_degrees(0); 16];
        let mut pwms = [CentiPercent(0); 16];
        for i in 0..16 {
            temps[i] = Temperature::from_celsius((25 + i * 2) as f64).unwrap();
            pwms[i] = CentiPercent::from_percentage(Percentage::new((10 + i * 5) as u8).unwrap());
        }
        let curve_data = CurveData {
            sensor: SensorIndex::new(1).unwrap(),
            temps,
            pwms,
        };

        apply_curve(&mut buf, FanId::Fan4, &curve_data);

        assert_eq!(read_fan_mode(&buf, FanId::Fan4), FanMode::Curve);
    }

    #[test]
    fn written_curve_config_reads_back_same_sensor() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let mut temps = [Temperature::from_centi_degrees(0); 16];
        let mut pwms = [CentiPercent(0); 16];
        for i in 0..16 {
            temps[i] = Temperature::from_celsius((25 + i * 2) as f64).unwrap();
            pwms[i] = CentiPercent::from_percentage(Percentage::new((10 + i * 5) as u8).unwrap());
        }
        let curve_data = CurveData {
            sensor: SensorIndex::new(1).unwrap(),
            temps,
            pwms,
        };

        apply_curve(&mut buf, FanId::Fan4, &curve_data);

        let read_data = read_curve(&buf, FanId::Fan4);
        assert_eq!(read_data.sensor, curve_data.sensor);
    }

    #[test]
    fn written_curve_config_reads_back_same_temperatures() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let mut temps = [Temperature::from_centi_degrees(0); 16];
        let mut pwms = [CentiPercent(0); 16];
        for i in 0..16 {
            temps[i] = Temperature::from_celsius((25 + i * 2) as f64).unwrap();
            pwms[i] = CentiPercent::from_percentage(Percentage::new((10 + i * 5) as u8).unwrap());
        }
        let curve_data = CurveData {
            sensor: SensorIndex::new(1).unwrap(),
            temps,
            pwms,
        };

        apply_curve(&mut buf, FanId::Fan4, &curve_data);

        let read_data = read_curve(&buf, FanId::Fan4);
        assert_eq!(read_data.temps, curve_data.temps);
    }

    #[test]
    fn written_curve_config_reads_back_same_pwms() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        let mut temps = [Temperature::from_centi_degrees(0); 16];
        let mut pwms = [CentiPercent(0); 16];
        for i in 0..16 {
            temps[i] = Temperature::from_celsius((25 + i * 2) as f64).unwrap();
            pwms[i] = CentiPercent::from_percentage(Percentage::new((10 + i * 5) as u8).unwrap());
        }
        let curve_data = CurveData {
            sensor: SensorIndex::new(1).unwrap(),
            temps,
            pwms,
        };

        apply_curve(&mut buf, FanId::Fan4, &curve_data);

        let read_data = read_curve(&buf, FanId::Fan4);
        assert_eq!(read_data.pwms, curve_data.pwms);
    }

    #[test]
    fn checksum_of_nonempty_buffer_is_nonzero() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        buf[1] = 0x42;
        buf[2] = 0xFF;

        let crc = compute_checksum(&buf);

        assert_ne!(crc, 0);
    }

    #[test]
    fn same_buffer_produces_identical_checksum() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        buf[1] = 0x42;
        buf[2] = 0xFF;

        let crc1 = compute_checksum(&buf);
        let crc2 = compute_checksum(&buf);

        assert_eq!(crc1, crc2);
    }

    #[test]
    fn finalize_writes_matching_checksum_to_buffer() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        buf[5] = 0xAA;

        finalize(&mut buf);

        assert_eq!(read_be16(&buf, CHECKSUM_OFFSET), compute_checksum(&buf));
    }

    #[test]
    fn finalized_buffer_passes_checksum_verification() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        buf[10] = 0xBB;
        finalize(&mut buf);

        assert!(verify_checksum(&buf));
    }

    #[test]
    fn corrupted_byte_fails_checksum_verification() {
        let mut buf = [0u8; CTRL_REPORT_SIZE];
        buf[10] = 0xBB;
        finalize(&mut buf);
        buf[10] = 0xCC;

        assert!(!verify_checksum(&buf));
    }
}
