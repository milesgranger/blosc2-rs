//! Blosc2 Rust bindings.

use std::ffi::c_void;
use std::ffi::CStr;
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

impl TryInto<String> for Codec {
    type Error = Box<dyn std::error::Error>;
    fn try_into(self) -> Result<String> {
        let mut compname = std::ptr::null();
        let rc = unsafe { ffi::blosc2_compcode_to_compname(self as _, &mut compname) };
        if rc == -1 {
            return Err(format!("Unsupported Codec {:?}", self).into());
        }
        unsafe {
            CString::from_raw(compname as _)
                .into_string()
                .map_err(|e| e.to_string().into())
        }
    }
}
impl<'a> TryFrom<&'a str> for Codec {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: &'a str) -> Result<Self> {
        let compname = CString::new(value)?;
        let compcode = unsafe { ffi::blosc2_compname_to_compcode(compname.as_ptr()) };
        if compcode == -1 {
            return Err(format!("Compcode {} not recognized", value).into());
        }
        Codec::try_from(compcode)
    }
}
impl TryFrom<i32> for Codec {
    type Error = Box<dyn std::error::Error>;

    fn try_from(compcode: i32) -> Result<Self> {
        match compcode as u32 {
            ffi::BLOSC_BLOSCLZ => Ok(Codec::BloscLz),
            ffi::BLOSC_LZ4 => Ok(Codec::LZ4),
            ffi::BLOSC_LZ4HC => Ok(Codec::LZ4HC),
            ffi::BLOSC_ZLIB => Ok(Codec::ZLIB),
            ffi::BLOSC_ZSTD => Ok(Codec::ZSTD),
            ffi::BLOSC_LAST_CODEC => Ok(Codec::LastCodec),
            ffi::BLOSC_LAST_REGISTERED_CODEC => Ok(Codec::LastRegisteredCodec),
            _ => Err(format!("Not match for compcode {}", compcode).into()),
        }
    }
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
    //! `blosc2_schunk`,`blosc2_storage`, and `Chunk` APIs

    use std::ffi::CStr;
    use std::io;
    use std::path::{Path, PathBuf};

    use super::*;

    /// Wrapper to [blosc2_storage]
    ///
    /// [blosc2_storage]: blosc2_sys::blosc2_storage
    ///
    /// Example
    /// -------
    /// ```
    /// use blosc2::schunk::Storage;
    ///
    /// let storage = Storage::default().set_urlpath("/some/path.blosc2");
    /// ```
    #[derive(Default)]
    pub struct Storage(ffi::blosc2_storage);

