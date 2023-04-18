use blosc2::{compress, decompress, destroy, init};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::fs;

fn criterion_benchmark(c: &mut Criterion) {
    init();

    let data = build_data();

    // Compress
    {
        let mut compression_group = c.benchmark_group("compress");
        data.iter().for_each(|(name, data)| {
            compression_group.throughput(Throughput::Bytes(data.len() as _));
            compression_group.bench_function(name, |b| {
                b.iter(|| compress(black_box(&data), None, None, None).unwrap())
            });
        });
    }

    // Decompress
    {
        let mut decompression_group = c.benchmark_group("decompress");
        data.iter().for_each(|(name, data)| {
            let compressed = compress(&data, None, None, None).unwrap();

            decompression_group.throughput(Throughput::Bytes(compressed.len() as _));
            decompression_group.bench_function(name, |b| {
                b.iter(|| decompress(black_box(&compressed)).unwrap())
            });
        });
    }

    // Roundtrip
    {
        let mut roundtrip_group = c.benchmark_group("roundtrip");
        data.iter().for_each(|(name, data)| {
            roundtrip_group.throughput(Throughput::Bytes(data.len() as _));
            roundtrip_group.bench_function(name, |b| {
                b.iter(|| {
                    decompress(&compress(black_box(&data), None, None, None).unwrap()).unwrap()
                })
            });
        });
    }

    destroy();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

fn build_data() -> Vec<(String, Vec<u8>)> {
    let mut data: Vec<(String, Vec<u8>)> =
        fs::read_dir(format!("{}/data", env!("CARGO_MANIFEST_DIR")))
            .unwrap()
            .filter(|f| {
                f.as_ref()
                    .ok()
                    .map(|f| f.file_type().unwrap().is_file())
                    .is_some()
            })
            .map(|f| {
                let name = f
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_owned()
                    .into_string()
                    .unwrap();
                let data = fs::read(f.unwrap().path()).unwrap();
                (name, data)
            })
            .collect();
    data.push((
        "repeating".to_owned(),
        std::iter::repeat(b"1234567890")
            .take(1_000_000)
            .flat_map(|v| v.to_vec())
            .collect(),
    ));
    // Same length as the repeating data
    data.push((
        "random".to_owned(),
        (0..10_000_000).map(|_| rand::random::<u8>()).collect(),
    ));
    data.push((
        "all-data".to_owned(),
        data.iter().flat_map(|(_, d)| d.to_owned()).collect(),
    ));
    data
}
