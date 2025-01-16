use criterion::{criterion_group, criterion_main, Criterion};
use gb_rs::gb::GbRs;
use gb_rs::gb::SmallInMemoryCartridge;

use std::fs::read;
use std::path::Path;

pub fn criterion_benchmark(c: &mut Criterion) {
    let rom = read(Path::new("tests/roms/dmg-acid2.gb")).expect("Unable to load test rom");
    let cartridge = SmallInMemoryCartridge::from_slice(rom.as_slice());
    let mut gb = GbRs::new(cartridge);

    c.bench_function("dmg-acid2_frames", |b| {
        b.iter(|| {
            gb.run_frame();
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
