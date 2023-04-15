use blosc2::{compress, decompress, destroy, init, CParams, DParams};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;

fn criterion_benchmark(c: &mut Criterion) {
    init();
    fs::read_dir(format!("{}/data", env!("CARGO_MANIFEST_DIR")))
        .unwrap()
        .filter(|f| {
            f.as_ref()
                .ok()
                .map(|f| f.file_type().unwrap().is_file())
                .is_some()
        })
        .for_each(|f| {
            let name = f.as_ref().unwrap().file_name().to_owned();
            let data = fs::read(f.unwrap().path()).unwrap();
            c.bench_function(
                &format!("compress - defaults - {}", name.to_str().unwrap()),
                |b| b.iter(|| compress(black_box(&data), None, None, None)),
            );
        });
    destroy();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
