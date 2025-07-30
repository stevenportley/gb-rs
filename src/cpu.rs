use crate::{
    bus::{Bus, Device},
    cart::CartridgeData,
    interrupts::IntSource,
};

#[inline(always)]
fn does_bit3_overflow(a: u8, b: u8) -> bool {
    let a = a & 0xF;
    let b = b & 0xF;

    return (0xF - a) < b;
}

#[inline(always)]
fn does_bit11_overflow(a: u16, b: u16) -> bool {
    let a = a & 0xFFF;
    let b = b & 0xFFF;

    return (0xFFF - a) < b;
}

#[inline(always)]
fn does_bit3_borrow(a: u8, b: u8) -> bool {
    let a = a & 0xF;
    let b = b & 0xF;
    return b > a;
}

pub struct Cpu<T: CartridgeData> {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,

    z_f: bool,
    n_f: bool,
    h_f: bool,
    c_f: bool,

    ime: bool,

    pub sleep: bool,
    pub bus: Bus<T>,
}

const PAGE0_OFFSET: u16 = 0xFF00;

// Bit indices to address particular register
const A_REG: u8 = 7;
const HL_REG: u8 = 2;

const HL_PTR: u8 = 6;

#[derive(Debug)]
pub enum Reg {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
    BC,
    DE,
    HL,
    SP,
}

/*
 * TODO: Replace decode with this to save space?
#[derive(Debug)]
pub enum Instruction2 {
    Nop,                              // 0x00 - No operation
    Load { src: Reg, dest: Reg },      // LD src, dest
    LoadImm8 { reg: Reg, value: u8 },  // LD reg, #8-bit value
    LoadImm16 { reg: Reg, value: u16 },// LD reg, #16-bit value
    LoadMemToReg { reg: Reg, addr: u16 },  // LD (addr), reg
    LoadRegToMem { reg: Reg, addr: u16 },  // LD reg, (addr)
    Add { reg: Reg },                  // ADD reg
    AddImm8 { value: u8 },             // ADD A, #8-bit value
    Sub { reg: Reg },                  // SUB reg
    And { reg: Reg },                  // AND reg
    Or { reg: Reg },                   // OR reg
    Xor { reg: Reg },                  // XOR reg
    Compare { reg: Reg },              // CP reg
    Inc { reg: Reg },                  // INC reg
    Dec { reg: Reg },                  // DEC reg
    Jump { addr: u16 },                // JP addr
    JumpRelative { offset: i8 },       // JR offset
    Call { addr: u16 },                // CALL addr
    Return,                            // RET
    Halt,                              // STOP
    Di,                                // DI (Disable interrupts)
    Ei,                                // EI (Enable interrupts)
    Rst { vector: u16 },               // RST vector
    Rotate { reg: Reg, direction: char }, // RLC, RL, etc.
    BitTest { reg: Reg, bit: u8 },      // BIT bit, reg
    Swap { reg: Reg },                 // SWAP reg
    Shift { reg: Reg, direction: char }, // SLA, SRA, etc.
    Undefined(u8),                     // For undefined opcodes
}
*/

impl<T: CartridgeData> Cpu<T> {
    #[inline(always)]
    fn no_op(_cpu: &mut Self, _opcode: u8) -> u8 {
        1
    }

    #[inline(always)]
    fn ld_r16_imm16(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        let imm16 = cpu.load_word();
        cpu.wreg16(r16, imm16);
        3
    }

    #[inline(always)]
    fn ld_r16mem_a(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        cpu.wr16mem(r16, cpu.a);
        2
    }

    #[inline(always)]
    fn inc_r16(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        let plus_one = cpu.rreg16(r16).wrapping_add(1);
        cpu.wreg16(r16, plus_one);
        2
    }

    #[inline(always)]
    fn inc_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = (opcode >> 3) & 0x7;
        let before = cpu.rreg8(r8);
        cpu.h_f = does_bit3_overflow(before, 1);
        cpu.n_f = false;
        let incre = before.wrapping_add(1);
        cpu.z_f = incre == 0;
        cpu.wreg8(r8, incre);

