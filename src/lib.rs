//! Blosc2 Rust bindings.

use std::ffi::c_void;
use std::ffi::CString;

use blosc2_sys as ffi;

/// Result type used in this library
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Default buffer size for intermediate de/compression results when required
pub const BUFSIZE: usize = 8196_usize;

/// Possible Filters
#[derive(Debug, Copy, Clone)]
pub enum Filter {
    NoFilter = ffi::BLOSC_NOFILTER as _,
    Shuffle = ffi::BLOSC_SHUFFLE as _,
    BitShuffle = ffi::BLOSC_BITSHUFFLE as _,
    Delta = ffi::BLOSC_DELTA as _,
    TruncPrec = ffi::BLOSC_TRUNC_PREC as _,
    LastFilter = ffi::BLOSC_LAST_FILTER as _,
    LastRegisteredFilter = ffi::BLOSC_LAST_REGISTERED_FILTER as _,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::Shuffle
    }
}

/// Possible compression codecs
#[derive(Debug, Copy, Clone)]
pub enum Codec {
    BloscLz = ffi::BLOSC_BLOSCLZ as _,
    LZ4 = ffi::BLOSC_LZ4 as _,
    LZ4HC = ffi::BLOSC_LZ4HC as _,
    ZLIB = ffi::BLOSC_ZLIB as _,
    ZSTD = ffi::BLOSC_ZSTD as _,
    LastCodec = ffi::BLOSC_LAST_CODEC as _,
    LastRegisteredCodec = ffi::BLOSC_LAST_REGISTERED_CODEC as _,
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

pub mod schunk {
    //! `blosc2_schunk` and `blosc2_storage` API

    use std::ffi::CStr;
    use std::path::PathBuf;

    use super::*;

    /// Wrapper to [blosc2_storage]
    ///
    /// [blosc2_storage]: blosc2_sys::blosc2_storage
    #[derive(Default)]
    pub struct Storage(ffi::blosc2_storage);

    impl Storage {
        /// Set url/file path to specify a file-backed `schunk`.
        pub fn set_urlpath<S: ToString>(mut self, urlpath: S) -> Result<Self> {
            self.0.urlpath = CString::new(urlpath.to_string())?.into_raw();
            Ok(self)
        }
        /// Set the contiguous nature of the `schunk`.
        pub fn set_contiguous(mut self, contiguous: bool) -> Self {
            self.0.contiguous = contiguous;
            self
        }
        /// Set compression parameters
        pub fn set_cparams(mut self, cparams: &mut CParams) -> Self {
            self.0.cparams = &mut cparams.0;
            self
        }
        /// Set decompression parameters
        pub fn set_dparams(mut self, dparams: &mut DParams) -> Self {
            self.0.dparams = &mut dparams.0;
            self
        }
    }

    /// Wrapper to [blosc2_schunk]
    ///
    /// [blosc2_schunk]: blosc2_sys::blosc2_schunk
    pub struct SChunk(pub(crate) *mut ffi::blosc2_schunk);

    // Loosely inspired by blosc2-python implementation
    impl SChunk {
        pub fn new(storage: Storage) -> Self {
            let mut storage = storage;
            let schunk = unsafe { ffi::blosc2_schunk_new(&mut storage.0) };
            Self(schunk)
        }

        #[inline]
        pub(crate) fn inner(&self) -> &ffi::blosc2_schunk {
            unsafe { &(*self.0) }
        }

        #[inline]
        #[allow(dead_code)]
        pub(crate) fn inner_mut(&mut self) -> &mut ffi::blosc2_schunk {
            unsafe { &mut (*self.0) }
        }

