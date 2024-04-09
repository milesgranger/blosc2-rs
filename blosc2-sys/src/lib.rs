#![allow(
    non_upper_case_globals,
    unused_imports,
    non_snake_case,
    improper_ctypes,
    non_camel_case_types
)]

#[cfg(not(feature = "regenerate-bindings"))]
mod bindings;
#[cfg(not(feature = "regenerate-bindings"))]
pub use bindings::*;

#[cfg(feature = "regenerate-bindings")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
