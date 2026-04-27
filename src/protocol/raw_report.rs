use std::collections::BTreeMap;

use crate::config::{Curve, CurvePoint, FanConfig, FanLabel};
use crate::device::DeviceSpec;
use crate::error::QuadroError;

use super::buffer;
use super::centi_percent::CentiPercent;
use super::curve_data::CurveData;
use super::fan::{FanId, FanMode};
use super::report::Report;
use super::temperature::Temperature;

pub struct RawReport {
    buffer: Vec<u8>,
    spec: DeviceSpec,
}

impl RawReport {
    pub fn from_bytes(bytes: Vec<u8>, spec: DeviceSpec) -> Self {
        Self { buffer: bytes, spec }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    pub fn verify_checksum(&self) -> bool {
        buffer::verify_checksum(&self.buffer)
    }

    fn fan_labels(&self) -> Vec<(FanLabel, FanId)> {
        let all_labels = [
            (FanLabel::Fan1, FanId::Fan1),
            (FanLabel::Fan2, FanId::Fan2),
            (FanLabel::Fan3, FanId::Fan3),
            (FanLabel::Fan4, FanId::Fan4),
            (FanLabel::Fan5, FanId::Fan5),
            (FanLabel::Fan6, FanId::Fan6),
            (FanLabel::Fan7, FanId::Fan7),
            (FanLabel::Fan8, FanId::Fan8),
        ];
        all_labels.iter().take(self.spec.num_fans).copied().collect()
    }

    pub fn to_report(&self) -> Result<Report, QuadroError> {
        let mut fans = BTreeMap::new();
        let offsets = &self.spec.fan_ctrl_offsets;
        for (label, fan_id) in &self.fan_labels() {
            let mode = buffer::read_fan_mode(&self.buffer, *fan_id, offsets);
            let fan_config = match mode {
                FanMode::Manual => {
                    let pwm = buffer::read_manual_pwm(&self.buffer, *fan_id, offsets);
                    FanConfig::Manual {
                        percentage: pwm.to_percentage(),
                    }
                }
                FanMode::Curve => {
                    let curve_data = buffer::read_curve(&self.buffer, *fan_id, offsets);
                    let points: Vec<CurvePoint> = curve_data
                        .temps
                        .iter()
                        .zip(curve_data.pwms.iter())
                        .map(|(t, p)| CurvePoint {
                            temp: *t,
                            percentage: p.to_percentage(),
                        })
                        .collect();
                    FanConfig::Curve {
                        sensor: curve_data.sensor,
                        points: Curve::new(points)?,
                    }
                }
            };
            fans.insert(*label, fan_config);
        }
        Ok(Report { fans })
    }

    pub fn with_report(&self, report: &Report) -> RawReport {
        let mut buf = self.buffer.clone();
        let offsets = &self.spec.fan_ctrl_offsets;

        for (label, fan_id) in &self.fan_labels() {
            if let Some(fan_config) = report.fans.get(label) {
                match fan_config {
                    FanConfig::Manual { percentage } => {
                        let cp = CentiPercent::from_percentage(*percentage);
                        buffer::apply_manual(&mut buf, *fan_id, cp, offsets);
                    }
                    FanConfig::Curve { sensor, points } => {
                        let mut temps = [Temperature::from_centi_degrees(0); 16];
                        let mut pwms = [CentiPercent(0); 16];
                        for (i, point) in points.points().iter().enumerate() {
                            temps[i] = point.temp;
                            pwms[i] = CentiPercent::from_percentage(point.percentage);
                        }
                        let curve_data = CurveData {
                            sensor: *sensor,
                            temps,
                            pwms,
                        };
                        buffer::apply_curve(&mut buf, *fan_id, &curve_data, offsets);
                    }
                }
            }
        }

        buffer::finalize(&mut buf);
        RawReport { buffer: buf, spec: self.spec.clone() }
    }
}
