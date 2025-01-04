use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gb_rs::gb::GbRs;
use gb_rs::rom::Rom;

use std::fs::read;
use std::path::Path;

pub fn run_1000_frames(gb: &mut GbRs) {
    for _ in 0..1000 {
        gb.run_frame();
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let rom = read(Path::new("testroms/dmg-acid2.gb")).expect("Unable to load tetris rom");
    let rom = Rom::from_slice(rom.as_slice());
    let mut gb = GbRs::new(rom);

    c.bench_function("tetris_1000frames", |b| b.iter(|| run_1000_frames(&mut gb)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