        /// Append data to SChunk, returning new number of chunks
        pub fn append_buffer<T>(&mut self, data: &[T]) -> Result<usize> {
            if data.is_empty() {
                return Ok(self.inner().nchunks as usize);
            }

            let size = std::mem::size_of_val(unsafe { &data.get_unchecked(0) });
            let typesize = self.inner().typesize as _;
            if size != typesize {
                let msg = format!("Size of T ({}) != schunk typesize ({})", size, typesize);
                return Err(msg.into());
            }

            let n = unsafe {
                ffi::blosc2_schunk_append_buffer(self.0, data.as_ptr() as _, data.len() as _)
            };
            if n < 0 {
                return Err(Blosc2Error::from(n as i32).into());
            }
            Ok(n as _)
        }

        /// Decompress a chunk, returning number of bytes written to output buffer
        pub fn decompress_chunk<T>(&mut self, nchunk: usize, dst: &mut [T]) -> Result<usize> {
            let mut chunk = std::ptr::null_mut();
            let mut needs_free: bool = false;
            let mut rc = unsafe {
                ffi::blosc2_schunk_get_chunk(
                    self.0,
                    nchunk as _,
                    &mut chunk as *mut *mut u8,
                    &mut needs_free as *mut bool,
                )
            };
            if rc < 0 {
                return Err(format!("Failed to get chunk {}, exit code: '{}'", nchunk, rc).into());
            }
            let mut nbytes = 0;
            let mut cbytes = 0;
            let mut blocksize = 0;
            rc = unsafe {
                ffi::blosc2_cbuffer_sizes(chunk as _, &mut nbytes, &mut cbytes, &mut blocksize)
            };
            if needs_free {
                unsafe { ffi::free(chunk as _) };
            }
            if rc < 0 {
                return Err(Blosc2Error::from(rc).into());
            }
            if dst.len() < nbytes as usize {
                return Err(format!(
                    "Output buffer not large enough, need {} but length is {}",
                    nbytes,
                    dst.len()
                )
                .into());
            }
            let size = unsafe {
                ffi::blosc2_schunk_decompress_chunk(
                    self.0,
                    nchunk as _,
                    dst.as_mut_ptr() as _,
                    nbytes,
                )
            };
            if size < 0 {
                return Err(Blosc2Error::from(size).into());
            } else if size == 0 {
                return Err(format!(
                    "Non-initialized error when decompressing chunk '{}'",
                    nchunk
                )
                .into());
            } else {
                Ok(size as _)
            }
        }

        /// Export this `SChunk` into a buffer
        pub fn into_vec(self) -> Result<Vec<u8>> {
            let mut needs_free = true;
            let mut ptr: *mut u8 = std::ptr::null_mut();
            let len = unsafe { ffi::blosc2_schunk_to_buffer(self.0, &mut ptr, &mut needs_free) };
            if len < 0 {
                return Err(Blosc2Error::from(len as i32).into());
            }

            let mut buf = unsafe { Vec::from_raw_parts(ptr, len as _, len as _) };
            if needs_free {
                buf = buf.clone(); // Clone into new since blosc is about to free this one
                unsafe { ffi::free(ptr as _) };
            }
            Ok(buf)
        }

        /// Create a Schunk from an owned `Vec<u8>`. Data will be owned by the Schunk and released
        /// via normal Rust semantics.
        pub fn from_vec(buf: Vec<u8>) -> Result<Self> {
            let mut buf = buf;
            let schunk =
                unsafe { ffi::blosc2_schunk_from_buffer(buf.as_mut_ptr(), buf.len() as _, false) };

            // copy is false, schunk/blosc2 takes ownership, and set that it'll be responsible to free it
            std::mem::forget(buf);
            unsafe { ffi::blosc2_schunk_avoid_cframe_free(schunk, false) };
            Ok(Self(schunk))
        }

        /// Create a Schunk from a slice. The data _will not be copied_ but the slice used as the
        /// underlying schunk cframe.
        pub fn from_slice(buf: &mut [u8]) -> Result<Self> {
            // schunk is a view of the slice, so copy is false and set to avoid freeing the buffer
            let schunk =
                unsafe { ffi::blosc2_schunk_from_buffer(buf.as_mut_ptr(), buf.len() as _, false) };
            if schunk.is_null() {
                return Err("Failed to get schunk from buffer".into());
            }
            unsafe { ffi::blosc2_schunk_avoid_cframe_free(schunk, true) };
            Ok(Self(schunk))
        }

