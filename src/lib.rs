//! Blosc2 Rust bindings.

use blosc2_sys as ffi;
use parking_lot::RwLock;
use std::ffi::{c_void, CStr, CString};
use std::sync::Arc;
use std::{io, mem};

pub const BLOSC2_VERSION_DATE: &'static str =
    unsafe { std::str::from_utf8_unchecked(blosc2_sys::BLOSC2_VERSION_DATE) };
pub const BLOSC2_VERSION_STRING: &'static str =
    unsafe { std::str::from_utf8_unchecked(blosc2_sys::BLOSC2_VERSION_STRING) };
pub use blosc2_sys::{
    BLOSC2_MAX_DIM, BLOSC2_VERSION_MAJOR, BLOSC2_VERSION_MINOR, BLOSC2_VERSION_RELEASE,
};

/// Result type used in this library
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// A Blosc2 library specific error. Often mapped to the
    /// raw error return code
    Blosc2(Blosc2Error),
    /// Any other error, converted into a string
    Other(String),
}
impl From<Blosc2Error> for Error {
    fn from(err: Blosc2Error) -> Self {
        Error::Blosc2(err)
    }
}
impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Other(err)
    }
}
impl<'a> From<&'a str> for Error {
    fn from(err: &'a str) -> Self {
        Error::Other(err.to_string())
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err.to_string())
    }
}
impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
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

/// Default buffer size for intermediate de/compression results when required
pub const BUFSIZE: usize = 8196_usize;

/// Possible Filters
#[derive(Copy, Clone)]
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

impl ToString for Filter {
    fn to_string(&self) -> String {
        match self {
            Self::NoFilter => "nofilter",
            Self::Shuffle => "shuffle",
            Self::BitShuffle => "bitshuffle",
            Self::Delta => "delta",
            Self::TruncPrec => "truncprec",
            Self::LastFilter => "lastfilter",
            Self::LastRegisteredFilter => "lastregisteredfilter",
        }
        .to_string()
    }
}

impl<'a> TryFrom<&'a str> for Filter {
    type Error = Error;

    fn try_from(val: &'a str) -> Result<Self> {
        match val.to_lowercase().as_str() {
            "nofilter" => Ok(Filter::NoFilter),
            "shuffle" => Ok(Filter::Shuffle),
            "bitshuffle" => Ok(Filter::BitShuffle),
            "delta" => Ok(Filter::Delta),
            "trunctprec" => Ok(Filter::TruncPrec),
            "lastfilter" => Ok(Filter::LastFilter),
            "lastregisteredfilter" => Ok(Filter::LastRegisteredFilter),
            _ => Err(Error::from(format!("No matching filter for '{}'", val))),
        }
    }
}

/// Possible compression codecs
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Codec {
    BloscLz = ffi::BLOSC_BLOSCLZ as _,
    LZ4 = ffi::BLOSC_LZ4 as _,
    LZ4HC = ffi::BLOSC_LZ4HC as _,
    ZLIB = ffi::BLOSC_ZLIB as _,
    ZSTD = ffi::BLOSC_ZSTD as _,
    LastCodec = ffi::BLOSC_LAST_CODEC as _,
    LastRegisteredCodec = ffi::BLOSC_LAST_REGISTERED_CODEC as _,
}

impl Codec {
    #[allow(dead_code)]
    fn to_name(&self) -> Result<String> {
        (*self).try_into()
    }
    fn to_name_cstring(&self) -> Result<CString> {
        let mut compname = std::ptr::null();
        let rc = unsafe { ffi::blosc2_compcode_to_compname(*self as _, &mut compname) };
        if rc == -1 {
            return Err(Error::Other(format!("Unsupported Codec {:?}", self)));
        }
        unsafe { Ok(CStr::from_ptr(compname as _).to_owned()) }
    }
}

impl TryInto<String> for Codec {
    type Error = Error;
    fn try_into(self) -> Result<String> {
        self.clone()
            .to_name_cstring()?
            .into_string()
            .map_err(|e| Error::Other(e.to_string()))
    }
}
impl<'a> TryFrom<&'a str> for Codec {
    type Error = Error;

    fn try_from(value: &'a str) -> Result<Self> {
        let compname = CString::new(value).map_err(|e| Error::Other(e.to_string()))?;
        let compcode = unsafe { ffi::blosc2_compname_to_compcode(compname.as_ptr()) };
        if compcode == -1 {
            return Err(Error::from(format!("Compcode {} not recognized", value)));
        }
        Codec::try_from(compcode)
    }
}
impl TryFrom<i32> for Codec {
    type Error = Error;

