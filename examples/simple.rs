use blosc2::{compress, decompress, destroy, init};

fn main() {
    init();
    let data = std::iter::repeat(b"some data here")
        .take(100_000)
        .flat_map(|v| v.to_vec())
        .collect::<Vec<u8>>();

    let compressed = compress(&data, None, None, None).unwrap();
    let decompressed = decompress(&compressed).unwrap();

    assert_eq!(data, decompressed);

    destroy();
}
