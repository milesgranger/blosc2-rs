use blosc2::{compress, decompress, Blosc2Guard};

fn main() {
    let _blosc2_guard = Blosc2Guard::get_or_init();

    let data = std::iter::repeat(b"some data here")
        .take(100_000)
        .flat_map(|v| v.to_vec())
        .collect::<Vec<u8>>();

    let compressed = compress(&data, None, None, None, None).unwrap();
    let decompressed = decompress(&compressed).unwrap();

    assert_eq!(data, decompressed);
}