    impl Storage {
        /// Set url/file path to specify a file-backed `schunk`.
        /// if not set, defaults to an in-memory `schunk`
        pub fn set_urlpath<S: AsRef<Path>>(mut self, urlpath: S) -> Result<Self> {
            self.0.urlpath =
                CString::new(urlpath.as_ref().to_string_lossy().to_string())?.into_raw();
            Ok(self)
        }
        /// Reference to the urlpath (if any)
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::schunk::Storage;
        ///
        /// let storage = Storage::default().set_urlpath("/some/path.blosc2").unwrap();
        /// assert_eq!(storage.get_urlpath().unwrap().unwrap(), "/some/path.blosc2");
        /// ```
        pub fn get_urlpath(&self) -> Result<Option<&str>> {
            if self.0.urlpath.is_null() {
                return Ok(None);
            }
            unsafe {
                CStr::from_ptr(self.0.urlpath)
                    .to_str()
                    .map(|v| Some(v))
                    .map_err(Into::into)
            }
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

    /// Wraps a single chunk of a super-chunk.
    ///
    /// Normally constructed via `Chunk::from_schunk`
    ///
    /// Example
    /// -------
    /// ```
    /// use std::io::Write;
    /// use blosc2::{CParams, DParams};
    /// use blosc2::schunk::{Storage, SChunk, Chunk};
    ///
    /// let input = b"some data";
    /// let storage = Storage::default()
    ///     .set_contiguous(true)
    ///     .set_cparams(&mut CParams::from(&input[0]))
    ///     .set_dparams(&mut DParams::default());
    /// let mut schunk = SChunk::new(storage);
    ///
    /// let n = schunk.write(input).unwrap();  // same as schunk.append_buffer(input)?;
    /// assert_eq!(n as usize, input.len());
    /// assert_eq!(schunk.n_chunks(), 1);
    ///
    /// let chunk = Chunk::from_schunk(&mut schunk, 0).unwrap();  // Get first (and only) chunk
    /// assert_eq!(chunk.info().unwrap().nbytes() as usize, input.len());
    /// ```
    pub struct Chunk {
        pub(crate) chunk: *mut u8,
        pub(crate) needs_free: bool,
    }

    impl Chunk {
        /// Create a new `Chunk` directly from a pointer, you probably
        /// want `Chunk::from_schunk` instead.
        pub fn new(chunk: *mut u8, needs_free: bool) -> Self {
            Self { chunk, needs_free }
        }

        /// Create a chunk made of uninitialized values
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::CParams;
        /// use blosc2::schunk::Chunk;
        ///
        /// let chunk = Chunk::uninit::<u8>(CParams::default(), 10).unwrap();
        /// assert_eq!(chunk.info().unwrap().nbytes(), 10);
        /// ```
        pub fn uninit<T>(cparams: CParams, len: usize) -> Result<Self> {
            let mut dst: Vec<T> =
                Vec::with_capacity(len + ffi::BLOSC_EXTENDED_HEADER_LENGTH as usize);
            if std::mem::size_of::<T>() != cparams.0.typesize as usize {
                return Err("typesize mismatch between CParams and T".into());
            }
            let nbytes = unsafe {
                ffi::blosc2_chunk_uninit(
                    cparams.0,
                    len as i32 * cparams.0.typesize,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.capacity() as i32 * cparams.0.typesize, // size in bytes
                )
            };
            if nbytes < 0 {
                return Err("Failed to create uninitialized chunk".into());
            }
            unsafe { dst.set_len(nbytes as _) };
            let ptr = dst.as_mut_ptr();
            std::mem::forget(dst);
            Ok(Self::new(ptr as _, true))
        }

        /// Create a chunk made of zeros
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::CParams;
        /// use blosc2::schunk::Chunk;
        ///
        /// let chunk = Chunk::zeros::<u8>(CParams::default(), 10).unwrap();
        /// assert_eq!(chunk.info().unwrap().nbytes(), 10);
        /// ```
        pub fn zeros<T>(cparams: CParams, len: usize) -> Result<Self> {
            if std::mem::size_of::<T>() != cparams.0.typesize as usize {
                return Err("typesize mismatch between CParams and T".into());
            }
            let mut dst: Vec<T> = Vec::with_capacity(
                (len * cparams.0.typesize as usize) + ffi::BLOSC_EXTENDED_HEADER_LENGTH as usize,
            );

            let nbytes = unsafe {
                ffi::blosc2_chunk_zeros(
                    cparams.0,
                    len as i32 * cparams.0.typesize,
                    dst.as_mut_ptr() as _,
                    dst.capacity() as i32, // size in bytes
                )
            };
            dbg!(nbytes);
            if nbytes < 0 {
                return Err(Box::new(Blosc2Error::from(nbytes)));
            }
            unsafe { dst.set_len(nbytes as usize) };
            let ptr = dst.as_mut_ptr();
            std::mem::forget(dst);
            Ok(Self::new(ptr as _, true))
        }

        /// Decompress the current chunk
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::CParams;
        /// use blosc2::schunk::Chunk;
        ///
        /// let cparams = CParams::default().set_typesize::<i64>();
        /// let mut chunk = Chunk::zeros::<i64>(cparams, 5).unwrap();
        ///
        /// assert_eq!(chunk.info().unwrap().nbytes(), 40);  // 5 elements * 64bit == 40bytes
        /// let decompressed = chunk.decompress::<i64>().unwrap();
        /// dbg!(decompressed.len());
        /// assert_eq!(decompressed, vec![0i64; 5]);
        /// ```
        pub fn decompress<T>(&mut self) -> Result<Vec<T>> {
            let slice = unsafe { std::slice::from_raw_parts(self.chunk, self.info()?.cbytes) };
            crate::decompress(slice)
        }

        /// Create a new `Chunk` from a `SChunk`
        #[inline]
        pub fn from_schunk(schunk: &mut SChunk, nchunk: usize) -> Result<Self> {
            let mut chunk: *mut u8 = std::ptr::null_mut();
            let mut needs_free: bool = false;
            let rc = unsafe {
                ffi::blosc2_schunk_get_chunk(
                    schunk.0,
                    nchunk as _,
                    &mut chunk as _,
                    &mut needs_free,
                )
            };
            if rc < 0 {
                return Err(Blosc2Error::from(rc as i32).into());
            }
            Ok(Self { chunk, needs_free })
        }
        /// Get `CompressedBufferInfo` for this chunk.
        #[inline]
        pub fn info(&self) -> Result<CompressedBufferInfo> {
            let mut nbytes = 0;
            let mut cbytes = 0;
            let mut blocksize = 0;
            let rc = unsafe {
                ffi::blosc2_cbuffer_sizes(self.chunk as _, &mut nbytes, &mut cbytes, &mut blocksize)
            };
            if rc < 0 {
                return Err(Blosc2Error::from(rc).into());
            }
            Ok(CompressedBufferInfo {
                nbytes: nbytes as _,
                cbytes: cbytes as _,
                blocksize: blocksize as _,
            })
        }
    }

    impl Drop for Chunk {
        fn drop(&mut self) {
            if self.needs_free {
                unsafe { ffi::free(self.chunk as _) };
            }
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
        #[inline]
        pub fn append_buffer<T>(&mut self, data: &[T]) -> Result<usize> {
            if data.is_empty() {
                return Ok(self.inner().nchunks as usize);
            }

            let size = std::mem::size_of::<T>();
            let typesize = self.inner().typesize as _;
            if size != typesize {
                let msg = format!("Size of T ({}) != schunk typesize ({})", size, typesize);
                return Err(msg.into());
            }
            self.append_buffer_unchecked(data)
        }

        /// Same as `append_buffer` but will not do any preliminary checks for matching typesize
        /// or if input buffer is empty.
        #[inline]
        pub fn append_buffer_unchecked<T>(&mut self, data: &[T]) -> Result<usize> {
            let n = unsafe {
                ffi::blosc2_schunk_append_buffer(self.0, data.as_ptr() as _, data.len() as _)
            };
            if n < 0 {
                return Err(Blosc2Error::from(n as i32).into());
            }
            Ok(n as _)
        }

        /// Decompress a chunk, returning number of bytes written to output buffer
        #[inline]
        pub fn decompress_chunk<T>(&mut self, nchunk: usize, dst: &mut [T]) -> Result<usize> {
            let chunk = Chunk::from_schunk(self, nchunk)?;
            let info = chunk.info()?;
            if dst.len() < info.nbytes as usize {
                let msg = format!(
                    "Not large enough, need {} but got {}",
                    info.nbytes,
                    dst.len()
                );
                return Err(msg.into());
            }

            let ptr = dst.as_mut_ptr() as _;
            let size = unsafe {
                ffi::blosc2_schunk_decompress_chunk(self.0, nchunk as _, ptr, info.nbytes as _)
            };

            if size < 0 {
                return Err(Blosc2Error::from(size).into());
            } else if size == 0 {
                let msg = format!("Non-initialized error decompressing chunk '{}'", nchunk);
                return Err(msg.into());
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

    impl io::Write for SChunk {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.append_buffer(buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// struct only used for wrapping a mutable reference to a `SChunk`
    /// to support `std::io::Read` decoding for a SChunk.
    ///
    /// This isn't needed for encoding `std::io::Write` since we can directly
    /// write buffers into SChunk. However, for `Read`, we can't be certain the
    /// decompressed chunks will fit into the supplied `&mut [u8]` buffer provided
    /// during `Read::read`. So this struct exists only to hold those intermediate
    /// buffer and position variables and not clutter `SChunk` implementation.
    pub struct SChunkDecoder<'schunk> {
        pub(crate) schunk: &'schunk mut SChunk,
        pub(crate) buf: io::Cursor<Vec<u8>>,
        pub(crate) nchunk: usize,
    }
    impl<'schunk> SChunkDecoder<'schunk> {
        pub fn new(schunk: &'schunk mut SChunk) -> Self {
            Self {
                schunk,
                buf: io::Cursor::new(vec![]),
                nchunk: 0,
            }
        }
    }

    impl<'schunk> io::Read for SChunkDecoder<'schunk> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            // Cursor is at end of buffer, so we can refill
            if self.buf.position() as usize == self.buf.get_ref().len() {
                if self.nchunk >= self.schunk.n_chunks() {
                    return Ok(0);
                }
                self.buf.get_mut().truncate(0);
                self.buf.set_position(0);

                // Get chunk and check if we can decompress directly into caller's buffer
                let chunk = Chunk::from_schunk(self.schunk, self.nchunk)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                let nbytes = chunk
                    .info()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
                    .nbytes();

                if nbytes <= buf.len() {
                    self.schunk
                        .decompress_chunk(self.nchunk, buf)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                    self.nchunk += 1;
                    return Ok(nbytes);
                } else {
                    self.buf.get_mut().resize(nbytes as _, 0u8);
                    let nbytes_written = self
                        .schunk
                        .decompress_chunk(self.nchunk, self.buf.get_mut().as_mut_slice())
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

                    // These should always be equal, otherwise blosc2 gave the wrong expected
                    // uncompressed size of this chunk.
                    debug_assert_eq!(nbytes_written, nbytes);
                }
                self.nchunk += 1;
            }
            self.buf.read(buf)
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
                None,
                None,
                None,
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
    /// Set the type size
    pub fn set_typesize<T>(mut self) -> Self {
        self.0.typesize = std::mem::size_of::<T>() as _;
        self
    }
}

impl Default for CParams {
    #[inline]
    fn default() -> Self {
        let mut cparams = ffi::blosc2_cparams::default();
        cparams.compcode = Codec::default() as _;
        cparams.clevel = CLevel::default() as _;
        cparams.typesize = 1;
        cparams.splitmode = ffi::BLOSC_FORWARD_COMPAT_SPLIT as _;
        cparams.filters[ffi::BLOSC2_MAX_FILTERS as usize - 1] = Filter::default() as _;
        cparams.nthreads = 1;
        Self(cparams)
    }
}

/// Create CParams from a reference to the type being compressed
impl<T> From<&T> for CParams {
    fn from(_: &T) -> Self {
        let mut cparams = CParams::default();
        cparams.0.typesize = std::mem::size_of::<T>() as _;
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
    nbytes: usize,
    /// Number of bytes to be read from compressed buffer
    cbytes: usize,
    /// Used internally by blosc2 when compressing the blocks, exposed here for completion.
    /// You probably won't need it.
    blocksize: usize,
}

impl CompressedBufferInfo {
    pub fn nbytes(&self) -> usize {
        self.nbytes
    }
    pub fn cbytes(&self) -> usize {
        self.cbytes
    }
    pub fn blocksize(&self) -> usize {
        self.blocksize
    }
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
    let mut dst = vec![src[0].clone(); max_compress_len(src)];
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

/// Return the max size a compressed buffer needs to be to hold `src`
#[inline(always)]
pub fn max_compress_len<T>(src: &[T]) -> usize {
    src.len() + ffi::BLOSC2_MAX_OVERHEAD as usize
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
    let mut dst = vec![src[0].clone(); max_compress_len(src)];
    let n_bytes = compress_into(src, &mut dst, clevel, filter, codec)?;

    dst.truncate(n_bytes);
    Ok(dst)
}

#[inline]
pub fn compress_into<T>(
    src: &[T],
    dst: &mut [T],
    clevel: Option<CLevel>,
    filter: Option<Filter>,
    codec: Option<Codec>,
) -> Result<usize> {
    if src.is_empty() {
        return Ok(0);
    }
    let typesize = std::mem::size_of::<T>();
    set_compressor(codec.unwrap_or_default())?;
    let n_bytes = unsafe {
        ffi::blosc2_compress(
            clevel.unwrap_or_default() as _,
            filter.unwrap_or_default() as _,
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
pub fn decompress_ctx<T>(src: &[u8], ctx: &mut Context) -> Result<Vec<T>> {
    if src.is_empty() {
        return Ok(vec![]);
    }
    let info = CompressedBufferInfo::try_from(src)?;
    let n_elements = info.nbytes as usize / std::mem::size_of::<T>();
    let mut dst = Vec::with_capacity(n_elements);

    let n_bytes = unsafe {
        ffi::blosc2_decompress_ctx(
            ctx.0,
            src.as_ptr() as *const c_void,
            src.len() as i32,
            dst.as_mut_ptr() as *mut c_void,
            info.nbytes as _,
        )
    };

    if n_bytes < 0 {
        return Err(Blosc2Error::from(n_bytes).into());
    }
    debug_assert_eq!(n_bytes as usize, info.nbytes);
    unsafe { dst.set_len(n_elements) };
    Ok(dst)
}

#[inline]
pub fn decompress_into_ctx<T>(src: &[T], dst: &mut [T], ctx: &mut Context) -> Result<usize> {
    if src.is_empty() {
        return Ok(0);
    }
    let info = CompressedBufferInfo::try_from(src)?;
    let n_bytes = unsafe {
        ffi::blosc2_decompress_ctx(
            ctx.0,
            src.as_ptr() as *const c_void,
            src.len() as i32,
            dst.as_mut_ptr() as *mut c_void,
            info.nbytes as _,
        )
    };

    if n_bytes < 0 {
        return Err(Blosc2Error::from(n_bytes).into());
    }
    debug_assert_eq!(n_bytes as usize, info.nbytes);
    Ok(n_bytes as _)
}

#[inline]
pub fn decompress<T>(src: &[u8]) -> Result<Vec<T>> {
    if src.is_empty() {
        return Ok(vec![]);
    }

    // blosc2 plays by bytes, we'll go by however many bytes per element
    // to set the vec length in actual elements
    let info = CompressedBufferInfo::try_from(src)?;
    let n_elements = info.nbytes as usize / std::mem::size_of::<T>();
    let mut dst = Vec::with_capacity(n_elements);

    let n_bytes = unsafe {
        ffi::blosc2_decompress(
            src.as_ptr() as *const c_void,
            src.len() as i32,
            dst.as_mut_ptr() as *mut c_void,
            info.nbytes as _,
        )
    };

    if n_bytes < 0 {
        return Err(Blosc2Error::from(n_bytes).into());
    }

    debug_assert_eq!(n_bytes as usize, info.nbytes);
    unsafe { dst.set_len(n_elements) };

    Ok(dst)
}

#[inline]
pub fn decompress_into<T>(src: &[u8], dst: &mut [T]) -> Result<usize> {
    let info = CompressedBufferInfo::try_from(src)?;
    let n_bytes = unsafe {
        ffi::blosc2_decompress(
            src.as_ptr() as *const c_void,
            src.len() as i32,
            dst.as_mut_ptr() as *mut c_void,
            info.nbytes as _,
        )
    };

    if n_bytes < 0 {
        return Err(Blosc2Error::from(n_bytes).into());
    }
    debug_assert_eq!(n_bytes as usize, info.nbytes);
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

pub fn set_nthreads(nthreads: usize) -> usize {
    let n = unsafe { ffi::blosc2_set_nthreads(nthreads as _) };
    n as _
}
pub fn get_nthreads() -> usize {
    unsafe { ffi::blosc2_get_nthreads() as _ }
}

/// Get the version for a given Codec.
pub fn get_complib_info(codec: Codec) -> Result<String> {
    let mut compname = std::ptr::null();
    let mut complib = std::ptr::null_mut();
    let mut version = std::ptr::null_mut();

    if unsafe { ffi::blosc2_compcode_to_compname(codec as i32, &mut compname) } == -1 {
        return Err("Codec not recognized".into());
    }
    if unsafe { ffi::blosc2_get_complib_info(compname as _, &mut complib, &mut version) } == -1 {
        return Err("Codec not supported".into());
    }

    let info = unsafe {
        format!(
            "{}: {}",
            CString::from_raw(complib)
                .into_string()
                .map_err(|e| e.to_string())?,
            CString::from_raw(version)
                .into_string()
                .map_err(|e| e.to_string())?
        )
    };
    Ok(info)
}

/// Get the Blosc2 version string
pub fn get_version_string() -> Result<String> {
    CStr::from_bytes_with_nul(ffi::BLOSC2_VERSION_STRING)
        .map_err(|e| e.to_string())?
        .to_str()
        .map_err(|e| e.to_string().into())
        .map(|s| s.to_string())
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
        let n_bytes = compress_into(input, &mut compressed, None, None, None)?;

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
        assert_eq!(n_compressed, 1020);

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
        assert_eq!(schunk.typesize(), 1);
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

    #[test]
    fn test_schunk_write() -> Result<()> {
        let input = std::iter::repeat(b"some data")
            .take(BUFSIZE)
            .flat_map(|v| v.to_vec())
            .collect::<Vec<u8>>();
        let storage = schunk::Storage::default()
            .set_contiguous(true)
            .set_cparams(&mut CParams::from(&input[0]))
            .set_dparams(&mut DParams::default());
        let mut schunk = schunk::SChunk::new(storage);

        let nbytes = std::io::copy(&mut Cursor::new(input.clone()), &mut schunk)?;
        assert_eq!(nbytes as usize, input.len());

        let ratio = schunk.compression_ratio(); // ~36.
        assert!(84. < ratio);
        assert!(86. > ratio);

        let mut uncompressed = vec![];
        let mut decoder = schunk::SChunkDecoder::new(&mut schunk);
        let n = std::io::copy(&mut decoder, &mut uncompressed)?;
        assert_eq!(input, uncompressed.as_slice());
        assert_eq!(n as usize, input.len());

        Ok(())
    }

    #[test]
    fn test_get_version_string() -> Result<()> {
        let version = get_version_string()?;
        assert_eq!(&version, "2.8.0");
        Ok(())
    }

    #[test]
    fn test_get_complib_version_string() -> Result<()> {
        let info = get_complib_info(Codec::BloscLz)?;
        assert_eq!(&info, "BloscLZ: 2.5.2");
        Ok(())
    }
}
