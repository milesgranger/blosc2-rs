use std::ffi::c_void;
use std::ffi::CString;

use blosc2_sys::ffi;

/// Result type used in this library
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Default buffer size for intermediate de/compression results when required
pub const BUFSIZE: usize = 8196_usize;

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum Filter {
    NoFilter = ffi::BLOSC_NOFILTER,
    Shuffle = ffi::BLOSC_SHUFFLE,
    BitShuffle = ffi::BLOSC_BITSHUFFLE,
    Delta = ffi::BLOSC_DELTA,
    TruncPrec = ffi::BLOSC_TRUNC_PREC,
    LastFilter = ffi::BLOSC_LAST_FILTER,
    LastRegisteredFilter = ffi::BLOSC_LAST_REGISTERED_FILTER,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::Shuffle
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum Codec {
    BloscLz = ffi::BLOSC_BLOSCLZ,
    LZ4 = ffi::BLOSC_LZ4,
    LZ4HC = ffi::BLOSC_LZ4HC,
    ZLIB = ffi::BLOSC_ZLIB,
    ZSTD = ffi::BLOSC_ZSTD,
    LastCodec = ffi::BLOSC_LAST_CODEC,
    LastRegisteredCodec = ffi::BLOSC_LAST_REGISTERED_CODEC,
}

impl Default for Codec {
    fn default() -> Self {
        Codec::BloscLz
    }
}

/// Possible CLevel settings
#[repr(u8)]
pub enum CLevel {
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
}

impl Default for CLevel {
    fn default() -> Self {
        CLevel::Nine // Ref blosc2 python default
    }
}

// pub mod schunk {
//     use super::*;

//     use super::*;

//     pub struct Storage(ffi::blosc2_storage);

//     pub struct SChunk(ffi::blosc2_schunk);

//     impl SChunk {
//         pub fn new() -> Self {
//             ffi::blosc2_schunk_new()
//         }
//     }
// }

pub mod read {
    ///! NOTE: These De/compressors are different from the blosc2 schunk. There are no frames, meta
    ///! layers, etc. It's _only_ meant for one or more independently compressed blocks. No more, no
    ///! less. If you're wanting `schunk` then hop over to the `crate::schunk` module(s).
    use super::*;

    pub struct Decompressor<R: std::io::Read> {
        rdr: R,
        src: [u8; BUFSIZE],
        src_pos: usize,
        src_end: usize,
        buf: [u8; BUFSIZE],
        buf_pos: usize,
        buf_end: usize,
    }

    impl<R: std::io::Read> Decompressor<R> {
        /// Blosc2 streaming decompressor. Will take care of any reader which
        /// contains one or more blocks of blosc2 encoded/compressed data.
        pub fn new(rdr: R) -> Decompressor<R> {
            Self {
                rdr,
                buf: [0u8; BUFSIZE], // Compressed data waiting to be copied to caller
                buf_pos: 0,
                buf_end: 0,
                src: [0u8; BUFSIZE], // Data from reader waiting to be compressed
                src_pos: 0,
                src_end: 0,
            }
        }
        pub fn into_inner(self) -> R {
            self.rdr
        }
        pub fn get_ref(&self) -> &R {
            &self.rdr
        }
        pub fn get_ref_mut(&mut self) -> &mut R {
            &mut self.rdr
        }
        fn read_from_buf(&mut self, buf: &mut [u8]) -> usize {
            let available_bytes = self.buf_end - self.buf_pos;
            let count = std::cmp::min(available_bytes, buf.len());
            buf[..count].copy_from_slice(&self.buf[self.buf_pos..self.buf_pos + count]);
            self.buf_pos += count;
            count
        }
        fn refill_src(&mut self) -> Result<usize> {
            // Previous decompress didn't decompress to end,
            // meaning there was a block end in the stream.
            // so we need to scoot the remainder to the front and
            // then continue to read onto the end of that chunk
            if self.src_pos > 0 {
                self.src.rotate_left(self.src_pos);
                self.src_end = self.src_end - self.src_pos;
                self.src_pos = 0;
            }
            let n_bytes = self.rdr.read(&mut self.src[self.src_end..])?;
            self.src_end += n_bytes;
            Ok(n_bytes)
        }
        fn decompress_src_into_buf(&mut self) -> Result<()> {
            // TODO: Can probably decompress directly into caller's buffer if it's big enough
            self.refill_src()?;
            if self.src_pos == self.src_end {
                return Ok(()); // Reader is empty and nothing left to decompress
            }
            let info = CompressedBufferInfo::try_from(&self.src[self.src_pos..self.src_end])?;
            self.buf_end = decompress_into(&self.src[self.src_pos..self.src_end], &mut self.buf)?;
            self.buf_pos = 0;
            self.src_pos = info.cbytes; // decompression ran up to cbytes (compressed bytes read from src buffer)
            Ok(())
        }
    }
    impl<R: std::io::Read> std::io::Read for Decompressor<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let count = self.read_from_buf(buf);
            if count > 0 {
                Ok(count)
            } else {
                self.decompress_src_into_buf()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                Ok(self.read_from_buf(buf))
            }
        }
    }

    /// Blosc2 streaming compressor. Will basically pack together blocks of
    /// compressed data at max ``BUFSIZE`` from source data.
    /// To decode it, one must use the `Decompressor` or make their own repeated calls
    /// to decompression functions. This is because a single call to a buffer with multiple
    /// compressed blocks will only decompress the first block.
    pub struct Compressor<R: std::io::Read> {
        rdr: R,
        src: [u8; BUFSIZE],
        src_pos: usize,
        src_end: usize,
        buf: [u8; BUFSIZE], // 64mb internal buffer
        buf_pos: usize,
        buf_end: usize,
    }

    impl<R: std::io::Read> Compressor<R> {
        pub fn new(rdr: R) -> Compressor<R> {
            Self {
                rdr,
                buf: [0u8; BUFSIZE], // Compressed data waiting to be copied to caller
                buf_pos: 0,
                buf_end: 0,
                src: [0u8; BUFSIZE], // Data from reader waiting to be compressed
                src_pos: 0,
                src_end: 0,
            }
        }
        pub fn into_inner(self) -> R {
            self.rdr
        }
        pub fn get_ref(&self) -> &R {
            &self.rdr
        }
        pub fn get_ref_mut(&mut self) -> &mut R {
            &mut self.rdr
        }
        fn read_from_buf(&mut self, buf: &mut [u8]) -> usize {
            let available_bytes = self.buf_end - self.buf_pos;
            let count = std::cmp::min(available_bytes, buf.len());
            buf[..count].copy_from_slice(&self.buf[self.buf_pos..self.buf_pos + count]);
            self.buf_pos += count;
            count
        }
        fn refill_src(&mut self) -> Result<usize> {
            self.src_end = self.rdr.read(&mut self.src)?;
            self.src_pos = 0;
            Ok(self.src_end)
        }
        fn compress_src_into_buf(&mut self) -> Result<()> {
            self.buf_end = compress_into(
                &mut self.src[self.src_pos..self.src_pos + self.src_end],
                &mut self.buf,
                CLevel::default() as _,
                Filter::default() as _,
                Codec::default() as _,
            )?;
            self.buf_pos = 0;
            Ok(())
        }
    }

    impl<R: std::io::Read> std::io::Read for Compressor<R> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let count = self.read_from_buf(buf);
            if count > 0 {
                Ok(count)
            } else {
                let n_read = self
                    .refill_src()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                if n_read == 0 {
                    return Ok(0);
                }
                self.compress_src_into_buf()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                Ok(self.read_from_buf(buf))
            }
        }
    }
}

