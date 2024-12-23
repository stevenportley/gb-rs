use core::fmt::Display;

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

impl JoypadInput {
    fn to_reg(&self) -> u8 {
        match self {
            JoypadInput::START | JoypadInput::DOWN => 0x8,
            JoypadInput::SELECT | JoypadInput::UP => 0x4,
            JoypadInput::B | JoypadInput::LEFT => 0x2,
            JoypadInput::A | JoypadInput::RIGHT => 0x1,
        }
    }

    fn is_button(&self) -> bool {
        match self {
            JoypadInput::START | JoypadInput::SELECT | JoypadInput::A | JoypadInput::B => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum JoypadDirection {
    PRESS,
    RELEASE,
}

#[derive(Clone)]
pub struct Joypad {
    dpad_state: u8,
    button_state: u8,
    reg: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            dpad_state: 0xF,
            button_state: 0xF,
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
                return (self.dpad_state & self.button_state) | self.reg;
            } else {
                return self.button_state | self.reg;
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
        let state_reg = if button.is_button() {
            &mut self.button_state
        } else {
            &mut self.dpad_state
        };

        let pressed = match direction {
            JoypadDirection::PRESS => true,
            JoypadDirection::RELEASE => false,
        };

        // Joypad input are active low,
        if pressed {
            *state_reg &= !button.to_reg();
        } else {
            *state_reg |= button.to_reg();
        }
    }

    pub fn get_state(&self) -> JoypadState {
        return JoypadState {
            joypad: self.clone(),
        };
    }
}

pub struct JoypadState {
    joypad: Joypad,
}

impl JoypadState {
    pub fn is_pressed(&self, button: JoypadInput) -> bool {
        let state_reg = if button.is_button() {
            self.joypad.button_state
        } else {
            self.joypad.dpad_state
        };

        state_reg & button.to_reg() == 0
    }
}

impl Display for JoypadState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "A: {}", self.is_pressed(JoypadInput::A))?;
        writeln!(f, "B: {}", self.is_pressed(JoypadInput::B))?;
        writeln!(f, "LEFT: {}", self.is_pressed(JoypadInput::LEFT))?;
        writeln!(f, "UP: {}", self.is_pressed(JoypadInput::UP))?;
        writeln!(f, "RIGHT: {}", self.is_pressed(JoypadInput::RIGHT))?;
        writeln!(f, "DOWN: {}", self.is_pressed(JoypadInput::DOWN))?;
        writeln!(f, "START: {}", self.is_pressed(JoypadInput::START))?;
        writeln!(f, "SELECT: {}", self.is_pressed(JoypadInput::SELECT))?;
        Ok(())
    }
}
