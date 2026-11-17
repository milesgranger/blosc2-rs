#![allow(
    non_upper_case_globals,
    unused_imports,
    non_snake_case,
    improper_ctypes,
    non_camel_case_types
)]

pub use libc;
use libc::{timespec, FILE};

#[cfg(not(feature = "regenerate-bindings"))]
mod bindings;
#[cfg(not(feature = "regenerate-bindings"))]
pub use bindings::*;

// build.rs blocklists the Windows LARGE_INTEGER union (blosc_timestamp_t);
// it is an 8-byte, 8-aligned union, so i64 is a layout-compatible stand-in.
#[cfg(all(feature = "regenerate-bindings", target_os = "windows"))]
pub type LARGE_INTEGER = i64;

#[cfg(feature = "regenerate-bindings")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