pub struct CParams(ffi::blosc2_cparams);

impl CParams {
    pub fn into_inner(self) -> ffi::blosc2_cparams {
        self.0
    }
    pub fn inner_ref_mut(&mut self) -> &mut ffi::blosc2_cparams {
        &mut self.0
    }
    pub fn set_codec(&mut self, codec: Codec) {
        self.0.compcode = codec as _;
    }
    pub fn set_clevel(&mut self, clevel: i32) {
        self.0.clevel = clevel as _;
    }
    pub fn set_filter(&mut self, filter: Filter) {
        self.0.filters[ffi::BLOSC2_MAX_FILTERS as usize - 1] = filter as _;
    }
}

impl Default for CParams {
    #[inline]
    fn default() -> Self {
        let mut cparams = ffi::blosc2_cparams::default();
        cparams.compcode = Codec::default() as _;
        cparams.clevel = CLevel::default() as _;
        cparams.typesize = 8;
        cparams.splitmode = ffi::BLOSC_FORWARD_COMPAT_SPLIT as _;
        cparams.filters[ffi::BLOSC2_MAX_FILTERS as usize - 1] = Filter::default() as _;
        cparams.nthreads = 1;
        Self(cparams)
    }
}

/// Create CParams from a reference to the type being compressed
impl<T> From<&T> for CParams {
    fn from(val: &T) -> Self {
        let mut cparams = CParams::default();
        cparams.0.typesize = (std::mem::size_of_val(val) * 8) as _;
        cparams
    }
}

