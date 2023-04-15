use std::path::PathBuf;

fn main() {
    // let lib = cmake::Config::new("c-blosc2")
    //     .define("CMAKE_C_FLAGS", "-O3 -std=c99")
    //     .define("STATIC_LIB", "ON")
    //     .define("SHARED_LIB", "ON")
    //     .define("BUILD_TESTS", "OFF")
    //     .define("BUILD_EXAMPLES", "OFF")
    //     .define("BUILD_BENCHMARKS", "OFF")
    //     .define("BUILD_FUZZERS", "OFF")
    //     .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
    //     .define("BLOSC_INSTALL", "ON")
    //     .always_configure(true)
    //     .build();

    println!("cargo:rustc-link-search=/home/milesg/mambaforge/envs/cramjam/lib64");

    // println!("cargo:rustc-link-search=native={}/lib64", lib.display());
    // println!("cargo:rustc-link-search=native={}/lib", lib.display());
    println!("cargo:rustc-link-lib=static=blosc2");
    // println!(
    //     "cargo:rustc-link-search={}/c-blosc2/install/lib64",
    //     env!("CARGO_MANIFEST_DIR")
    // );

    let out = PathBuf::from(&(format!("{}/bindings.rs", std::env::var("OUT_DIR").unwrap())));
    bindgen::Builder::default()
        // .header(&format!("{}/include/blosc2.h", lib.display()))
        .header("/home/milesg/mambaforge/envs/cramjam/include/blosc2.h")
        // .header(format!(
        //     "{}/c-blosc2/install/include/blosc2.h",
        //     env!("CARGO_MANIFEST_DIR")
        // ))
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
