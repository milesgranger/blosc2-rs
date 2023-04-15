use blosc2::{compress, destroy, init};

fn main() {
    init();
    let data = std::iter::repeat(b"some data here")
        .take(100_000)
        .flat_map(|v| v.to_vec())
        .collect::<Vec<u8>>();

    let mut compressed = vec![0u8; data.len() + data.len()];
    for _ in 0..10_000 {
        let _ = blosc2::compress_into(
            &data,
            &mut compressed,
            blosc2::CLevel::Nine,
            blosc2::Filter::Shuffle,
            blosc2::Codec::BloscLz,
        )
        .unwrap();
    }
    destroy();
}