pub struct DParams(pub(crate) ffi::blosc2_dparams);

impl DParams {
    pub fn set_n_threads(&mut self, n: usize) {
        self.0.nthreads = n as _;
    }
}

impl Default for DParams {
    #[inline]
    fn default() -> Self {
        let mut dparams = ffi::blosc2_dparams::default();
        dparams.nthreads = 1;
        Self(dparams)
    }
}

/// Container struct for de/compression ops requiring context when used in multithreaded environments
#[derive(Clone)]
pub struct Context(pub(crate) *mut ffi::blosc2_context);

impl From<DParams> for Context {
    fn from(dparams: DParams) -> Self {
        Self(unsafe { ffi::blosc2_create_dctx(dparams.0) })
    }
}
impl From<CParams> for Context {
    fn from(cparams: CParams) -> Self {
        Self(unsafe { ffi::blosc2_create_cctx(cparams.0) })
    }
}

impl Default for Context {
    fn default() -> Self {
        let ctx = unsafe { ffi::blosc2_create_cctx(CParams::default().into_inner()) };
        Self(ctx)
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ffi::blosc2_free_ctx(self.0) };
        }
    }
}

/// Info about a compressed buffer
/// Normal construction via `CompressedBufferInfo::try_from(&[u8])?`
pub struct CompressedBufferInfo {
    /// Number of bytes decompressed
    pub nbytes: usize,
    /// Number of bytes to be read from compressed buffer
    pub cbytes: usize,
    /// Used internally by blosc2 when compressing the blocks, exposed here for completion.
    /// You probably won't need it.
    pub blocksize: usize,
}

impl<T> TryFrom<&[T]> for CompressedBufferInfo {
    type Error = Box<dyn std::error::Error>;

    #[inline]
    fn try_from(buf: &[T]) -> Result<Self> {
        let mut nbytes = 0i32;
        let mut cbytes = 0i32;
        let mut blocksize = 0i32;
        let code = unsafe {
            ffi::blosc2_cbuffer_sizes(
                buf.as_ptr() as *const c_void,
                &mut nbytes as *mut _,
                &mut cbytes as *mut _,
                &mut blocksize as *mut _,
            )
        };
        if code < 0 {
            return Err(format!("Failed to decompress, exit code: '{}'", code).into());
        }
        Ok(CompressedBufferInfo {
            nbytes: nbytes as _,
            cbytes: cbytes as _,
            blocksize: blocksize as _,
        })
    }
}
/// Context interface to compression, does not require call to init/destroy. For
/// use in multithreaded applications
#[inline]
pub fn compress_ctx<T: Clone>(src: &[T], ctx: &mut Context) -> Result<Vec<T>> {
    if src.is_empty() {
        return Ok(vec![]);
    }
    let mut dst = vec![src[0].clone(); src.len() + ffi::BLOSC2_MAX_OVERHEAD as usize];
    let size = compress_into_ctx(src, &mut dst, ctx)?;
    if dst.len() > size {
        dst.truncate(size as _);
    }
    Ok(dst)
}

#[inline]
pub fn compress_into_ctx<T: Clone>(src: &[T], dst: &mut [T], ctx: &mut Context) -> Result<usize> {
    if src.is_empty() {
        return Ok(0);
    }
    let size = unsafe {
        ffi::blosc2_compress_ctx(
            ctx.0,
            src.as_ptr() as *const c_void,
            src.len() as _,
            dst.as_mut_ptr() as *mut c_void,
            dst.len() as _,
        )
    };

    if size == 0 {
        return Err(format!("Buffer is incompressible").into());
    } else if size < 0 {
        return Err(format!("Failed to compress, exit code '{}' from blosc2", size).into());
    }
    Ok(size as _)
}