    fn try_from(compcode: i32) -> Result<Self> {
        match compcode as _ {
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

impl TryFrom<usize> for CLevel {
    type Error = Error;

    fn try_from(val: usize) -> Result<Self> {
        match val {
            0 => Ok(CLevel::Zero),
            1 => Ok(CLevel::One),
            2 => Ok(CLevel::Two),
            3 => Ok(CLevel::Three),
            4 => Ok(CLevel::Four),
            5 => Ok(CLevel::Five),
            6 => Ok(CLevel::Six),
            7 => Ok(CLevel::Seven),
            8 => Ok(CLevel::Eight),
            9 => Ok(CLevel::Nine),
            _ => Err(Error::from(format!(
                "Compression level must be 0-9, got {}",
                val
            ))),
        }
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
    pub struct Storage {
        inner: ffi::blosc2_storage,
        cparams: CParams,
        dparams: DParams,
    }

    impl Default for Storage {
        fn default() -> Self {
            let storage = unsafe { ffi::blosc2_get_blosc2_storage_defaults() };
            Storage {
                inner: storage,
                cparams: CParams::default(),
                dparams: DParams::default(),
            }
        }
    }

    impl Storage {
        /// Set url/file path to specify a file-backed `schunk`.
        /// if not set, defaults to an in-memory `schunk`
        pub fn set_urlpath<S: AsRef<Path>>(mut self, urlpath: S) -> Result<Self> {
            self.inner.urlpath = CString::new(urlpath.as_ref().to_string_lossy().to_string())
                .map_err(|e| Error::Other(e.to_string()))?
                .into_raw();
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
            if self.inner.urlpath.is_null() {
                return Ok(None);
            }
            unsafe {
                CStr::from_ptr(self.inner.urlpath)
                    .to_str()
                    .map(|v| Some(v))
                    .map_err(|e| Error::Other(e.to_string()))
            }
        }
        /// Set the contiguous nature of the `schunk`.
        pub fn set_contiguous(mut self, contiguous: bool) -> Self {
            self.inner.contiguous = contiguous;
            self
        }
        /// Set compression parameters
        pub fn set_cparams(mut self, cparams: CParams) -> Self {
            self.cparams = cparams;
            self.inner.cparams = &mut self.cparams.0;
            self
        }
        /// Get compression parameters
        pub fn get_cparams(&self) -> &CParams {
            &self.cparams
        }
        /// Set decompression parameters
        pub fn set_dparams(mut self, dparams: DParams) -> Self {
            self.dparams = dparams;
            self.inner.dparams = &mut self.dparams.0;
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
    ///     .set_cparams(CParams::from(&input[0]))
    ///     .set_dparams(DParams::default());
    /// let mut schunk = SChunk::new(storage);
    ///
    /// let n = schunk.write(input).unwrap();  // same as schunk.append_buffer(input)?;
    /// assert_eq!(n as usize, input.len());
    /// assert_eq!(schunk.n_chunks(), 1);
    ///
    /// let chunk = Chunk::from_schunk(&mut schunk, 0).unwrap();  // Get first (and only) chunk
    /// assert_eq!(chunk.info().unwrap().nbytes() as usize, input.len());
    /// ```
    #[derive(Clone)]
    pub struct Chunk {
        pub(crate) chunk: Arc<RwLock<*mut u8>>,
        pub(crate) needs_free: bool,
    }

    unsafe impl Sync for Chunk {}
    unsafe impl Send for Chunk {}

    impl TryFrom<Vec<u8>> for Chunk {
        type Error = Error;
        #[inline]
        fn try_from(v: Vec<u8>) -> Result<Self> {
            Self::from_vec(v)
        }
    }

    impl Chunk {
        /// Create a new `Chunk` directly from a pointer, you probably
        /// want `Chunk::from_schunk` instead.
        pub fn new(chunk: *mut u8, needs_free: bool) -> Self {
            Self {
                chunk: Arc::new(RwLock::new(chunk)),
                needs_free,
            }
        }

        /// Construct Chunk from vector of bytes, this Vec is assumed to be the result of a valid
        /// chunk de/compression or other initialization method like uinit/zeros etc.
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::{schunk::Chunk, compress};
        ///
        /// let buf: Vec<u8> = compress(&vec![0i32, 1, 2, 3, 4], None, None, None, None).unwrap();
        /// assert!(Chunk::from_vec(buf).is_ok());
        /// ```
        #[inline]
        pub fn from_vec(v: Vec<u8>) -> Result<Self> {
            let mut v = v;
            v.shrink_to_fit();
            if let Err(_) = CompressedBufferInfo::try_from(v.as_slice()) {
                return Err("Appears this buffer is not a valid blosc2 chunk".into());
            }
            let ptr = v.as_mut_ptr();
            mem::forget(v);
            Ok(Self::new(ptr as _, true))
        }

        /// Export this Chunk into a `Vec<u8>`
        /// Maybe clone underlying vec if it's managed by blosc2
        pub fn into_vec(self) -> Result<Vec<u8>> {
            let info = self.info()?;
            let buf =
                unsafe { Vec::from_raw_parts(*self.chunk.write(), info.cbytes(), info.cbytes()) };
            if !self.needs_free {
                return Ok(buf.clone());
            }
            Ok(buf)
        }

        /// Get the raw buffer of this chunk
        pub fn as_slice(&self) -> Result<&[u8]> {
            let info = CompressedBufferInfo::try_from(*self.chunk.read() as *const c_void)?;
            let slice = unsafe { std::slice::from_raw_parts(*self.chunk.read(), info.cbytes()) };
            Ok(slice)
        }

        /// Return the number of elements in the compressed Chunk
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::{schunk::Chunk, compress};
        ///
        /// let chunk: Chunk = compress(&vec![0i32, 1, 2, 3, 4], None, None, None, None)
        ///    .unwrap()
        ///    .try_into()
        ///    .unwrap();
        /// assert_eq!(chunk.len::<i32>().unwrap(), 5);
        /// ```
        #[inline]
        pub fn len<T>(&self) -> Result<usize> {
            CompressedBufferInfo::try_from(*self.chunk.read() as *const c_void)
                .map(|info| info.nbytes() / mem::size_of::<T>())
        }

        #[inline]
        pub fn getitems<T>(&self, offset: usize, n_items: usize) -> Result<Vec<T>> {
            if self.len::<T>()? < offset + n_items {
                return Err(format!(
                    "Would be out of bounds. Chunk contains {} elements",
                    self.len::<T>()?
                )
                .into());
            }
            getitems(self.as_slice()?, offset, n_items)
        }

        /// Create a chunk made of uninitialized values
        ///
        /// Examplek
        /// -------
        /// ```
        /// use blosc2::CParams;
        /// use blosc2::schunk::Chunk;
        ///
        /// let chunk = Chunk::uninit::<u8>(CParams::default(), 10).unwrap();
        /// assert_eq!(chunk.info().unwrap().nbytes(), 10);
        /// ```
        pub fn uninit<T>(cparams: CParams, len: usize) -> Result<Self> {
            let mut cparams = cparams;
            if mem::size_of::<T>() != cparams.0.typesize as usize {
                cparams.0.typesize = mem::size_of::<T>() as _;
            }
            let mut dst = Vec::with_capacity(
                (len * cparams.0.typesize as usize) + ffi::BLOSC_EXTENDED_HEADER_LENGTH as usize,
            );
            let nbytes = unsafe {
                ffi::blosc2_chunk_uninit(
                    cparams.0,
                    len as i32 * cparams.0.typesize,
                    dst.as_mut_ptr() as *mut c_void,
                    dst.capacity() as i32,
                )
            };
            if nbytes < 0 {
                return Err("Failed to create uninitialized chunk".into());
            }
            unsafe { dst.set_len(nbytes as _) };
            Self::from_vec(dst)
        }

        /// Create a chunk made of repeating a value
        ///
        /// Examplek
        /// -------
        /// ```
        /// use blosc2::CParams;
        /// use blosc2::schunk::Chunk;
        ///
        /// let mut chunk = Chunk::repeatval::<f32>(CParams::default().set_typesize::<f32>(), 0.123, 4).unwrap();
        /// assert_eq!(chunk.info().unwrap().nbytes(), 16);  // 4 elements * 4 bytes each
        /// assert_eq!(chunk.decompress::<f32>().unwrap(), vec![0.123_f32, 0.123, 0.123, 0.123]);
        /// ```
        pub fn repeatval<T>(cparams: CParams, value: T, len: usize) -> Result<Self> {
            let mut cparams = cparams;
            if mem::size_of::<T>() != cparams.0.typesize as usize {
                cparams.0.typesize = mem::size_of::<T>() as _;
            }
            let mut dst = Vec::with_capacity(
                (len * cparams.0.typesize as usize) + ffi::BLOSC_EXTENDED_HEADER_LENGTH as usize,
            );
            let nbytes = unsafe {
                ffi::blosc2_chunk_repeatval(
                    cparams.0,
                    len as i32 * cparams.0.typesize,
                    dst.as_mut_ptr() as _,
                    dst.capacity() as _,
                    &value as *const T as _,
                )
            };
            if nbytes < 0 {
                return Err("Failed to create chunk".into());
            }
            unsafe { dst.set_len(nbytes as _) };
            Self::from_vec(dst)
        }

        /// Create a chunk made of zeros
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::CParams;
        /// use blosc2::schunk::Chunk;
        ///
        /// let cparams = CParams::default().set_typesize::<u32>();
        /// let chunk = Chunk::zeros::<u32>(cparams, 10).unwrap();
        /// assert_eq!(chunk.info().unwrap().nbytes(), 40);  // 10 elements * 4 bytes each
        /// ```
        pub fn zeros<T>(cparams: CParams, len: usize) -> Result<Self> {
            let mut cparams = cparams;
            if mem::size_of::<T>() != cparams.0.typesize as usize {
                cparams.0.typesize = mem::size_of::<T>() as _;
            }
            let mut dst = Vec::with_capacity(
                (len * cparams.0.typesize as usize) + ffi::BLOSC_EXTENDED_HEADER_LENGTH as usize,
            );

            let nbytes = unsafe {
                ffi::blosc2_chunk_zeros(
                    cparams.0,
                    len as i32 * cparams.0.typesize,
                    dst.as_mut_ptr() as _,
                    dst.capacity() as i32,
                )
            };
            if nbytes < 0 {
                return Err(Error::Blosc2(Blosc2Error::from(nbytes)));
            }
            unsafe { dst.set_len(nbytes as usize) };
            Self::from_vec(dst)
        }

        /// Compression ratio of the `Chunk`
        ///
        /// Example
        /// -------
        /// ```
        /// use blosc2::schunk::Chunk;
        ///
        /// let chunk = Chunk::compress(&vec![0i32; 1_000], None, None, None, None).unwrap();
        /// let ratio = chunk.compression_ratio().unwrap();
        /// assert_eq!(ratio, 125.0);
        /// ```
        pub fn compression_ratio(&self) -> Result<f32> {
            let info = self.info()?;
            if info.cbytes() == 0 {
                return Ok(0f32);
            }
            Ok(info.nbytes() as f32 / info.cbytes() as f32)
        }

        /// Helper method to construct a `Chunk` directly from compression.
        /// This is equivelent to:
        /// ```
        /// use blosc2::{compress, schunk::Chunk};
        ///
        /// let compressed = compress(&vec![0u8, 1, 2, 3], None, None, None, None).unwrap();
        /// let chunk = Chunk::from_vec(compressed).unwrap();
        /// assert_eq!(chunk.len::<u8>().unwrap(), 4);
        /// ```
        #[inline]
        pub fn compress<T: 'static>(
            src: &[T],
            typesize: Option<usize>,
            clevel: Option<CLevel>,
            filter: Option<Filter>,
            codec: Option<Codec>,
        ) -> Result<Self> {
            crate::compress(src, typesize, clevel, filter, codec).map(Self::from_vec)?
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
        pub fn decompress<T>(&self) -> Result<Vec<T>> {
            let slice =
                unsafe { std::slice::from_raw_parts(*self.chunk.read(), self.info()?.cbytes) };
            crate::decompress(slice)
        }

        /// Create a new `Chunk` from a `SChunk`
        #[inline]
        pub fn from_schunk(schunk: &mut SChunk, nchunk: usize) -> Result<Self> {
            let mut chunk: *mut u8 = std::ptr::null_mut();
            let mut needs_free: bool = false;
            let rc = unsafe {
                ffi::blosc2_schunk_get_chunk(
                    *schunk.0.read(),
                    nchunk as _,
                    &mut chunk as _,
                    &mut needs_free,
                )
            };
            if rc < 0 {
                return Err(Error::Blosc2(Blosc2Error::from(rc as i32)));
            }
            Ok(Self {
                chunk: Arc::new(RwLock::new(chunk)),
                needs_free,
            })
        }

        /// Uncompressed bytes in this Chunk
        pub fn nbytes(&self) -> Result<usize> {
            self.info().map(|info| info.nbytes)
        }

        /// Compressed bytes in this Chunk
        pub fn cbytes(&self) -> Result<usize> {
            self.info().map(|info| info.cbytes)
        }

        /// Get `CompressedBufferInfo` for this chunk.
        #[inline]
        pub fn info(&self) -> Result<CompressedBufferInfo> {
            let mut nbytes = 0;
            let mut cbytes = 0;
            let mut blocksize = 0;
            let rc = unsafe {
                ffi::blosc2_cbuffer_sizes(
                    *self.chunk.read() as _,
                    &mut nbytes,
                    &mut cbytes,
                    &mut blocksize,
                )
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
            // drop if needs freed and this is last strong ref
            if self.needs_free && Arc::strong_count(&self.chunk) == 1 {
                unsafe { blosc2_sys::libc::free(*self.chunk.write() as _) };
            }
        }
    }

    /// Wrapper to [blosc2_schunk]
    ///
    /// [blosc2_schunk]: blosc2_sys::blosc2_schunk
    #[derive(Clone)]
    pub struct SChunk(pub(crate) Arc<RwLock<*mut ffi::blosc2_schunk>>);

    unsafe impl Send for SChunk {}

    // Loosely inspired by blosc2-python implementation
    impl SChunk {
        pub fn new(storage: Storage) -> Self {
            let mut storage = storage;
            let schunk = unsafe { ffi::blosc2_schunk_new(&mut storage.inner) };
            Self(Arc::new(RwLock::new(schunk)))
        }

        pub fn copy(&self) -> Self {
            let schunk =
                unsafe { ffi::blosc2_schunk_copy(*self.0.read(), (**self.0.read()).storage) };
            Self(Arc::new(RwLock::new(schunk)))
        }

        pub fn frame(&self) -> Result<&[u8]> {
            unsafe {
                if (**self.0.read()).frame.is_null() {
                    return Err(Error::from("schunk frame is null"));
                }
                let len = ffi::blosc2_schunk_frame_len(*self.0.read()) as usize;
                let buf = std::slice::from_raw_parts((**self.0.read()).frame as _, len);
                Ok(buf)
            }
        }

        #[inline]
        pub(crate) fn inner(&self) -> &ffi::blosc2_schunk {
            unsafe { &(**self.0.read()) }
        }

        #[inline]
        #[allow(dead_code)]
        pub(crate) fn inner_mut(&mut self) -> &mut ffi::blosc2_schunk {
            unsafe { &mut (**self.0.write()) }
        }

        /// Append data to SChunk, returning new number of chunks
        #[inline]
        pub fn append_buffer<T>(&mut self, data: &[T]) -> Result<usize> {
            if data.is_empty() {
                return Ok(self.inner().nchunks as usize);
            }
            let nbytes = mem::size_of::<T>() * data.len();
            let typesize = self.typesize();
            if nbytes % self.typesize() != 0 {
                let msg = format!("Buffer ({nbytes}) not evenly divisible by typesize: {typesize}");
                return Err(Error::Other(msg));
            }
            let nchunks = unsafe {
                ffi::blosc2_schunk_append_buffer(*self.0.read(), data.as_ptr() as _, nbytes as _)
            };
            if nchunks < 0 {
                return Err(Blosc2Error::from(nchunks as i32).into());
            }
            Ok(nchunks as _)
        }

        /// Decompress a chunk, returning number of elements of `T` written to output buffer
        #[inline]
        pub fn decompress_chunk<T>(&mut self, nchunk: usize, dst: &mut [T]) -> Result<usize> {
            let chunk = Chunk::from_schunk(self, nchunk)?;
            let info = chunk.info()?;
            if dst.len() * mem::size_of::<T>() < info.nbytes as usize {
                let msg = format!(
                    "Not large enough, need {} bytes but got buffer w/ {} bytes of storage",
                    info.nbytes,
                    dst.len() * mem::size_of::<T>()
                );
                return Err(msg.into());
            }

            let ptr = dst.as_mut_ptr() as _;
            let size = unsafe {
                ffi::blosc2_schunk_decompress_chunk(
                    *self.0.read(),
                    nchunk as _,
                    ptr,
                    info.nbytes as _,
                )
            };

            if size < 0 {
                return Err(Blosc2Error::from(size).into());
            } else if size == 0 {
                let msg = format!("Non-initialized error decompressing chunk '{}'", nchunk);
                return Err(msg.into());
            } else {
                Ok((size / mem::size_of::<T>() as i32) as _)
            }
        }

        #[inline]
        pub fn decompress_chunk_vec<T>(&mut self, nchunk: usize) -> Result<Vec<T>> {
            let chunk = Chunk::from_schunk(self, nchunk)?;
            chunk.decompress()
        }

        /// Set slice buffer
        pub fn set_slice_buffer(&self, start: usize, stop: usize, buf: &[u8]) -> Result<()> {
            if stop > self.len() {
                return Err(Error::from(format!(
                    "`stop`: {} must be less than len: {}",
                    stop,
                    self.len(),
                )));
            }

            if buf.len() % self.typesize() != 0 {
                return Err(Error::from(
                    "Buffer is not evenly divisible by SChunk typesize",
                ));
            }

            let rc = unsafe {
                ffi::blosc2_schunk_set_slice_buffer(
                    *self.0.write(),
                    start as _,
                    stop as _,
                    buf.as_ptr() as *const _ as *mut _,
                )
            };
            if rc != 0 {
                return Err(Blosc2Error::from(rc).into());
            }
            Ok(())
        }

        /// Get uncompressed slice of data from start until stop. Returned as bytes, which
        /// can be transmuted/casted into the concrete item type.
        /// start/stop is by items, not by bytes
        pub fn get_slice_buffer(&self, start: usize, stop: usize) -> Result<Vec<u8>> {
            if stop > self.len() {
                return Err(Error::from(format!(
                    "Out of bounds. `stop`={}, is more than length={}",
                    stop,
                    self.len()
                )));
            }
            if stop <= start {
                return Err(Error::from("start must be less than stop"));
            }
            let nbytes = (stop - start) * self.typesize();
            let mut buf = vec![0u8; nbytes];
            let rc = unsafe {
                ffi::blosc2_schunk_get_slice_buffer(
                    *self.0.read(),
                    start as _,
                    stop as _,
                    buf.as_mut_ptr() as _,
                )
            };
            if rc != 0 {
                return Err(Blosc2Error::from(rc).into());
            }
            Ok(buf)
        }

        /// Convenience method to `get_slice_buffer` which will transmute resulting bytes buffer into `Vec<T>` for you.
        /// **NB** This will check T is same size as schunk's typesize so is _fairly_ safe.
        pub fn get_slice_buffer_as_type<T>(&self, start: usize, stop: usize) -> Result<Vec<T>> {
            if mem::size_of::<T>() != self.typesize() {
                return Err(Error::from("Size of T does not match schunk typesize"));
            }
            let buf = self.get_slice_buffer(start, stop)?;
            Ok(unsafe { mem::transmute(buf) })
        }

        pub fn get_chunk(&self, nchunk: usize) -> Result<Chunk> {
            let mut needs_free = true;
            let mut chunk = std::ptr::null_mut();
            let nbytes = unsafe {
                ffi::blosc2_schunk_get_chunk(
                    *self.0.read(),
                    nchunk as _,
                    &mut chunk as _,
                    &mut needs_free,
                )
            };
            if nbytes < 0 {
                Err(Blosc2Error::from(nbytes).into())
            } else {
                Ok(Chunk::new(chunk, needs_free))
            }
        }

        /// Export this `SChunk` into a buffer
        pub fn into_vec(self) -> Result<Vec<u8>> {
            if self.is_empty() {
                return Ok(vec![]);
            }

            unsafe { ffi::blosc2_schunk_avoid_cframe_free(*self.0.read(), true) };

            let mut needs_free = true;
            let mut ptr: *mut u8 = std::ptr::null_mut();
            let len =
                unsafe { ffi::blosc2_schunk_to_buffer(*self.0.read(), &mut ptr, &mut needs_free) };
            if len < 0 {
                return Err(Blosc2Error::from(len as i32).into());
            }

            let mut buf = unsafe { Vec::from_raw_parts(ptr, len as _, len as _) };
            if !needs_free {
                buf = buf.clone(); // Clone into new since blosc is about to free this one
            }
            Ok(buf)
        }

        /// Create a Schunk from an owned `Vec<u8>`. Data will be owned by the Schunk and released
        /// via normal Rust semantics.
        pub fn from_vec(buf: Vec<u8>) -> Result<Self> {
            let mut buf = buf;
            let schunk =
                unsafe { ffi::blosc2_schunk_from_buffer(buf.as_mut_ptr(), buf.len() as _, false) };
            if schunk.is_null() {
                return Err(Error::from(
                    "Failed to get schunk from buffer; might not be valid buffer for schunk",
                ));
            }
            unsafe { ffi::blosc2_schunk_avoid_cframe_free(schunk, true) };
            mem::forget(buf); // blosc2
            Ok(Self(Arc::new(RwLock::new(schunk))))
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

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }

    impl Drop for SChunk {
        fn drop(&mut self) {
            // drop if this is the only reference to pointer
            if Arc::strong_count(&self.0) == 1 && !(*self.0.read()).is_null() {
                unsafe { ffi::blosc2_schunk_free(*self.0.write()) };
            }
        }
    }

    impl io::Write for SChunk {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.append_buffer(buf)?;
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
                let chunk = Chunk::from_schunk(self.schunk, self.nchunk)?;
                let nbytes = chunk.info()?.nbytes();

                if nbytes <= buf.len() {
                    self.schunk.decompress_chunk(self.nchunk, buf)?;
                    self.nchunk += 1;
                    return Ok(nbytes);
                } else {
                    self.buf.get_mut().resize(nbytes as _, 0u8);
                    let nbytes_written: usize = self
                        .schunk
                        .decompress_chunk(self.nchunk, self.buf.get_mut().as_mut_slice())?;

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
/// let values = vec![0u8, 1, 2, 3];
/// let cparams = CParams::new::<u8>()
///     .set_nthreads(2);  // Optionally adjust default values
/// ```
/// [blosc2_cparams]: blosc2_sys::blosc2_cparams
pub struct CParams(ffi::blosc2_cparams);

impl CParams {
    pub fn new<T>() -> Self {
        Self::default().set_typesize::<T>()
    }
    pub fn from_typesize(typesize: usize) -> Self {
        let mut cparams = Self::default();
        cparams.0.typesize = typesize as _;
        cparams
    }
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
    pub fn set_nthreads(mut self, n: usize) -> Self {
        self.0.nthreads = n as _;
        self
    }
    /// Get number of threads
    pub fn get_nthreads(&self) -> usize {
        self.0.nthreads as _
    }
    /// Set the type size
    pub fn set_typesize<T>(mut self) -> Self {
        self.0.typesize = mem::size_of::<T>() as _;
        self
    }
    pub fn get_typesize(&self) -> usize {
        self.0.typesize as _
    }
}

impl Default for CParams {
    #[inline]
    fn default() -> Self {
        let cparams = unsafe { ffi::blosc2_get_blosc2_cparams_defaults() };
        Self(cparams)
    }
}

/// Create CParams from a reference to the type being compressed
impl<T> From<&T> for CParams {
    fn from(_: &T) -> Self {
        let mut cparams = CParams::default();
        cparams.0.typesize = mem::size_of::<T>() as _;
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
///     .set_nthreads(2);  // Optionally adjust default values
/// ```
///
/// [blosc2_dparams]: blosc2_sys::blosc2_dparams
pub struct DParams(pub(crate) ffi::blosc2_dparams);

impl DParams {
    /// Set number of theads for decompression, defaults to 1
    pub fn set_nthreads(mut self, n: usize) -> Self {
        self.0.nthreads = n as _;
        self
    }
    pub fn get_nthreads(&self) -> usize {
        self.0.nthreads as _
    }
}

impl Default for DParams {
    #[inline]
    fn default() -> Self {
        let dparams = unsafe { ffi::blosc2_get_blosc2_dparams_defaults() };
        Self(dparams)
    }
}

/// Wrapper to [blosc2_context].  
/// Container struct for de/compression ops requiring context when used in multithreaded environments
///
/// [blosc2_context]: blosc2_sys::blosc2_context
#[derive(Clone)]
pub struct Context(pub(crate) *mut ffi::blosc2_context);

impl Context {
    /// Get the CParams from this context
    ///
    /// Example
    /// -------
    /// ```
    /// use blosc2::{Context, CParams};
    ///
    /// let ctx = Context::from(CParams::default().set_typesize::<i64>());
    /// assert_eq!(ctx.get_cparams().unwrap().get_typesize(), 8);
    /// ```
    pub fn get_cparams(&self) -> Result<CParams> {
        if self.0.is_null() {
            return Err("Context pointer is null".into());
        }
        let mut cparams = CParams::default();
        let rc = unsafe { ffi::blosc2_ctx_get_cparams(self.0, &mut cparams.0) };
        if rc < 0 {
            return Err(Blosc2Error::from(rc).into());
        }
        Ok(cparams)
    }
    /// Get the DParams from this context
    ///
    /// Example
    /// -------
    /// ```
    /// use blosc2::{Context, DParams};
    ///
    /// let ctx = Context::from(DParams::default().set_nthreads(12));
    /// assert_eq!(ctx.get_dparams().unwrap().get_nthreads(), 12);
    /// ```
    pub fn get_dparams(&self) -> Result<DParams> {
        if self.0.is_null() {
            return Err("Context pointer is null".into());
        }
        let mut dparams = DParams::default();
        let rc = unsafe { ffi::blosc2_ctx_get_dparams(self.0, &mut dparams.0) };
        if rc < 0 {
            return Err(Blosc2Error::from(rc).into());
        }
        Ok(dparams)
    }
}

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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    type Error = Error;

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
impl TryFrom<*const c_void> for CompressedBufferInfo {
    type Error = Error;

    #[inline]
    fn try_from(ptr: *const c_void) -> Result<Self> {
        let mut nbytes = 0i32;
        let mut cbytes = 0i32;
        let mut blocksize = 0i32;
        let code = unsafe {
            ffi::blosc2_cbuffer_sizes(
                ptr,
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

/// Retrieve a number of elements from a `Chunk`
///
/// Example
/// -------
/// ```
/// use blosc2::{getitems, compress};
///
/// let chunk = compress(&vec![0u32, 1, 2, 3, 4], None, None, None, None).unwrap();
///
/// let offset = 1;
/// let n_items = 2;
/// let items = getitems::<u32>(&chunk, offset, n_items).unwrap();
/// assert_eq!(items, vec![1u32, 2]);
/// ```
#[inline]
pub fn getitems<T>(src: &[u8], offset: usize, n_items: usize) -> Result<Vec<T>> {
    let mut dst = Vec::with_capacity(n_items);
    let nbytes = unsafe {
        ffi::blosc2_getitem(
            src.as_ptr() as *const c_void,
            src.len() as _,
            offset as _,
            n_items as _,
            dst.as_mut_ptr() as *mut c_void,
            (n_items * mem::size_of::<T>()) as i32,
        )
    };
    if nbytes < 0 {
        return Err(Blosc2Error::from(nbytes).into());
    }
    unsafe { dst.set_len(nbytes as usize / mem::size_of::<T>()) };
    Ok(dst)
}

/// Get a list of supported compressors in this build
#[inline]
pub fn list_compressors() -> Result<Vec<Codec>> {
    let names = unsafe {
        let ptr = ffi::blosc2_list_compressors();
        CStr::from_ptr(ptr)
            .to_str()
            .map(ToString::to_string)
            .map_err(|e| Error::Other(e.to_string()))
    }?;

    let mut compressors = vec![];
    for name in names.split(',') {
        compressors.push(Codec::try_from(name)?);
    }
    Ok(compressors)
}

/// Context interface to compression, does not require call to init/destroy. For
/// use in multithreaded applications
#[inline]
pub fn compress_ctx<T>(src: &[T], ctx: &mut Context) -> Result<Vec<u8>> {
    if src.is_empty() {
        return Ok(vec![]);
    }
    let mut dst =
        vec![0u8; max_compress_len(src, ctx.get_cparams().ok().map(|c| c.get_typesize()))];
    let size = compress_into_ctx(src, &mut dst, ctx)?;
    if dst.len() > size {
        dst.truncate(size as _);
    }
    Ok(dst)
}

#[inline]
pub fn compress_into_ctx<T>(src: &[T], dst: &mut [u8], ctx: &mut Context) -> Result<usize> {
    if src.is_empty() {
        return Ok(0);
    }
    let size = unsafe {
        ffi::blosc2_compress_ctx(
            ctx.0,
            src.as_ptr() as *const c_void,
            (src.len() * ctx.get_cparams()?.get_typesize()) as _,
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
pub fn max_compress_len<T>(src: &[T], typesize: Option<usize>) -> usize {
    (src.len() * typesize.unwrap_or_else(|| mem::size_of::<T>()))
        + ffi::BLOSC2_MAX_OVERHEAD as usize
}

pub fn max_compress_len_bytes(len_bytes: usize) -> usize {
    len_bytes + ffi::BLOSC2_MAX_OVERHEAD as usize
}

#[inline]
pub fn compress<T: 'static>(
    src: &[T],
    typesize: Option<usize>,
    clevel: Option<CLevel>,
    filter: Option<Filter>,
    codec: Option<Codec>,
) -> Result<Vec<u8>> {
    if src.is_empty() {
        return Ok(vec![]);
    }
    let mut dst = Vec::with_capacity(max_compress_len(src, typesize));
    let typesize = typesize.unwrap_or_else(|| mem::size_of::<T>());
    set_compressor(codec.unwrap_or_default())?;

    // If input is bytes, but typesize is >1 then we use src len directly
    // since blosc2_compress want's the length in bytes
    let multiplier = (&src[0] as &dyn std::any::Any)
        .downcast_ref::<u8>()
        .map(|_| 1)
        .unwrap_or(typesize);

    let n_bytes = unsafe {
        ffi::blosc2_compress(
            clevel.unwrap_or_default() as _,
            filter.unwrap_or_default() as _,
            typesize as _,
            src.as_ptr() as *const c_void,
            (src.len() * multiplier) as _,
            dst.as_mut_ptr() as *mut c_void,
            dst.capacity() as _,
        )
    };
    if n_bytes < 0 {
        return Err(Blosc2Error::from(n_bytes).into());
    } else if n_bytes == 0 {
        return Err("Data is not compressable.".into());
    }
    unsafe { dst.set_len(n_bytes as _) };
    Ok(dst)
}

#[inline]
pub fn compress_into<T: 'static>(
    src: &[T],
    dst: &mut [u8],
    typesize: Option<usize>,
    clevel: Option<CLevel>,
    filter: Option<Filter>,
    codec: Option<Codec>,
) -> Result<usize> {
    if src.is_empty() {
        return Ok(0);
    }
    let typesize = typesize.unwrap_or_else(|| mem::size_of::<T>());
    set_compressor(codec.unwrap_or_default())?;

    // If input is bytes, but typesize is >1 then we use src len directly
    // since blosc2_compress want's the length in bytes
    let multiplier = (&src[0] as &dyn std::any::Any)
        .downcast_ref::<u8>()
        .map(|_| 1)
        .unwrap_or(typesize);

    let n_bytes = unsafe {
        ffi::blosc2_compress(
            clevel.unwrap_or_default() as _,
            filter.unwrap_or_default() as _,
            typesize as _,
            src.as_ptr() as *const c_void,
            (src.len() * multiplier) as _,
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
    let n_elements = info.nbytes as usize / mem::size_of::<T>();
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

/// Get the number of elements `T` in the compressed chunk
#[inline]
pub fn len<T>(src: &[u8]) -> Result<usize> {
    let info = CompressedBufferInfo::try_from(src)?;
    Ok(info.nbytes() / mem::size_of::<T>())
}

#[inline]
pub fn decompress<T>(src: &[u8]) -> Result<Vec<T>> {
    if src.is_empty() {
        return Ok(vec![]);
    }

    // blosc2 plays by bytes, we'll go by however many bytes per element
    // to set the vec length in actual elements
    let info = CompressedBufferInfo::try_from(src)?;
    let n_elements = info.nbytes as usize / mem::size_of::<T>();
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
    if src.is_empty() {
        return Ok(0);
    }
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
    let codec_name = codec.to_name_cstring()?;
    let rc = unsafe { ffi::blosc1_set_compressor(codec_name.as_ptr()) };
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

/// Free possible memory temporaries and thread resources. Use this
/// when you are not going to use Blosc for a long while.
/// A 0 if succeeds, in case of problems releasing the resources,
/// it returns a negative number.
pub fn free_resources() -> Result<()> {
    let ret = unsafe { ffi::blosc2_free_resources() };
    if ret != 0 {
        Err(Error::Blosc2(Blosc2Error::from(ret)))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ctor::{ctor, dtor};
    use std::io::Cursor;

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
        let compressed = compress(input, None, None, None, None)?;
        let decompressed = decompress_ctx(&compressed, &mut Context::from(DParams::default()))?;
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_decompress_into_ctx() -> Result<()> {
        let input = b"some data";
        let compressed = compress(input, None, None, None, None)?;
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
        let compressed = compress(input, None, None, None, None)?;
        let decompressed = decompress(&compressed)?;
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_basic_roundtrip_into() -> Result<()> {
        let input = b"some data";
        let mut compressed = vec![0u8; 100];
        let n_bytes = compress_into(input, &mut compressed, None, None, None, None)?;

        let mut decompressed = vec![0u8; input.len()];
        let n_out = decompress_into(&compressed[..n_bytes], &mut decompressed)?;

        assert_eq!(n_out, input.len());
        assert_eq!(input, decompressed.as_slice());
        Ok(())
    }

    #[test]
    fn test_schunk_basic() -> Result<()> {
        let input = b"some data";
        let storage = schunk::Storage::default()
            .set_contiguous(true)
            .set_cparams(CParams::from(&input[0]))
            .set_dparams(DParams::default());
        let mut schunk = schunk::SChunk::new(storage);

        assert!(schunk.is_contiguous());
        assert_eq!(schunk.typesize(), 1);
        assert!(schunk.path().is_none());

        let mut decompressed = vec![0u8; input.len()];

        let n = schunk.append_buffer(input)?;
        schunk.decompress_chunk(n - 1, &mut decompressed)?;
        assert_eq!(input, decompressed.as_slice());

        {
            // ensure clone then drop doesn't free the schunk ptr, original still needs is
            let _cloned = schunk.clone();
        }
        assert_eq!(schunk.n_chunks(), 1);

        // Reconstruct thru vec
        let v = schunk.into_vec()?;
        schunk = schunk::SChunk::from_vec(v)?;
        assert_eq!(schunk.n_chunks(), 1);

        Ok(())
    }

    #[test]
    fn test_schunk_thread_shared() -> Result<()> {
        let input = b"some data";
        let storage = schunk::Storage::default()
            .set_contiguous(true)
            .set_cparams(CParams::from(&input[0]))
            .set_dparams(DParams::default());
        let mut schunk = schunk::SChunk::new(storage);

        schunk.append_buffer(input)?;

        let mut schunk2 = schunk.clone();
        std::thread::spawn(move || {
            assert_eq!(schunk2.n_chunks(), 1);
            schunk2.append_buffer(b"more data").unwrap();
        })
        .join()
        .unwrap();

        assert_eq!(schunk.n_chunks(), 2);
        assert_eq!(
            b"some data",
            schunk.decompress_chunk_vec(0).unwrap().as_slice()
        );
        assert_eq!(
            b"more data",
            schunk.decompress_chunk_vec(1).unwrap().as_slice()
        );

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_schunk_write() -> Result<()> {
        let input = std::iter::repeat(b"some data")
            .take(BUFSIZE)
            .flat_map(|v| v.to_vec())
            .collect::<Vec<u8>>();
        let storage = schunk::Storage::default()
            .set_contiguous(true)
            .set_cparams(CParams::from(&input[0]))
            .set_dparams(DParams::default());
        let mut schunk = schunk::SChunk::new(storage);

        let nbytes = std::io::copy(&mut Cursor::new(input.clone()), &mut schunk)
            .map_err(|e| Error::Other(e.to_string()))?;
        assert_eq!(nbytes as usize, input.len());

        let ratio = schunk.compression_ratio();
        assert!(84. < ratio);
        assert!(86. > ratio);

        let mut uncompressed = vec![];
        let mut decoder = schunk::SChunkDecoder::new(&mut schunk);
        let n = std::io::copy(&mut decoder, &mut uncompressed).unwrap();
        assert_eq!(input, uncompressed.as_slice());
        assert_eq!(n as usize, input.len());

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_get_version_string() -> Result<()> {
        let version = get_version_string()?;
        assert_eq!(&version, "2.14.5.dev");
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_get_complib_version_string() -> Result<()> {
        let info = get_complib_info(Codec::BloscLz)?;
        assert_eq!(&info, "BloscLZ: 2.5.3");
        Ok(())
    }

    #[test]
    fn test_list_compressors() {
        let compressors = list_compressors().unwrap();
        for compressor in &[
            Codec::BloscLz,
            Codec::LZ4,
            Codec::LZ4HC,
            Codec::ZLIB,
            Codec::ZSTD,
        ] {
            assert!(compressors.contains(compressor));
        }
    }
}