        // --- PROPERTIES ---

        /// Check if storage of Schunk is contiguous.
        pub fn is_contiguous(&self) -> bool {
            unsafe { (*(self.inner()).storage).contiguous }
        }

        /// Check typesize
        pub fn typesize(&self) -> usize {
            self.inner().typesize as _
        }

        /// Compression ratio of the Schunk
        pub fn compression_ratio(&self) -> f32 {
            if self.inner().cbytes == 0 {
                return 0f32;
            }
            self.inner().nbytes as f32 / self.inner().cbytes as f32
        }

        /// Number of chunks
        pub fn n_chunks(&self) -> usize {
            self.inner().nchunks as _
        }

        /// Chunk shape
        pub fn chunk_shape(&self) -> usize {
            (self.inner().chunksize / self.inner().typesize) as _
        }

        /// blocksize
        pub fn blocksize(&self) -> usize {
            self.inner().blocksize as _
        }

        /// Count of uncompressed bytes
        pub fn nbytes(&self) -> usize {
            self.inner().nbytes as _
        }

        /// Count of compressed bytes
        pub fn cbytes(&self) -> usize {
            self.inner().cbytes as _
        }

        /// Path where the Schunk is stored, if file backed.
        pub fn path(&self) -> Option<std::path::PathBuf> {
            let urlpath_ptr = unsafe { (*(self.inner().storage)).urlpath };
            if urlpath_ptr.is_null() {
                return None;
            }
            let urlpath = unsafe { CStr::from_ptr(urlpath_ptr) };
            urlpath.to_str().map(PathBuf::from).ok()
        }

