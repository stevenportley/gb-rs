
#[derive(Debug)]
pub enum JoypadInput {
    START,
    SELECT,
    B,
    A,
    DOWN,
    UP,
    LEFT,
    RIGHT,
}

#[derive(Debug)]
pub enum JoypadDirection {
    PRESS,
    RELEASE,
}


pub struct Joypad {
    dpad_state: u8,
    select_state: u8,
    reg: u8,
}


impl Joypad {

    pub fn new() -> Self {
        Self {
            dpad_state: 0xF,
            select_state: 0xF,
            reg: 0x30,
        }
    }

    fn select_dpad(&self) -> bool {
        return self.reg & 0x10 == 0;
    }

    fn select_buttons(&self) -> bool {
        return self.reg & 0x20 == 0;
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        if addr != 0xFF00 {
            panic!("Invalid write address to joypad!");
        }

        self.reg = val & 0x30;
    }

    pub fn read(&self, addr: u16) -> u8 {
        if addr != 0xFF00 {
            panic!("Invalid write address to joypad!");
        }

        if self.select_buttons() {
            if self.select_dpad() {
                return (self.dpad_state & self.select_state) | self.reg;
            } else {
                return self.select_state | self.reg;
            }
        } else {
            if self.select_dpad() {
                return self.dpad_state | self.reg;
            } else {
                return 0x3F;
            }
        }
    }

    pub fn input(&mut self, button: JoypadInput, direction: JoypadDirection) {
        println!("Joypad input: {:?} Direction {:?}", button, direction);

        let press = match direction {
            JoypadDirection::PRESS => true,
            JoypadDirection::RELEASE => false,
        };

        match button {
            JoypadInput::START => {
                if press {
                    self.select_state &= !0x8;
                } else {
                    self.select_state |= 0x8;
                }
            },
            JoypadInput::SELECT => {
                if press {
                    self.select_state &= !0x4;
                } else {
                    self.select_state |= 0x4;
                }
            },
            JoypadInput::B => {
                if press {
                    self.select_state &= !0x2;
                } else {
                    self.select_state |= 0x2;
                }

            },
            JoypadInput::A => 
                if press {
                    self.select_state &= !0x1;
                } else {
                    self.select_state |= 0x1;
                },
            JoypadInput::DOWN => {
                if press {
                    self.dpad_state &= !0x8;
                } else {
                    self.dpad_state |= 0x8;
                }
                },
            JoypadInput::UP => {
                if press {
                    self.dpad_state &= !0x4;
                } else {
                    self.dpad_state |= 0x4;
                }
                },
            JoypadInput::LEFT => {
                if press {
                    self.dpad_state &= !0x2;
                } else {
                    self.dpad_state |= 0x2;
                }
                },
            JoypadInput::RIGHT => {
                if press {
                    self.dpad_state &= !0x1;
                } else {
                    self.dpad_state |= 0x1;
                }
                },
        };

    }

}


