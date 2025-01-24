use gb_rs::{
    cart::{get_cart_header, CartridgeData},
    gb::GbRs,
};
use std::fs::read;
use std::path::Path;
use std::time;

struct VecCart {
    rom: Vec<u8>,
    ram: Vec<u8>,
}

impl VecCart {
    pub fn from_slice(data: &[u8]) -> Self {
        let header = get_cart_header(data);

        let rom = Vec::from(data);
        let ram = vec![0; header.ram_size as usize];

        Self { rom, ram }
    }
}

impl CartridgeData for VecCart {
    type Rom = Vec<u8>;
    type Ram = Vec<u8>;

    fn rom(&self) -> &Self::Rom {
        &self.rom
    }

    fn rom_mut(&mut self) -> &mut Self::Rom {
        &mut self.rom
    }

    fn ram(&self) -> &Self::Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Self::Ram {
        &mut self.ram
    }
}

fn rom_test(rom_path: &str) {
    let rom_path = Path::new(rom_path);
    let rom = read(rom_path).expect(format!("Unable to load test rom: {:?}", rom_path).as_str());
    let cartridge = VecCart::from_slice(rom.as_slice());

    let mut gb = GbRs::new(cartridge);

    let timeout = time::Instant::now() + time::Duration::from_secs(30);

    let mut cnt = 0;

    while !gb.cpu.is_passed() {
        gb.run_one();

        if cnt == 1000 {
            // Timeout check
            assert!(time::Instant::now() < timeout);
            cnt = 0;
        }

        cnt += 1;
    }
}

#[test]
fn mbc1_bits_bank1() {
    rom_test("tests/roms/mooneye/mbc1/bits_bank1.gb");
}

#[test]
fn mbc1_bits_bank2() {
    rom_test("tests/roms/mooneye/mbc1/bits_bank2.gb");
}

#[test]
fn mbc1_bits_mode() {
    rom_test("tests/roms/mooneye/mbc1/bits_mode.gb");
}

#[test]
fn mbc1_bits_ramg() {
    rom_test("tests/roms/mooneye/mbc1/bits_ramg.gb");
}

#[test]
fn mbc1_512k() {
    rom_test("tests/roms/mooneye/mbc1/rom_512kb.gb");
}

#[test]
fn mbc1_1mb() {
    rom_test("tests/roms/mooneye/mbc1/rom_1Mb.gb");
}

#[test]
fn mbc1_2mb() {
    rom_test("tests/roms/mooneye/mbc1/rom_2Mb.gb");
}

#[test]
fn mbc1_4mb() {
    rom_test("tests/roms/mooneye/mbc1/rom_4Mb.gb");
}

#[test]
fn mbc1_8mb() {
    rom_test("tests/roms/mooneye/mbc1/rom_8Mb.gb");
}