        /// Returns under of elements in Schunk (nbytes / typesize)
        pub fn len(&self) -> usize {
            (self.inner().nbytes / self.inner().typesize as i64) as usize
        }
    }
}

pub mod read {
    //! NOTE: These De/compressors are different from the blosc2 schunk. There are no frames, meta
    //! layers, etc. It's _only_ meant for one or more independently compressed blocks. No more, no
    //! less. If you're wanting `schunk` then hop over to the [schunk] module.
    //!
    //! [schunk]: crate::schunk
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

/// Wrapper to [blosc2_cparams].  
/// Compression parameters.
///
/// A normal way to construct this is using `std::convert::From<&T>(val)`
/// so it will create with default parameters and the correct `typesize`.
///
/// Example
/// -------
/// ```
/// use blosc2::CParams;
/// let values = vec![0, 1, 2, 3];
/// let cparams = CParams::from(&values[0])
///     .set_threads(2);  // Optionally adjust default values
/// ```
/// [blosc2_cparams]: blosc2_sys::blosc2_cparams
pub struct CParams(ffi::blosc2_cparams);

impl CParams {
    pub(crate) fn into_inner(self) -> ffi::blosc2_cparams {
        self.0
    }
    #[allow(dead_code)]
    pub(crate) fn inner_ref_mut(&mut self) -> &mut ffi::blosc2_cparams {
        &mut self.0
    }
    /// Set codec, defaults to [Codec::BloscLz]
    ///
    /// [Codec::BloscLz]: crate::Codec::BloscLz
    pub fn set_codec(mut self, codec: Codec) -> Self {
        self.0.compcode = codec as _;
        self
    }
    /// Set clevel, defaults to [CLevel::Nine]
    ///
    /// [CLevel::Nine]: crate::CLevel::Nine
    pub fn set_clevel(mut self, clevel: CLevel) -> Self {
        self.0.clevel = clevel as _;
        self
    }
    /// Set filter, defaults to [Filter::Shuffle]
    ///
    /// [Filter::Shuffle]: crate::Filter::Shuffle
    pub fn set_filter(mut self, filter: Filter) -> Self {
        self.0.filters[ffi::BLOSC2_MAX_FILTERS as usize - 1] = filter as _;
        self
    }
    /// Set number of threads, defaults to 1
    pub fn set_threads(mut self, n: usize) -> Self {
        self.0.nthreads = n as _;
        self
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

/// Wrapper to [blosc2_dparams].  
/// Decompression parameters, normally constructed via `DParams::default()`.
///
/// Example
/// -------
/// ```
/// use blosc2::DParams;
/// let dparams = DParams::default()
///     .set_threads(2);  // Optionally adjust default values
/// ```
///
/// [blosc2_dparams]: blosc2_sys::blosc2_dparams
pub struct DParams(pub(crate) ffi::blosc2_dparams);

impl DParams {
    /// Set number of theads for decompression, defaults to 1
    pub fn set_threads(mut self, n: usize) -> Self {
        self.0.nthreads = n as _;
        self
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

/// Wrapper to [blosc2_context].  
/// Container struct for de/compression ops requiring context when used in multithreaded environments
///
/// [blosc2_context]: blosc2_sys::blosc2_context
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
            return Err(Blosc2Error::from(code).into());
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
        return Err(Blosc2Error::from(size).into());
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
        return Err(Blosc2Error::from(n_bytes).into());
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
        return Err(Blosc2Error::from(n_bytes).into());
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
        return Err(Blosc2Error::from(n_bytes).into());
    }
    Ok(n_bytes as _)
}

#[inline]
pub fn set_compressor(codec: Codec) -> Result<()> {
    let codec = match codec {
        Codec::BloscLz => CString::new("blosclz").unwrap(),
        _ => unimplemented!("Only blosclz codec supported for now"),
    };
    let rc = unsafe { ffi::blosc1_set_compressor(codec.as_ptr()) };
    if rc < 0 {
        Err(Blosc2Error::from(rc).into())
    } else {
        Ok(())
    }
}

/// Call before using blosc2, unless using specific ctx de/compression variants
pub fn init() {
    unsafe { ffi::blosc2_init() }
}

/// Call at end of using blosc2 library, unless you've never called `blosc2_init`
pub fn destroy() {
    unsafe { ffi::blosc2_destroy() }
}

/// Possible errors arising from Blosc2 library
#[derive(Debug)]
#[repr(i32)]
pub enum Blosc2Error {
    /// Generic failure
    Failure = ffi::BLOSC2_ERROR_FAILURE,
    /// Bad stream
    Stream = ffi::BLOSC2_ERROR_STREAM,
    /// Invalid data
    Data = ffi::BLOSC2_ERROR_DATA,
    /// Memory alloc/realloc failure
    MemoryAlloc = ffi::BLOSC2_ERROR_MEMORY_ALLOC,
    /// Not enough space to read
    ReadBuffer = ffi::BLOSC2_ERROR_READ_BUFFER,
    /// Not enough space to write
    WriteBuffer = ffi::BLOSC2_ERROR_WRITE_BUFFER,
    /// Codec not supported
    CodecSupport = ffi::BLOSC2_ERROR_CODEC_SUPPORT,
    /// Invalid parameter supplied to codec
    CodecParam = ffi::BLOSC2_ERROR_CODEC_PARAM,
    /// Codec dictionary error
    CodecDict = ffi::BLOSC2_ERROR_CODEC_DICT,
    /// Version not supported
    VersionSupport = ffi::BLOSC2_ERROR_VERSION_SUPPORT,
    /// Invalid value in header
    InvalidHeader = ffi::BLOSC2_ERROR_INVALID_HEADER,
    /// Invalid parameter supplied to function
    InvalidParam = ffi::BLOSC2_ERROR_INVALID_PARAM,
    /// File read failure
    FileRead = ffi::BLOSC2_ERROR_FILE_READ,
    /// File write failure
    FileWrite = ffi::BLOSC2_ERROR_FILE_WRITE,
    /// File open failure
    FileOpen = ffi::BLOSC2_ERROR_FILE_OPEN,
    /// Not found
    NotFound = ffi::BLOSC2_ERROR_NOT_FOUND,
    /// Bad run length encoding
    RunLength = ffi::BLOSC2_ERROR_RUN_LENGTH,
    /// Filter pipeline error
    FilterPipeline = ffi::BLOSC2_ERROR_FILTER_PIPELINE,
    /// Chunk insert failure
    ChunkInsert = ffi::BLOSC2_ERROR_CHUNK_INSERT,
    /// Chunk append failure
    ChunkAppend = ffi::BLOSC2_ERROR_CHUNK_APPEND,
    /// Chunk update failure
    ChunkUpdate = ffi::BLOSC2_ERROR_CHUNK_UPDATE,
    /// Sizes larger than 2gb not supported
    TwoGBLimit = ffi::BLOSC2_ERROR_2GB_LIMIT,
    /// Super-chunk copy failure
    SchunkCopy = ffi::BLOSC2_ERROR_SCHUNK_COPY,
    /// Wrong type for frame
    FrameType = ffi::BLOSC2_ERROR_FRAME_TYPE,
    /// File truncate failure
    FileTruncate = ffi::BLOSC2_ERROR_FILE_TRUNCATE,
    /// Thread or thread context creation failure
    ThreadCreate = ffi::BLOSC2_ERROR_THREAD_CREATE,
    /// Postfilter failure
    PostFilter = ffi::BLOSC2_ERROR_POSTFILTER,
    /// Special frame failure
    FrameSpecial = ffi::BLOSC2_ERROR_FRAME_SPECIAL,
    /// Special super-chunk failure
    SchunkSpecial = ffi::BLOSC2_ERROR_SCHUNK_SPECIAL,
    /// IO plugin error
    PluginIO = ffi::BLOSC2_ERROR_PLUGIN_IO,
    /// Remove file failure
    FileRemove = ffi::BLOSC2_ERROR_FILE_REMOVE,
    /// Pointer is null
    NullPointer = ffi::BLOSC2_ERROR_NULL_POINTER,
    /// Invalid index
    InvalidIndex = ffi::BLOSC2_ERROR_INVALID_INDEX,
    /// Metalayer has not been found
    MetalayerNotFound = ffi::BLOSC2_ERROR_METALAYER_NOT_FOUND,
}

impl std::fmt::Display for Blosc2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Blosc2Error {}

impl From<i32> for Blosc2Error {
    fn from(code: i32) -> Self {
        match code {
            ffi::BLOSC2_ERROR_FAILURE => Blosc2Error::Failure,
            ffi::BLOSC2_ERROR_STREAM => Blosc2Error::Stream,
            ffi::BLOSC2_ERROR_DATA => Blosc2Error::Data,
            ffi::BLOSC2_ERROR_MEMORY_ALLOC => Blosc2Error::MemoryAlloc,
            ffi::BLOSC2_ERROR_READ_BUFFER => Blosc2Error::ReadBuffer,
            ffi::BLOSC2_ERROR_WRITE_BUFFER => Blosc2Error::WriteBuffer,
            ffi::BLOSC2_ERROR_CODEC_SUPPORT => Blosc2Error::CodecSupport,
            ffi::BLOSC2_ERROR_CODEC_PARAM => Blosc2Error::CodecParam,
            ffi::BLOSC2_ERROR_CODEC_DICT => Blosc2Error::CodecDict,
            ffi::BLOSC2_ERROR_VERSION_SUPPORT => Blosc2Error::VersionSupport,
            ffi::BLOSC2_ERROR_INVALID_HEADER => Blosc2Error::InvalidHeader,
            ffi::BLOSC2_ERROR_INVALID_PARAM => Blosc2Error::InvalidParam,
            ffi::BLOSC2_ERROR_FILE_READ => Blosc2Error::FileRead,
            ffi::BLOSC2_ERROR_FILE_WRITE => Blosc2Error::FileWrite,
            ffi::BLOSC2_ERROR_FILE_OPEN => Blosc2Error::FileOpen,
            ffi::BLOSC2_ERROR_NOT_FOUND => Blosc2Error::NotFound,
            ffi::BLOSC2_ERROR_RUN_LENGTH => Blosc2Error::RunLength,
            ffi::BLOSC2_ERROR_FILTER_PIPELINE => Blosc2Error::FilterPipeline,
            ffi::BLOSC2_ERROR_CHUNK_INSERT => Blosc2Error::ChunkInsert,
            ffi::BLOSC2_ERROR_CHUNK_APPEND => Blosc2Error::ChunkAppend,
            ffi::BLOSC2_ERROR_CHUNK_UPDATE => Blosc2Error::ChunkUpdate,
            ffi::BLOSC2_ERROR_2GB_LIMIT => Blosc2Error::TwoGBLimit,
            ffi::BLOSC2_ERROR_SCHUNK_COPY => Blosc2Error::SchunkCopy,
            ffi::BLOSC2_ERROR_FRAME_TYPE => Blosc2Error::FrameType,
            ffi::BLOSC2_ERROR_FILE_TRUNCATE => Blosc2Error::FileTruncate,
            ffi::BLOSC2_ERROR_THREAD_CREATE => Blosc2Error::ThreadCreate,
            ffi::BLOSC2_ERROR_POSTFILTER => Blosc2Error::PostFilter,
            ffi::BLOSC2_ERROR_FRAME_SPECIAL => Blosc2Error::FrameSpecial,
            ffi::BLOSC2_ERROR_SCHUNK_SPECIAL => Blosc2Error::SchunkSpecial,
            ffi::BLOSC2_ERROR_PLUGIN_IO => Blosc2Error::PluginIO,
            ffi::BLOSC2_ERROR_FILE_REMOVE => Blosc2Error::FileRemove,
            ffi::BLOSC2_ERROR_NULL_POINTER => Blosc2Error::NullPointer,
            ffi::BLOSC2_ERROR_INVALID_INDEX => Blosc2Error::InvalidIndex,
            ffi::BLOSC2_ERROR_METALAYER_NOT_FOUND => Blosc2Error::MetalayerNotFound,
            _ => panic!("Error code {} not matched in existing Error codes", code),
        }
    }
}

#[cfg(test)]
mod tests {
    use ctor::{ctor, dtor};
    use std::io::{Cursor, Read};

    use super::*;

    #[ctor]
    fn blosc2_init() {
        init();
    }

    #[dtor]
    fn blosc2_destory() {
        destroy();
    }

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

    #[test]
    fn test_schunk_basic() -> Result<()> {
        let input = b"some data";
        let storage = schunk::Storage::default()
            .set_contiguous(true)
            .set_cparams(&mut CParams::from(&input[0]))
            .set_dparams(&mut DParams::default());
        let mut schunk = schunk::SChunk::new(storage);

        assert!(schunk.is_contiguous());
        assert_eq!(schunk.typesize(), 8);
        assert!(schunk.path().is_none());

        let mut decompressed = vec![0u8; input.len()];

        let n = schunk.append_buffer(input)?;
        schunk.decompress_chunk(n - 1, &mut decompressed)?;
        assert_eq!(input, decompressed.as_slice());

        // Reconstruct thru slice
        let mut v = schunk.into_vec()?;
        {
            schunk = schunk::SChunk::from_slice(&mut v)?;
            assert_eq!(schunk.n_chunks(), 1);
        }

        // Reconstruct thru vec
        schunk = schunk::SChunk::from_vec(v)?;
        assert_eq!(schunk.n_chunks(), 1);

        Ok(())
    }
}
