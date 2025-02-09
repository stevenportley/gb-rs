use criterion::{criterion_group, criterion_main, Criterion};
use gb_rs::gb::GbRs;
use gb_rs::gb::SmallInMemoryCartridge;

use std::fs::read;
use std::path::Path;

pub fn acid2_benchmark(c: &mut Criterion) {
    let rom = read(Path::new("tests/roms/dmg-acid2.gb")).expect("Unable to load test rom");
    let cartridge = SmallInMemoryCartridge::from_slice(rom.as_slice());
    let mut gb = GbRs::new(cartridge);

    c.bench_function("dmg-acid2_frames", |b| {
        b.iter(|| {
            gb.run_frame();
        })
    });
}

pub fn ppu_stress_benchmark(c: &mut Criterion) {
    let rom = read(Path::new("tests/benchmarks/vectdemo.gb")).expect("Unable to load test rom");
    let cartridge = SmallInMemoryCartridge::from_slice(rom.as_slice());
    let mut gb = GbRs::new(cartridge);

    c.bench_function("vectdemo_frames", |b| {
        b.iter(|| {
            for _ in 0..100 {
                gb.run_frame();
            }
        })
    });
}

criterion_group!{
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(1000);
    targets = acid2_benchmark, ppu_stress_benchmark}
criterion_main!(benches);
