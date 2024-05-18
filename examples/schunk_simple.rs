const KB: usize = 1024;
const MB: usize = 1024 * KB;

const CHUNKSIZE: usize = 1_000 * 1_000;
const NCHUNKS: usize = 100;
const NTHREADS: usize = 4;

fn main() {
    blosc2::init();

    let mut data = vec![0_i32; CHUNKSIZE];
    let mut data_dest = vec![0_i32; CHUNKSIZE];

    println!(
        "Blosc2 version info: {} ({})",
        blosc2::get_version_string().unwrap(),
        String::from_utf8(blosc2::BLOSC2_VERSION_DATE.to_vec()).unwrap()
    );

    // Create a super-chunk container
    let cparams = blosc2::CParams::default()
        .set_typesize::<i32>()
        .set_clevel(blosc2::CLevel::Nine)
        .set_nthreads(NTHREADS);
    let dparams = blosc2::DParams::default().set_nthreads(NTHREADS);
    let storage = blosc2::schunk::Storage::default()
        .set_cparams(cparams)
        .set_dparams(dparams);
    let mut schunk = blosc2::schunk::SChunk::new(storage);

    let ttotal = std::time::Instant::now();
    for nchunk in 0..NCHUNKS {
        for i in 0..CHUNKSIZE {
            data[i] = (i * nchunk) as i32;
        }
        let nchunks = schunk.append_buffer(&data).unwrap();
        if nchunks != nchunk + 1 {
            panic!(
                "Unexpected nchunks! Got {}, but expected {}",
                nchunks,
                nchunk + 1
            );
        }
    }

    println!(
        "Compression ratio: {:.1} MB -> {:.1} MB ({:.1}x)",
        schunk.nbytes() as f32 / MB as f32,
        schunk.cbytes() as f32 / MB as f32,
        schunk.compression_ratio()
    );
    println!(
        "Compression time: {:.3}s, {:.1}MB/s",
        ttotal.elapsed().as_secs_f32(),
        schunk.nbytes() as f32 / (ttotal.elapsed().as_secs_f32() * MB as f32)
    );

    // Retrieve and decompress the chunks (0-based count)
    let ttotal = std::time::Instant::now();
    for nchunk in (0..(NCHUNKS - 1)).rev() {
        // The error check in c-blosc2 example is removed b/c will have been raised thru .unwrap call
        let dsize = schunk.decompress_chunk(nchunk, &mut data_dest).unwrap();
        assert_eq!(dsize, CHUNKSIZE);
    }
    println!(
        "Decompression time: {:.3}s, {:.1}MB/s",
        ttotal.elapsed().as_secs_f32(),
        schunk.nbytes() as f32 / (ttotal.elapsed().as_secs_f32() * MB as f32)
    );

    // Check integrity of the second chunk (made of non-zeros)
    schunk.decompress_chunk(1, &mut data_dest).unwrap();
    for i in 0..CHUNKSIZE {
        if data_dest[i] != i as i32 {
            panic!(
                "Decompressed data differs from original {}, {}",
                i as i32, data_dest[i]
            );
        }
    }

    blosc2::destroy();
}
