#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanId {
    Fan1,
    Fan2,
    Fan3,
    Fan4,
    Fan5,
    Fan6,
    Fan7,
    Fan8,
}

impl FanId {
    pub fn offset(&self, offsets: &[usize]) -> usize {
        offsets[self.index()]
    }

    pub fn index(&self) -> usize {
        match self {
            FanId::Fan1 => 0,
            FanId::Fan2 => 1,
            FanId::Fan3 => 2,
            FanId::Fan4 => 3,
            FanId::Fan5 => 4,
            FanId::Fan6 => 5,
            FanId::Fan7 => 6,
            FanId::Fan8 => 7,
        }
    }

    pub fn all_up_to(count: usize) -> Vec<FanId> {
        let all = [
            FanId::Fan1,
            FanId::Fan2,
            FanId::Fan3,
            FanId::Fan4,
            FanId::Fan5,
            FanId::Fan6,
            FanId::Fan7,
            FanId::Fan8,
        ];
        all.iter().take(count).copied().collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanMode {
    Manual,
    Curve,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::FAN_CTRL_OFFSETS;

    #[test]
    fn fan1_offset_is_0x36() {
        assert_eq!(FanId::Fan1.offset(&FAN_CTRL_OFFSETS), 0x36);
    }

    #[test]
    fn fan2_offset_is_0x8b() {
        assert_eq!(FanId::Fan2.offset(&FAN_CTRL_OFFSETS), 0x8b);
    }

    #[test]
    fn fan3_offset_is_0xe0() {
        assert_eq!(FanId::Fan3.offset(&FAN_CTRL_OFFSETS), 0xe0);
    }

    #[test]
    fn fan4_offset_is_0x135() {
        assert_eq!(FanId::Fan4.offset(&FAN_CTRL_OFFSETS), 0x135);
    }
}
