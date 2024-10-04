
pub struct Timer {
    tima: u8,
    tma: u8,
    tac: u8,
    system_counter: u16,
}


impl Timer {
    pub fn new() -> Self {
        Timer {
            tima: 0,
            tma: 0,
            tac: 0x0,
            system_counter: 0,
        }
    }

    fn enabled(&self) -> bool {
        return (self.tac & 0x4) == 0x4;
    }


    //TODO: Should handle reset of DIV (and other things?)
    //      whenever we see a HALT instruction
    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF04 => { self.system_counter = 0; },
            0xFF05 => { self.tima = val; },
            0xFF06 => { self.tma = val; },
            0xFF07 => { 
                self.tac = val; 
            },
            _ => { unreachable!("Invalid write to timer"); }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => { return (self.system_counter >> 8) as u8; },
            0xFF05 => { return self.tima; },
            0xFF06 => { return self.tma; },
            0xFF07 => { return self.tac; },
            _ => { unreachable!("Invalid write to timer"); }
        }
    }

    pub fn tick(&mut self) -> bool {
        let pre_add = self.system_counter;
        self.system_counter = self.system_counter.wrapping_add(1);

        if !self.enabled() {
            return false;
        }

        // See: https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html
        let num_shift = match self.tac & 0x3 {
            // This is really just log2 of the table on pandocs
            0 => 8,
            1 => 2,
            2 => 4,
            3 => 6,
            _ => unreachable!("No"),
        } - 1;

       
        // Check and see if the LSB triggered falling edge
        let pre_lsb = ((pre_add >> num_shift) & 1) == 1;
        let post_lsb = ((self.system_counter >> num_shift) & 1) == 1;

        if pre_lsb && !post_lsb {
            // Timer tick!
            self.tima = self.tima.wrapping_add(1);
            if self.tima == 0 {
                self.tima = self.tma;
                return true;
            }
        }

        return false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled() {
        let mut timer = Timer::new();
        assert_eq!(timer.enabled(), false);
        for _ in 0..1000 {
            assert_eq!(timer.tick(), false);
            assert_eq!(timer.read(0xFF05), 0);
        }
    }

    #[test]
    fn clock0() {
        let mut timer = Timer::new();
        timer.write(0xFF07, 0x4);
        assert_eq!(timer.enabled(), true);
        for _ in 0..255 {
            assert_eq!(timer.tick(), false);
            assert_eq!(timer.read(0xFF05), 0);
        }

        assert_eq!(timer.tick(), false);
        assert_eq!(timer.read(0xFF05), 1);
    }

    #[test]
    fn clock1() {
        let mut timer = Timer::new();
        timer.write(0xFF07, 0x5);
        assert_eq!(timer.enabled(), true);
        for _ in 0..3 {
            assert_eq!(timer.tick(), false);
            assert_eq!(timer.read(0xFF05), 0);
        }

        assert_eq!(timer.tick(), false);
        assert_eq!(timer.read(0xFF05), 1);
    }

    #[test]
    fn clock2() {
        let mut timer = Timer::new();
        timer.write(0xFF07, 0x6);
        assert_eq!(timer.enabled(), true);
        for _ in 0..15 {
            assert_eq!(timer.tick(), false);
            assert_eq!(timer.read(0xFF05), 0);
        }

        assert_eq!(timer.tick(), false);
        assert_eq!(timer.read(0xFF05), 1);
    }

    #[test]
    fn clock3() {
        let mut timer = Timer::new();
        timer.write(0xFF07, 0x7);
        assert_eq!(timer.enabled(), true);
        for _ in 0..63 {
            assert_eq!(timer.tick(), false);
            assert_eq!(timer.read(0xFF05), 0);
        }

        assert_eq!(timer.tick(), false);
        assert_eq!(timer.read(0xFF05), 1);
    }

    #[test]
    fn interrupt_basic() {
        let mut timer = Timer::new();

        // This should trigger an overflow
        // (interrupt) every timer tick
        timer.write(0xFF06, 0xFF);
        timer.write(0xFF07, 0x7);
        timer.write(0xFF05, 0xFF);
        assert_eq!(timer.enabled(), true);

        for _ in 0..5 {
            for _ in 0..63 {
                assert_eq!(timer.tick(), false);
                assert_eq!(timer.read(0xFF05), 0xFF);
            }
            assert_eq!(timer.tick(), true);
            assert_eq!(timer.read(0xFF05), 0xFF);
        }

    }

    #[test]
    fn blargg_instr_timing_incre_every_four() {
        // The blargg 'instr_timing'
        // test configures the timer
        // to increment every four ticks
        let mut timer = Timer::new();
        timer.tma = 0;
        timer.tac = 0x5;

        for i in 0..10 {

            for _ in 0..3 {
                timer.tick();
                assert_eq!(timer.tima, i);
            }

            timer.tick();
            assert_eq!(timer.tima, i+1);

        }

    }
}

