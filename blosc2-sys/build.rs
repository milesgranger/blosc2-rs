use std::path::PathBuf;

fn main() {
    let lib = cmake::Config::new("c-blosc2")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("CMAKE_C_FLAGS", "-fPIE")
        .define("BLOSC_INSTALL", "ON")
        .define("BUILD_TESTS", "OFF")
        .define("BUILD_EXAMPLES", "OFF")
        .define("BUILD_BENCHMARKS", "OFF")
        .define("BUILD_FUZZERS", "OFF")
        .define("STATIC_LIB", "ON")
        .define("SHARED_LIB", "ON")
        .build();

    println!("cargo:rustc-link-search={}/lib64", lib.display());
    println!("cargo:rustc-link-search={}/lib", lib.display());
    println!("cargo:rustc-link-lib=static=blosc2");

    let out = PathBuf::from(&(format!("{}/src/ffi.rs", env!("CARGO_MANIFEST_DIR"))));
    bindgen::Builder::default()
        .header(&format!("{}/include/blosc2.h", lib.display()))
        .layout_tests(false)
        .no_default("tagMONITORINFOEXA") // Windows specific, no default [u8;40usize]
        .opaque_type("_IMAGE_TLS_DIRECTORY64") // Windows specific, error[E0588]: packed type cannot transitively contain a #[repr(align)] type
        .derive_default(true)
        .derive_copy(true)
        .derive_debug(true)
        .generate()
        .unwrap()
        .write_to_file(out)
        .unwrap();
}