        return if r8 == HL_PTR { 3 } else { 1 };
    }

    #[inline(always)]
    fn dec_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = (opcode >> 3) & 0x7;
        let before = cpu.rreg8(r8);
        let dec = before.wrapping_sub(1);
        cpu.wreg8(r8, dec);

        cpu.z_f = dec == 0;
        cpu.h_f = does_bit3_borrow(before, 1);
        cpu.n_f = true;

        return if r8 == HL_PTR { 3 } else { 1 };
    }

    #[inline(always)]
    fn ld_r8_imm8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = (opcode >> 3) & 0x7;
        let imm8 = cpu.load_byte();
        cpu.wreg8(r8, imm8);
        return if r8 == HL_PTR { 3 } else { 2 };
    }

    #[inline(always)]
    fn rlca(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rlc(A_REG);
        cpu.z_f = false;
        1
    }

    #[inline(always)]
    fn ld_imm16_sp(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();

        // This is why: https://rgbds.gbdev.io/docs/v0.8.0/gbz80.7#LD__n16_,SP
        cpu.bus.write(imm16, cpu.sp as u8);
        cpu.bus.write(imm16 + 1, (cpu.sp >> 8) as u8);
        5
    }

    #[inline(always)]
    fn add_hl_r16(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        let hl_val = cpu.rreg16(HL_REG);
        let reg_val = cpu.rreg16(r16);
        let (new_val, overflow) = hl_val.overflowing_add(reg_val);
        cpu.wreg16(HL_REG, new_val);
        cpu.n_f = false;
        cpu.h_f = does_bit11_overflow(hl_val, reg_val);
        cpu.c_f = overflow;
        2
    }

    #[inline(always)]
    fn ld_a_r16mem(cpu: &mut Self, opcode: u8) -> u8 {
        let r16mem = (opcode >> 4) & 0x3;
        cpu.a = cpu.rr16mem(r16mem);
        2
    }

    #[inline(always)]
    fn dec_r16(cpu: &mut Self, opcode: u8) -> u8 {
        let r16 = (opcode >> 4) & 0x3;
        let minus_one = cpu.rreg16(r16).wrapping_sub(1);
        cpu.wreg16(r16, minus_one);
        2
    }

    #[inline(always)]
    fn rrca(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rrc(A_REG);
        cpu.z_f = false;
        1
    }

    #[inline(always)]
    fn stop(_cpu: &mut Self, opcode: u8) -> u8 {
        todo!("Stop instruction not implemented! opcode: {}", opcode);
    }

    #[inline(always)]
    fn rla(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rl(A_REG);
        cpu.z_f = false;
        1
    }

    #[inline(always)]
    fn jr_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let offset = cpu.load_byte() as i8;
        let curr_pc = cpu.pc as i32;
        let new_pc = curr_pc + offset as i32;
        cpu.pc = new_pc as u16;
        return 3;
    }

    #[inline(always)]
    fn rra(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.rr(A_REG);
        cpu.z_f = false;
        1
    }

    #[inline(always)]
    fn jr_cond_imm8(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        let imm8 = cpu.load_byte();
        if cpu.check_cond(cond) {
            let offset = imm8 as i8;
            let curr_pc = cpu.pc as i32;
            let new_pc = curr_pc + offset as i32;

            cpu.pc = new_pc as u16;
            return 3;
        }
        return 2;
    }

    #[inline(always)]
    fn daa(cpu: &mut Self, _opcode: u8) -> u8 {
        //https://forums.nesdev.org/viewtopic.php?t=15944
        if cpu.n_f {
            if cpu.c_f {
                cpu.a = cpu.a.wrapping_sub(0x60);
            }
            if cpu.h_f {
                cpu.a = cpu.a.wrapping_sub(0x6);
            }
        } else {
            if cpu.c_f || cpu.a > 0x99 {
                cpu.a = cpu.a.wrapping_add(0x60);
                cpu.c_f = true;
            }
            if cpu.h_f || (cpu.a & 0x0f) > 0x09 {
                cpu.a = cpu.a.wrapping_add(0x6);
            }
        }

        cpu.z_f = cpu.a == 0;
        cpu.h_f = false;
        1
    }

    #[inline(always)]
    fn cpl(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.a = !cpu.a;
        cpu.n_f = true;
        cpu.h_f = true;
        1
    }

    #[inline(always)]
    fn scf(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = true;
        1
    }

    #[inline(always)]
    fn ccf(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = !cpu.c_f;
        return 1;
    }

    #[inline(always)]
    fn ld_r8_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let src = opcode & 0x7;
        let dst = (opcode >> 3) & 0x7;
        let d = cpu.rreg8(src);
        cpu.wreg8(dst, d);
        return if (dst == HL_PTR) || (src == HL_PTR) {
            2
        } else {
            1
        };
    }

    #[inline(always)]
    fn halt(cpu: &mut Self, _opcode: u8) -> u8 {
        if cpu.ime {
            cpu.sleep = true;
            return 1;
        }

        if !cpu.bus.interrupt_pending() {
            cpu.sleep = true;
            return 1;
        }

        //TODO: Handle HALT bug
        //assert!(false);
        return 1;
    }

    #[inline(always)]
    fn add_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        let reg_val = cpu.rreg8(r8);

        cpu.h_f = does_bit3_overflow(reg_val, cpu.a);

        let (new_val, does_overflow) = cpu.a.overflowing_add(reg_val);
        cpu.c_f = does_overflow;
        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.a = new_val;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn adc_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        let reg_val = cpu.rreg8(r8);
        cpu.n_f = false;
        cpu.h_f = does_bit3_overflow(cpu.a, reg_val);

        let (added, overflow) = reg_val.overflowing_add(cpu.a);

        if cpu.c_f {
            // Next two conditionals check if the carry will overflow
            if does_bit3_overflow(added, 1) {
                cpu.h_f = true;
            }

            cpu.c_f = overflow || added == 0xFF;
            cpu.a = added.wrapping_add(1);
        } else {
            cpu.c_f = overflow;
            cpu.a = added;
        }

        cpu.z_f = cpu.a == 0;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn sub_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x07;
        let reg_val = cpu.rreg8(r8);
        let new_val = cpu.a.wrapping_sub(reg_val);

        cpu.h_f = does_bit3_borrow(cpu.a, reg_val);
        cpu.c_f = reg_val > cpu.a;
        cpu.n_f = true;
        cpu.z_f = new_val == 0;

        cpu.a = new_val;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn sbc_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x07;
        let carry = if cpu.c_f { 1 } else { 0 };
        let reg_val = cpu.rreg8(r8);

        cpu.c_f = (reg_val as u16 + carry as u16) > cpu.a as u16;

        cpu.h_f = does_bit3_borrow(cpu.a, reg_val);

        cpu.n_f = true;
        cpu.a = cpu.a.wrapping_sub(reg_val);

        if does_bit3_borrow(cpu.a, carry) {
            cpu.h_f = true;
        }

        cpu.a = cpu.a.wrapping_sub(carry);
        cpu.z_f = cpu.a == 0;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn and_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        cpu.a = cpu.a & cpu.rreg8(r8);
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = true;
        cpu.c_f = false;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn xor_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        cpu.a = cpu.a ^ cpu.rreg8(r8);
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn or_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        cpu.a = cpu.a | cpu.rreg8(r8);
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn cp_a_r8(cpu: &mut Self, opcode: u8) -> u8 {
        let r8 = opcode & 0x7;
        let reg_val = cpu.rreg8(r8);

        cpu.h_f = does_bit3_borrow(cpu.a, reg_val);
        cpu.c_f = reg_val > cpu.a;
        cpu.n_f = true;
        let new_val = cpu.a.wrapping_sub(reg_val);
        cpu.z_f = new_val == 0;

        return if r8 == HL_PTR { 2 } else { 1 };
    }

    #[inline(always)]
    fn add_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.h_f = does_bit3_overflow(imm8, cpu.a);

        let (new_val, does_overflow) = imm8.overflowing_add(cpu.a);
        cpu.c_f = does_overflow;
        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.a = new_val;
        2
    }

    #[inline(always)]
    fn adc_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.n_f = false;
        cpu.h_f = does_bit3_overflow(cpu.a, imm8);

        let (added, overflow) = cpu.a.overflowing_add(imm8);

        if cpu.c_f {
            // Next two conditionals check if the carry will overflow
            if does_bit3_overflow(added, 1) {
                cpu.h_f = true;
            }

            cpu.c_f = overflow || added == 0xFF;
            cpu.a = added.wrapping_add(1);
        } else {
            cpu.c_f = overflow;
            cpu.a = added;
        }

        cpu.z_f = cpu.a == 0;
        return 2;
    }

    #[inline(always)]
    fn sub_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        let new_val = cpu.a.wrapping_sub(imm8);

        cpu.h_f = does_bit3_borrow(cpu.a, imm8);
        cpu.c_f = imm8 > cpu.a;
        cpu.n_f = true;
        cpu.z_f = new_val == 0;
        cpu.a = new_val;

        2
    }

    #[inline(always)]
    fn sbc_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        let carry = if cpu.c_f { 1 } else { 0 };

        cpu.c_f = (imm8 as u16 + carry as u16) > cpu.a as u16;
        cpu.h_f = does_bit3_borrow(cpu.a, imm8);
        cpu.n_f = true;
        cpu.a = cpu.a.wrapping_sub(imm8);

        if does_bit3_borrow(cpu.a, carry) {
            cpu.h_f = true;
        }

        cpu.a = cpu.a.wrapping_sub(carry);
        cpu.z_f = cpu.a == 0;
        2
    }

    #[inline(always)]
    fn and_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.a & imm8;
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = true;
        cpu.c_f = false;
        2
    }

    #[inline(always)]
    fn xor_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.a ^ imm8;
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        2
    }

    #[inline(always)]
    fn or_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.a | imm8;
        cpu.z_f = if cpu.a == 0 { true } else { false };
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        2
    }

    #[inline(always)]
    fn cp_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();

        cpu.h_f = does_bit3_borrow(cpu.a, imm8);
        cpu.c_f = imm8 > cpu.a;

        cpu.n_f = true;
        let new_val = cpu.a.wrapping_sub(imm8);
        cpu.z_f = new_val == 0;
        2
    }

    #[inline(always)]
    fn ret_cond(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        if !cpu.check_cond(cond) {
            return 2;
        }

        cpu.pc = cpu.pop_stack();
        return 5;
    }

    #[inline(always)]
    fn pop_r16stk(cpu: &mut Self, opcode: u8) -> u8 {
        let r16stk = (opcode >> 4) & 0x3;
        match r16stk {
            0 => {
                let val = cpu.pop_stack();
                cpu.b = (val >> 8) as u8;
                cpu.c = (val & 0xFF) as u8;
            }
            1 => {
                let val = cpu.pop_stack();
                cpu.d = (val >> 8) as u8;
                cpu.e = (val & 0xFF) as u8;
            }
            2 => {
                let val = cpu.pop_stack();
                cpu.h = (val >> 8) as u8;
                cpu.l = (val & 0xFF) as u8;
            }
            3 => {
                let val = cpu.pop_stack();
                cpu.a = (val >> 8) as u8;
                cpu.z_f = (val & 0x80) == 0x80;
                cpu.n_f = (val & 0x40) == 0x40;
                cpu.h_f = (val & 0x20) == 0x20;
                cpu.c_f = (val & 0x10) == 0x10;
            }
            _ => {
                unreachable!("Invalid pop_r16stk");
            }
        }
        3
    }

    #[inline(always)]
    fn push_r16stk(cpu: &mut Self, opcode: u8) -> u8 {
        let r16stk = (opcode >> 4) & 0x3;
        let val = match r16stk {
            0 => ((cpu.b as u16) << 8) | (cpu.c as u16),
            1 => ((cpu.d as u16) << 8) | (cpu.e as u16),
            2 => ((cpu.h as u16) << 8) | (cpu.l as u16),
            3 => {
                let mut val = (cpu.a as u16) << 8;
                val = if cpu.z_f { val | 0x80 } else { val };
                val = if cpu.n_f { val | 0x40 } else { val };
                val = if cpu.h_f { val | 0x20 } else { val };
                val = if cpu.c_f { val | 0x10 } else { val };
                val
            }
            _ => {
                unreachable!("Invalid push_r16stk");
            }
        };

        cpu.push_stack(val);
        return 4;
    }

    #[inline(always)]
    fn ret(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.pc = cpu.pop_stack();
        return 4;
    }

    #[inline(always)]
    fn reti(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.ime = true;
        cpu.pc = cpu.pop_stack();
        4
    }

    #[inline(always)]
    fn jp_cond_imm16(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        let imm16 = cpu.load_word();
        if cpu.check_cond(cond) {
            cpu.pc = imm16;
            return 4;
        }

        return 3;
    }

    #[inline(always)]
    fn jp_imm16(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.pc = imm16;
        4
    }

    #[inline(always)]
    fn call_cond_imm16(cpu: &mut Self, opcode: u8) -> u8 {
        let cond = (opcode >> 3) & 0x3;
        let imm16 = cpu.load_word();

        if !cpu.check_cond(cond) {
            return 3;
        }

        cpu.push_stack(cpu.pc);
        cpu.pc = imm16;
        return 6;
    }

    #[inline(always)]
    fn call_imm16(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.push_stack(cpu.pc);
        cpu.pc = imm16;
        6
    }

    #[inline(always)]
    fn rst_tgt3(cpu: &mut Self, opcode: u8) -> u8 {
        cpu.push_stack(cpu.pc);
        let tgt = (opcode >> 3) & 0x7;
        cpu.pc = (tgt as u16) << 3;
        return 4;
    }

    #[inline(always)]
    fn invalid(_cpu: &mut Self, opcode: u8) -> u8 {
        panic!("Received invalid instruction! opcode: {}", opcode);
    }

    #[inline(always)]
    fn ldh_imm8_a(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.bus.write(imm8 as u16 + PAGE0_OFFSET, cpu.a);
        3
    }

    #[inline(always)]
    fn ldh_c_a(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.bus.write(PAGE0_OFFSET + cpu.c as u16, cpu.a);
        2
    }

    #[inline(always)]
    fn add_sp_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        let s_i = imm8 as i8;

        let new_sp = cpu.sp.wrapping_add(s_i as u16);
        cpu.z_f = false;
        cpu.n_f = false;
        cpu.h_f = does_bit3_overflow(imm8, cpu.sp as u8);

        let (_, does_bit7_of) = (cpu.sp as u8).overflowing_add(imm8);
        cpu.c_f = does_bit7_of;

        cpu.sp = new_sp;
        4
    }

    #[inline(always)]
    fn jp_hl(cpu: &mut Self, _opcode: u8) -> u8 {
        let hl = ((cpu.h as u16) << 8) | cpu.l as u16;
        cpu.pc = hl;
        1
    }

    #[inline(always)]
    fn ld_imm16_a(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.bus.write(imm16, cpu.a);
        4
    }

    #[inline(always)]
    fn ldh_a_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        cpu.a = cpu.bus.read(imm8 as u16 + PAGE0_OFFSET);
        3
    }

    #[inline(always)]
    fn ldh_a_c(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.a = cpu.bus.read(cpu.c as u16 + PAGE0_OFFSET);
        2
    }

    #[inline(always)]
    fn prefix(cpu: &mut Self, _opcode: u8) -> u8 {
        let next_byte = cpu.load_byte();
        let cycles = match next_byte {
            0..=0x7 => Self::prefix_rlc(cpu, next_byte),
            0x8..=0xF => Self::prefix_rrc(cpu, next_byte),
            0x10..=0x17 => Self::prefix_rl(cpu, next_byte),
            0x18..=0x1F => Self::prefix_rr(cpu, next_byte),
            0x20..=0x27 => Self::prefix_sla(cpu, next_byte),
            0x28..=0x2f => Self::prefix_sra(cpu, next_byte),
            0x30..=0x37 => Self::prefix_swap(cpu, next_byte),
            0x38..=0x3F => Self::prefix_srl(cpu, next_byte),
            0x40..=0x7F => Self::prefix_bit(cpu, next_byte),
            0x80..=0xBF => Self::prefix_res(cpu, next_byte),
            0xC0..=0xFF => Self::prefix_set(cpu, next_byte),
        };

        cycles
    }

    #[inline(always)]
    fn di(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.ime = false;
        1
    }

    #[inline(always)]
    fn ei(cpu: &mut Self, _opcode: u8) -> u8 {
        cpu.ime = true;
        1
    }

    #[inline(always)]
    fn ld_hl_sp_imm8(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm8 = cpu.load_byte();
        let sp = cpu.sp as i32;
        let i_8 = imm8 as i8;
        let new_hl = (sp + i_8 as i32) as u16;

        cpu.z_f = false;
        cpu.n_f = false;
        cpu.h_f = does_bit3_overflow(imm8 as u8, cpu.sp as u8);

        // Checking for signed byte add overflow
        let (_, does_of) = (cpu.sp as u8).overflowing_add(imm8);

        cpu.c_f = does_of;
        cpu.h = (new_hl >> 8) as u8;
        cpu.l = (new_hl & 0xFF) as u8;
        3
    }

    #[inline(always)]
    fn ld_sp_hl(cpu: &mut Self, _opcode: u8) -> u8 {
        let new_sp = ((cpu.h as u16) << 8) | cpu.l as u16;
        cpu.sp = new_sp;
        2
    }

    #[inline(always)]
    fn ld_a_imm16(cpu: &mut Self, _opcode: u8) -> u8 {
        let imm16 = cpu.load_word();
        cpu.a = cpu.bus.read(imm16);
        4
    }

    #[inline(always)]
    fn prefix_rlc(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rlc(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_rrc(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rrc(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_rl(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rl(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_rr(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        cpu.rr(r);
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_sla(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let new_val = reg_val << 1;
        cpu.wreg8(r, new_val);

        cpu.c_f = (reg_val & 0x80) == 0x80;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.z_f = new_val == 0;
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_sra(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let msb = reg_val & 0x80;
        let new_val = (reg_val >> 1) | msb;
        cpu.wreg8(r, new_val);

        cpu.c_f = (reg_val & 0x1) == 0x1;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.z_f = new_val == 0;
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_swap(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let upper_nibble = reg_val >> 4;
        let lower_nibble = reg_val & 0xF;
        let new_val = (lower_nibble << 4) | upper_nibble;
        cpu.wreg8(r, new_val);

        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = false;
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_srl(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let reg_val = cpu.rreg8(r);
        let new_val = reg_val >> 1;
        cpu.wreg8(r, new_val);

        cpu.z_f = new_val == 0;
        cpu.n_f = false;
        cpu.h_f = false;
        cpu.c_f = (reg_val & 0x1) == 0x1;
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_bit(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let b3 = (opcode >> 3) & 0x7;

        let reg_val = cpu.rreg8(r);
        let mask = 0x1 << b3;

        cpu.z_f = (reg_val & mask) == 0;
        cpu.n_f = false;
        cpu.h_f = true;
        return if r == HL_PTR { 3 } else { 2 };
    }

    #[inline(always)]
    fn prefix_res(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let b3 = (opcode >> 3) & 0x7;

        let reg_val = cpu.rreg8(r);
        let mask = 0x1 << b3;
        let new_val = reg_val & !mask;
        cpu.wreg8(r, new_val);
        return if r == HL_PTR { 4 } else { 2 };
    }

    #[inline(always)]
    fn prefix_set(cpu: &mut Self, opcode: u8) -> u8 {
        let r = opcode & 0x7;
        let b3 = (opcode >> 3) & 0x7;

        let reg_val = cpu.rreg8(r);
        let mask = 0x1 << b3;
        let new_val = reg_val | mask;
        cpu.wreg8(r, new_val);
        return if r == HL_PTR { 4 } else { 2 };
    }

    //TODO: Add an API to build the CPU in a state that
    //      does not skip the boot rom
    pub fn new(bus: Bus<T>) -> Self {
        Cpu {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
            h_f: true,
            c_f: true,
            n_f: false,
            z_f: true,
            ime: false,
            sleep: false,
            bus,
        }

        // I don't remember exactly why this was
        // here, but I think this might be here
        // back when the PPU was just a bank of memory
        // and we needed 0xFF44 to always return 0x90
        // to make it work with the Gameboy Doctor tests.
        // TODO: Make sure this isn't needed anymore and remove
        // this dead code.  We shouldn't need GB doctor anymore.
        //
        // Temporary to make LCD work with test ROMs
        // cpu.bus.write(0xFF44, 0x90);
    }

    #[inline(always)]
    fn rreg8(&mut self, dst: u8) -> u8 {
        match dst {
            0 => return self.b,
            1 => return self.c,
            2 => return self.d,
            3 => return self.e,
            4 => return self.h,
            5 => return self.l,
            6 => {
                // This is a special case, instead of setting a register,
                // we use the memory location pointed to by the HL register
                let hl = ((self.h as u16) << 8) | (self.l as u16);
                return self.bus.read(hl);
            }
            7 => return self.a,
            _ => unreachable!("rreg8 with invalid bit index! {dst}"),
        }
    }

    #[inline(always)]
    fn wreg8(&mut self, dst: u8, val: u8) {
        match dst {
            0 => self.b = val,
            1 => self.c = val,
            2 => self.d = val,
            3 => self.e = val,
            4 => self.h = val,
            5 => self.l = val,
            6 => {
                // This is a special case, instead of setting a register,
                // we use the memory location pointed to by the HL register
                let hl = ((self.h as u16) << 8) | (self.l as u16);
                self.bus.write(hl, val);
            }
            7 => self.a = val,
            _ => unreachable!("Set reg8 with invalid bit index! {dst}"),
        }
    }

    #[inline(always)]
    fn wreg16(&mut self, dst: u8, val: u16) {
        let high: u8 = (val >> 8) as u8;
        let low: u8 = val as u8;

        match dst {
            0 => {
                self.b = high;
                self.c = low;
            }
            1 => {
                self.d = high;
                self.e = low;
            }
            2 => {
                self.h = high;
                self.l = low;
            }
            3 => {
                self.sp = val;
            }
            _ => {
                unreachable!("Set reg16 with invalid bit index! {dst}")
            }
        }
    }

    #[inline(always)]
    fn rreg16(&mut self, dst: u8) -> u16 {
        let make_u16 = |h, l| -> u16 { (h as u16) << 8 | (l as u16) };

        match dst {
            0 => return make_u16(self.b, self.c),
            1 => return make_u16(self.d, self.e),
            2 => return make_u16(self.h, self.l),
            3 => return self.sp,
            _ => unreachable!("rreg16 with invalid bit index! {dst}"),
        }
    }

    #[inline(always)]
    fn rr16mem(&mut self, r16mem: u8) -> u8 {
        let make_u16 = |h, l| -> u16 { (h as u16) << 8 | (l as u16) };
        match r16mem {
            0 => return self.bus.read(make_u16(self.b, self.c)),
            1 => return self.bus.read(make_u16(self.d, self.e)),
            2 => {
                let mut hl = make_u16(self.h, self.l);
                let ret = self.bus.read(hl);
                hl = hl + 1;
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
                return ret;
            }
            3 => {
                let mut hl = make_u16(self.h, self.l);
                let ret = self.bus.read(hl);
                hl = hl.wrapping_sub(1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
                return ret;
            }
            _ => unreachable!("rr16mem with invalid bit index! {r16mem}"),
        }
    }

    #[inline(always)]
    fn check_cond(&self, cond: u8) -> bool {
        match cond {
            0 => return !self.z_f,
            1 => return self.z_f,
            2 => return !self.c_f,
            3 => return self.c_f,
            _ => unreachable!("invalid check_cond"),
        }
    }

    #[inline(always)]
    fn wr16mem(&mut self, r16mem: u8, val: u8) {
        let make_u16 = |h, l| -> u16 { (h as u16) << 8 | (l as u16) };
        match r16mem {
            0 => self.bus.write(make_u16(self.b, self.c), val),
            1 => self.bus.write(make_u16(self.d, self.e), val),
            2 => {
                let mut hl = make_u16(self.h, self.l);
                self.bus.write(hl, val);
                hl = hl.wrapping_add(1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
            }
            3 => {
                let mut hl = make_u16(self.h, self.l);
                self.bus.write(hl, val);
                hl = hl.wrapping_sub(1);
                self.h = (hl >> 8) as u8;
                self.l = (hl & 0xFF) as u8;
            }
            _ => unreachable!("wr16mem with invalid bit index! {r16mem}"),
        }
    }

    #[inline(always)]
    fn push_stack(&mut self, val: u16) {
        self.sp = self.sp - 1;
        self.bus.write(self.sp, (val >> 8) as u8);
        self.sp = self.sp - 1;
        self.bus.write(self.sp, (val & 0xFF) as u8);
    }

    #[inline(always)]
    fn pop_stack(&mut self) -> u16 {
        let mut ret = self.bus.read(self.sp) as u16;
        self.sp = self.sp + 1;
        ret |= (self.bus.read(self.sp) as u16) << 8;
        self.sp = self.sp + 1;
        return ret;
    }

    #[inline(always)]
    fn load_byte(&mut self) -> u8 {
        let next_byte = self.bus.read(self.pc);
        self.pc += 1;
        return next_byte;
    }

    #[inline(always)]
    fn load_word(&mut self) -> u16 {
        let next = self.load_byte();
        let next_next = self.load_byte();

        return (next as u16) | ((next_next as u16) << 8);
    }

    #[inline(always)]
    fn rlc(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let msb = reg_val >> 7;
        let new_val = (reg_val << 1) | msb;
        self.c_f = msb == 1;
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.wreg8(reg, (reg_val << 1) | msb);
    }

    #[inline(always)]
    fn rl(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let msb = reg_val >> 7;
        let new_lsb = if self.c_f { 1 } else { 0 };
        let new_val = (reg_val << 1) | new_lsb;

        self.wreg8(reg, new_val);
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.c_f = msb == 1;
    }

    #[inline(always)]
    fn rrc(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let lsb = reg_val & 1;
        let new_val = (reg_val >> 1) | (lsb << 7);

        self.wreg8(reg, new_val);
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.c_f = lsb == 1;
    }

    #[inline(always)]
    fn rr(&mut self, reg: u8) {
        let reg_val = self.rreg8(reg);
        let new_msb = if self.c_f { 0x80 } else { 0 };
        let new_val = (reg_val >> 1) | new_msb;

        self.wreg8(reg, new_val);
        self.z_f = new_val == 0;
        self.n_f = false;
        self.h_f = false;
        self.c_f = (reg_val & 1) == 1;
    }

    #[inline(always)]
    pub fn is_passed(&self) -> bool {
        return self.bus.is_passed();
    }

    pub fn run_one(&mut self) -> usize {
        // Review this and make sure all four conditions are handled correctly
        // with IME and HALT
        if self.sleep {
            if self.bus.interrupt_pending() {
                self.sleep = false;
            }

            self.bus.run_cycles(1);
            return 1;
        }

        if self.ime {
            if let Some(interrupt) = self.bus.query_interrupt() {
                let cycles = self.handle_interrupt(interrupt);
                self.bus.run_cycles(cycles as u16);
                return cycles;
            }
        }

        let opcode = self.bus.read(self.pc);
        self.pc += 1;
        let cycles = match opcode {
            0x00 => Self::no_op(self, opcode),
            0x01 => Self::ld_r16_imm16(self, opcode),
            0x02 => Self::ld_r16mem_a(self, opcode),
            0x03 => Self::inc_r16(self, opcode),
            0x04 => Self::inc_r8(self, opcode),
            0x05 => Self::dec_r8(self, opcode),
            0x06 => Self::ld_r8_imm8(self, opcode),
            0x07 => Self::rlca(self, opcode),
            0x08 => Self::ld_imm16_sp(self, opcode),
            0x09 => Self::add_hl_r16(self, opcode),
            0x0A => Self::ld_a_r16mem(self, opcode),
            0x0B => Self::dec_r16(self, opcode),
            0x0C => Self::inc_r8(self, opcode),
            0x0D => Self::dec_r8(self, opcode),
            0x0E => Self::ld_r8_imm8(self, opcode),
            0x0F => Self::rrca(self, opcode),
            0x10 => Self::stop(self, opcode),
            0x11 => Self::ld_r16_imm16(self, opcode),
            0x12 => Self::ld_r16mem_a(self, opcode),
            0x13 => Self::inc_r16(self, opcode),
            0x14 => Self::inc_r8(self, opcode),
            0x15 => Self::dec_r8(self, opcode),
            0x16 => Self::ld_r8_imm8(self, opcode),
            0x17 => Self::rla(self, opcode),
            0x18 => Self::jr_imm8(self, opcode),
            0x19 => Self::add_hl_r16(self, opcode),
            0x1A => Self::ld_a_r16mem(self, opcode),
            0x1B => Self::dec_r16(self, opcode),
            0x1C => Self::inc_r8(self, opcode),
            0x1D => Self::dec_r8(self, opcode),
            0x1E => Self::ld_r8_imm8(self, opcode),
            0x1F => Self::rra(self, opcode),
            0x20 => Self::jr_cond_imm8(self, opcode),
            0x21 => Self::ld_r16_imm16(self, opcode),
            0x22 => Self::ld_r16mem_a(self, opcode),
            0x23 => Self::inc_r16(self, opcode),
            0x24 => Self::inc_r8(self, opcode),
            0x25 => Self::dec_r8(self, opcode),
            0x26 => Self::ld_r8_imm8(self, opcode),
            0x27 => Self::daa(self, opcode),
            0x28 => Self::jr_cond_imm8(self, opcode),
            0x29 => Self::add_hl_r16(self, opcode),
            0x2A => Self::ld_a_r16mem(self, opcode),
            0x2B => Self::dec_r16(self, opcode),
            0x2C => Self::inc_r8(self, opcode),
            0x2D => Self::dec_r8(self, opcode),
            0x2E => Self::ld_r8_imm8(self, opcode),
            0x2F => Self::cpl(self, opcode),
            0x30 => Self::jr_cond_imm8(self, opcode),
            0x31 => Self::ld_r16_imm16(self, opcode),
            0x32 => Self::ld_r16mem_a(self, opcode),
            0x33 => Self::inc_r16(self, opcode),
            0x34 => Self::inc_r8(self, opcode),
            0x35 => Self::dec_r8(self, opcode),
            0x36 => Self::ld_r8_imm8(self, opcode),
            0x37 => Self::scf(self, opcode),
            0x38 => Self::jr_cond_imm8(self, opcode),
            0x39 => Self::add_hl_r16(self, opcode),
            0x3A => Self::ld_a_r16mem(self, opcode),
            0x3B => Self::dec_r16(self, opcode),
            0x3C => Self::inc_r8(self, opcode),
            0x3D => Self::dec_r8(self, opcode),
            0x3E => Self::ld_r8_imm8(self, opcode),
            0x3F => Self::ccf(self, opcode),
            0x40..=0x75 | 0x77..=0x7F => Self::ld_r8_r8(self, opcode),
            0x76 => Self::halt(self, opcode),
            0x80..=0x87 => Self::add_a_r8(self, opcode),
            0x88..=0x8F => Self::adc_a_r8(self, opcode),
            0x90..=0x97 => Self::sub_a_r8(self, opcode),
            0x98..=0x9F => Self::sbc_a_r8(self, opcode),
            0xA0..=0xA7 => Self::and_a_r8(self, opcode),
            0xA8..=0xAF => Self::xor_a_r8(self, opcode),
            0xB0..=0xB7 => Self::or_a_r8(self, opcode),
            0xB8..=0xBF => Self::cp_a_r8(self, opcode),
            0xC0 => Self::ret_cond(self, opcode),
            0xC1 => Self::pop_r16stk(self, opcode),
            0xC2 => Self::jp_cond_imm16(self, opcode),
            0xC3 => Self::jp_imm16(self, opcode),
            0xC4 => Self::call_cond_imm16(self, opcode),
            0xC5 => Self::push_r16stk(self, opcode),
            0xC6 => Self::add_a_imm8(self, opcode),
            0xC7 => Self::rst_tgt3(self, opcode),
            0xC8 => Self::ret_cond(self, opcode),
            0xC9 => Self::ret(self, opcode),
            0xCA => Self::jp_cond_imm16(self, opcode),
            0xCB => Self::prefix(self, opcode),
            0xCC => Self::call_cond_imm16(self, opcode),
            0xCD => Self::call_imm16(self, opcode),
            0xCE => Self::adc_a_imm8(self, opcode),
            0xCF => Self::rst_tgt3(self, opcode),
            0xD0 => Self::ret_cond(self, opcode),
            0xD1 => Self::pop_r16stk(self, opcode),
            0xD2 => Self::jp_cond_imm16(self, opcode),
            0xD3 => Self::invalid(self, opcode),
            0xD4 => Self::call_cond_imm16(self, opcode),
            0xD5 => Self::push_r16stk(self, opcode),
            0xD6 => Self::sub_a_imm8(self, opcode),
            0xD7 => Self::rst_tgt3(self, opcode),
            0xD8 => Self::ret_cond(self, opcode),
            0xD9 => Self::reti(self, opcode),
            0xDA => Self::jp_cond_imm16(self, opcode),
            0xDB => Self::invalid(self, opcode),
            0xDC => Self::call_cond_imm16(self, opcode),
            0xDD => Self::invalid(self, opcode),
            0xDE => Self::sbc_a_imm8(self, opcode),
            0xDF => Self::rst_tgt3(self, opcode),
            0xE0 => Self::ldh_imm8_a(self, opcode),
            0xE1 => Self::pop_r16stk(self, opcode),
            0xE2 => Self::ldh_c_a(self, opcode),
            0xE3 => Self::invalid(self, opcode),
            0xE4 => Self::invalid(self, opcode),
            0xE5 => Self::push_r16stk(self, opcode),
            0xE6 => Self::and_a_imm8(self, opcode),
            0xE7 => Self::rst_tgt3(self, opcode),
            0xE8 => Self::add_sp_imm8(self, opcode),
            0xE9 => Self::jp_hl(self, opcode),
            0xEA => Self::ld_imm16_a(self, opcode),
            0xEB => Self::invalid(self, opcode),
            0xEC => Self::invalid(self, opcode),
            0xED => Self::invalid(self, opcode),
            0xEE => Self::xor_a_imm8(self, opcode),
            0xEF => Self::rst_tgt3(self, opcode),
            0xF0 => Self::ldh_a_imm8(self, opcode),
            0xF1 => Self::pop_r16stk(self, opcode),
            0xF2 => Self::ldh_a_c(self, opcode),
            0xF3 => Self::di(self, opcode),
            0xF4 => Self::invalid(self, opcode),
            0xF5 => Self::push_r16stk(self, opcode),
            0xF6 => Self::or_a_imm8(self, opcode),
            0xF7 => Self::rst_tgt3(self, opcode),
            0xF8 => Self::ld_hl_sp_imm8(self, opcode),
            0xF9 => Self::ld_sp_hl(self, opcode),
            0xFA => Self::ld_a_imm16(self, opcode),
            0xFB => Self::ei(self, opcode),
            0xFC => Self::invalid(self, opcode),
            0xFD => Self::invalid(self, opcode),
            0xFE => Self::cp_a_imm8(self, opcode),
            0xFF => Self::rst_tgt3(self, opcode),
        }
        .into();

        self.bus.run_cycles(cycles as u16);
        cycles
    }

    pub fn handle_interrupt(&mut self, int_source: IntSource) -> usize {
        self.ime = false;
        self.push_stack(self.pc);

        self.pc = match int_source {
            IntSource::VBLANK => 0x40,
            IntSource::LCD => 0x48,
            IntSource::TIMER => 0x50,
            IntSource::SERIAL => 0x58,
            IntSource::JOYPAD => 0x60,
        };

        self.bus.clear_interrupt(int_source);
        return 5;
    }
}