#[inline]
pub fn compress<T: Clone>(
    src: &[T],
    clevel: Option<CLevel>,
    filter: Option<Filter>,
    codec: Option<Codec>,
) -> Result<Vec<T>> {
    if src.is_empty() {
        return Ok(vec![]);
    }
    let cdc = codec.unwrap_or_default();
    let clvl = clevel.unwrap_or_default();
    let fltr = filter.unwrap_or_default();

    let mut dst = vec![src[0].clone(); src.len() + ffi::BLOSC2_MAX_OVERHEAD as usize];
    let n_bytes = compress_into(src, &mut dst, clvl, fltr, cdc)?;

    dst.truncate(n_bytes);
    Ok(dst)
}

#[inline]
pub fn compress_into<T>(
    src: &[T],
    dst: &mut [T],
    clevel: CLevel,
    filter: Filter,
    codec: Codec,
) -> Result<usize> {
    if src.is_empty() {
        return Ok(0);
    }
    let typesize = std::mem::size_of_val(&src[0]) * 8;
    set_compressor(codec)?;
    let n_bytes = unsafe {
        ffi::blosc2_compress(
            clevel as _,
            filter as _,
            typesize as _,
            src.as_ptr() as *const c_void,
            src.len() as _,
            dst.as_mut_ptr() as *mut c_void,
            dst.len() as _,
        )
    };
    if n_bytes < 0 {
        return Err(format!("Failed to compress, exit code '{}' from blosc2", n_bytes).into());
    } else if n_bytes == 0 {
        return Err("Data is not compressable.".into());
    }

    Ok(n_bytes as _)
}

#[inline]
pub fn decompress_ctx<T: Clone>(src: &[T], ctx: &mut Context) -> Result<Vec<T>> {
    if src.is_empty() {
        return Ok(vec![]);
    }
    let info = CompressedBufferInfo::try_from(src)?;
    let mut dst = vec![src[0].clone(); info.nbytes];
    let n_bytes = decompress_into_ctx(src, &mut dst, ctx)?;
    if dst.len() > n_bytes as _ {
        dst.truncate(n_bytes as _);
    }
    Ok(dst)
}

#[inline]
pub fn decompress_into_ctx<T: Clone>(src: &[T], dst: &mut [T], ctx: &mut Context) -> Result<usize> {
    if src.is_empty() {
        return Ok(0);
    }
    let n_bytes = unsafe {
        ffi::blosc2_decompress_ctx(
            ctx.0,
            src.as_ptr() as _,
            src.len() as _,
            dst.as_mut_ptr() as _,
            dst.len() as _,
        )
    };
    if n_bytes < 0 {
        return Err(format!("Failed to compress buffer, return code: '{}'", n_bytes).into());
    }
    Ok(n_bytes as _)
}

#[inline]
pub fn decompress<T: Clone>(src: &[T]) -> Result<Vec<T>> {
    if src.is_empty() {
        return Ok(vec![]);
    }
    let info = CompressedBufferInfo::try_from(src)?;
    let mut dst = vec![src[0].clone(); info.nbytes];
    let n_bytes = decompress_into(src, &mut dst)?;
    if dst.len() > n_bytes {
        dst.truncate(n_bytes);
    }
    Ok(dst)
}

#[inline]
pub fn decompress_into<T>(src: &[T], dst: &mut [T]) -> Result<usize> {
    let n_bytes = unsafe {
        ffi::blosc2_decompress(
            src.as_ptr() as *const c_void,
            src.len() as _,
            dst.as_mut_ptr() as *mut c_void,
            dst.len() as _,
        )
    };
    if n_bytes < 0 {
        return Err(format!("Failed to decompress, exit code: '{}'", n_bytes).into());
    }
    Ok(n_bytes as _)
}

