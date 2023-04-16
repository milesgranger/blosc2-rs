use blosc2::{compress, decompress, destroy, init, CParams, DParams};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::{ffi::OsString, fs};

fn criterion_benchmark(c: &mut Criterion) {
    init();

    let data: Vec<(OsString, Vec<u8>)> =
        fs::read_dir(format!("{}/data", env!("CARGO_MANIFEST_DIR")))
            .unwrap()
            .filter(|f| {
                f.as_ref()
                    .ok()
                    .map(|f| f.file_type().unwrap().is_file())
                    .is_some()
            })
            .map(|f| {
                let name = f.as_ref().unwrap().file_name().to_owned();
                let data = fs::read(f.unwrap().path()).unwrap();
                (name, data)
            })
            .collect();

    let mut compression_group = c.benchmark_group("compress");

    // Compress all data, then file-by-file
    let all_data = data
        .iter()
        .map(|(_, d)| d.clone())
        .flatten()
        .collect::<Vec<u8>>();
    compression_group.throughput(Throughput::Bytes(all_data.len() as _));
    compression_group.bench_function("defaults - all-data", |b| {
        b.iter(|| compress(black_box(&all_data), None, None, None))
    });
    data.iter().for_each(|(name, data)| {
        compression_group.throughput(Throughput::Bytes(data.len() as _));
        compression_group.bench_function(&format!("defaults - {}", name.to_str().unwrap()), |b| {
            b.iter(|| compress(black_box(&data), None, None, None))
        });
    });

    // TODO: Decompress all data, then file-by-file

    // TODO: Roundtrip all data, then file-by-file

    destroy();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
