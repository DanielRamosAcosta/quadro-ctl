use serde::{Deserialize, Serialize};

use crate::protocol::{FanId, Percentage, SensorIndex, Temperature};
use super::curve::Curve;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub enum FanLabel {
    #[serde(rename = "fan1")]
    Fan1,
    #[serde(rename = "fan2")]
    Fan2,
    #[serde(rename = "fan3")]
    Fan3,
    #[serde(rename = "fan4")]
    Fan4,
    #[serde(rename = "fan5")]
    Fan5,
    #[serde(rename = "fan6")]
    Fan6,
    #[serde(rename = "fan7")]
    Fan7,
    #[serde(rename = "fan8")]
    Fan8,
}

impl From<FanLabel> for FanId {
    fn from(label: FanLabel) -> Self {
        match label {
            FanLabel::Fan1 => FanId::Fan1,
            FanLabel::Fan2 => FanId::Fan2,
            FanLabel::Fan3 => FanId::Fan3,
            FanLabel::Fan4 => FanId::Fan4,
            FanLabel::Fan5 => FanId::Fan5,
            FanLabel::Fan6 => FanId::Fan6,
            FanLabel::Fan7 => FanId::Fan7,
            FanLabel::Fan8 => FanId::Fan8,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode")]
pub enum FanConfig {
    #[serde(rename = "manual")]
    Manual { percentage: Percentage },
    #[serde(rename = "curve")]
    Curve {
        sensor: SensorIndex,
        points: Curve,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CurvePoint {
    pub temp: Temperature,
    pub percentage: Percentage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_label_fan1_converts_to_fan_id_fan1() {
        assert_eq!(FanId::from(FanLabel::Fan1), FanId::Fan1);
    }

    #[test]
    fn fan_label_fan2_converts_to_fan_id_fan2() {
        assert_eq!(FanId::from(FanLabel::Fan2), FanId::Fan2);
    }

    #[test]
    fn fan_label_fan3_converts_to_fan_id_fan3() {
        assert_eq!(FanId::from(FanLabel::Fan3), FanId::Fan3);
    }

    #[test]
    fn fan_label_fan4_converts_to_fan_id_fan4() {
        assert_eq!(FanId::from(FanLabel::Fan4), FanId::Fan4);
    }
}