#[inline]
pub fn set_compressor(codec: Codec) -> Result<()> {
    let codec = match codec {
        Codec::BloscLz => CString::new("blosclz").unwrap(),
        _ => unimplemented!("Only blosclz codec supported for now"),
    };
    if unsafe { ffi::blosc1_set_compressor(codec.as_ptr()) } < 0 {
        Err(format!("The codec '{:?}' is not available", codec).into())
    } else {
        Ok(())
    }
}

/// Call before using blosc2, unless using specific ctx de/compression variants
pub fn blosc2_init() {
    unsafe { ffi::blosc2_init() }
}

/// Call at end of using blosc2 library, unless you've never called `blosc2_init`
pub fn blosc2_destroy() {
    unsafe { ffi::blosc2_destroy() }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use super::*;

    #[test]
    fn test_compress_ctx() -> Result<()> {
        let input = b"some data";
        let compressed = compress_ctx(input, &mut Context::from(CParams::from(&input[0])))?;
        let decompressed = decompress(&compressed)?;
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_compress_into_ctx() -> Result<()> {
        let input = b"some data";
        let mut compressed = vec![0u8; 100];
        let n_bytes = compress_into_ctx(
            input,
            &mut compressed,
            &mut Context::from(CParams::from(&input[0])),
        )?;
        let decompressed = decompress(&compressed[..n_bytes])?;
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_decompress_ctx() -> Result<()> {
        let input = b"some data";
        let compressed = compress(input, None, None, None)?;
        let decompressed = decompress_ctx(&compressed, &mut Context::from(DParams::default()))?;
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_decompress_into_ctx() -> Result<()> {
        let input = b"some data";
        let compressed = compress(input, None, None, None)?;
        let mut decompressed = vec![0u8; input.len()];
        let n_bytes = decompress_into_ctx(
            &compressed,
            &mut decompressed,
            &mut Context::from(DParams::default()),
        )?;
        assert_eq!(n_bytes, input.len());
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_basic_roundtrip() -> Result<()> {
        let input = b"some data";
        let compressed = compress(input, None, None, None)?;
        let decompressed = decompress(&compressed)?;
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_basic_roundtrip_into() -> Result<()> {
        let input = b"some data";
        let mut compressed = vec![0u8; 100];
        let n_bytes = compress_into(
            input,
            &mut compressed,
            CLevel::default() as _,
            Filter::default() as _,
            Codec::default() as _,
        )?;

        let mut decompressed = vec![0u8; input.len()];
        let n_out = decompress_into(&compressed[..n_bytes], &mut decompressed)?;

        assert_eq!(n_out, input.len());
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_basic_compressor() -> Result<()> {
        let input = b"some data";
        let cursor = Cursor::new(input);

        let mut compressed: Vec<u8> = vec![];
        let mut compressor = read::Compressor::new(cursor);
        compressor.read_to_end(&mut compressed)?;

        let mut decompressed = vec![0u8; input.len()];
        decompress_into(&mut compressed, &mut decompressed)?;

        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_basic_decompressor() -> Result<()> {
        let mut stream = vec![];
        stream.extend_from_slice(compress(b"foo", None, None, None)?.as_slice());
        stream.extend_from_slice(compress(b"bar", None, None, None)?.as_slice());

        let mut decompressor = read::Decompressor::new(Cursor::new(stream));
        let mut decompressed = vec![];
        decompressor.read_to_end(&mut decompressed)?;

        assert_eq!(b"foobar", decompressed.as_slice());

        Ok(())
    }

    #[test]
    fn test_basic_large_stream() -> Result<()> {
        // stream that's well beyond BUFSIZE
        let stream = std::iter::repeat(b"foobar")
            .take(BUFSIZE * 2)
            .flat_map(|s| s.to_vec())
            .collect::<Vec<u8>>();

        let mut compressed = vec![];
        let mut compressor = read::Compressor::new(Cursor::new(stream.clone()));
        let n_compressed = std::io::copy(&mut compressor, &mut compressed)?;
        assert_eq!(n_compressed, 2496);

        let mut decompressed = vec![];
        let mut decompressor = read::Decompressor::new(Cursor::new(compressed));
        let n_decompressed = std::io::copy(&mut decompressor, &mut decompressed)?;
        assert_eq!(n_decompressed as usize, stream.len());

        assert_eq!(&decompressed, &stream);
        Ok(())
    }
}
