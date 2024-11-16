pub struct InterruptController {
    pub int_en: u8,
    pub int_f: u8, // IF, but I can't use `if`
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IntSource {
    VBLANK = 0x1,
    LCD = 0x2,
    TIMER = 0x4,
    SERIAL = 0x8,
    JOYPAD = 0x10,
}

impl InterruptController {
    pub fn new() -> Self {
        InterruptController {
            int_en: 0,
            int_f: 0,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF0F => {
                self.int_f = val & 0x1F;
            }
            0xFFFF => {
                self.int_en = val & 0x1F;
            }
            _ => {
                unreachable!("Invalid memory access");
            }
        };
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF0F => {
                return self.int_f;
            }
            0xFFFF => {
                return self.int_en;
            }
            _ => {
                unreachable!("Invalid memory access");
            }
        };
    }

    pub fn interrupt(&mut self, int_source: IntSource) {
        self.int_f |= int_source as u8;
    }

    pub fn interrupt_clear(&mut self, int_source: IntSource) {
        self.int_f &= !(int_source as u8);
    }

    pub fn pending(&self) -> bool {
        self.int_f != 0
    }
}

impl Iterator for InterruptController {
    type Item = IntSource;

    fn next(&mut self) -> Option<Self::Item> {
        let masked: u8 = self.int_f & self.int_en;
        let mut bit_idx = 1;

        while masked != 0 {
            if (masked & bit_idx) != 0 {
                return match bit_idx {
                    0x1 => Some(IntSource::VBLANK),
                    0x2 => Some(IntSource::LCD),
                    0x4 => Some(IntSource::TIMER),
                    0x8 => Some(IntSource::SERIAL),
                    0x10 => Some(IntSource::JOYPAD),
                    _ => unreachable!("No: int flag: {} int en: {}", self.int_f, self.int_en),
                };
            }

            bit_idx <<= 1;
        }

        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled() {
        let mut int_contr = InterruptController::new();
        assert!(int_contr.next().is_none());
    }

    #[test]
    fn timer_int() {
        let mut int_contr = InterruptController::new();
        assert!(int_contr.next().is_none());

        int_contr.interrupt(IntSource::TIMER);
        // Enable the interrupt
        int_contr.write(0xFFFF, IntSource::TIMER as u8);
        assert_eq!(int_contr.next().unwrap(), IntSource::TIMER);
    }
}
