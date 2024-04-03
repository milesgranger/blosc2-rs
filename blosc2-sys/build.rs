use std::path::PathBuf;

#[cfg(feature = "use-system-blosc2")]
const BLOSC2_VERSION: &'static str = "2.14.0";

fn main() {
    let header = {
        // build blosc2 from source
        #[cfg(not(feature = "use-system-blosc2"))]
        {
            let lib = cmake::Config::new("c-blosc2")
                .define("BLOSC_INSTALL", "ON")
                .define("CMAKE_C_FLAGS", "-fPIE")
                .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
                .define("STATIC_LIB", "ON")
                .define("SHARED_LIB", "OFF")
                .define("BUILD_TESTS", "OFF")
                .define("BUILD_EXAMPLES", "OFF")
                .define("BUILD_SHARED_LIBS", "OFF")
                .define("BUILD_BENCHMARKS", "OFF")
                .define("BUILD_FUZZERS", "OFF")
                // .define("BUILD_PLUGINS", "OFF")
                .pic(true)
                .always_configure(true)
                .build();

            println!("cargo:rustc-link-search=native={}/lib64", lib.display());
            println!("cargo:rustc-link-search=native={}/lib", lib.display());
            println!("cargo:rustc-link-lib=static=blosc2");
            format!("{}/include/blosc2.h", lib.display())
        }

        // Use system blosc2
        #[cfg(feature = "use-system-blosc2")]
        {
            let lib = pkg_config::Config::new()
                .exactly_version(BLOSC2_VERSION)
                .probe("blosc2")
                .unwrap();
            for linkpath in lib.link_paths {
                println!("cargo:rustc-link-search={}", linkpath.display());
            }
            println!("cargo:rustc-link-lib=blosc2");
            format!("{}/blosc2.h", lib.include_paths[0].display())
        }
    };

    let out = PathBuf::from(&(format!("{}/bindings.rs", std::env::var("OUT_DIR").unwrap())));
    bindgen::Builder::default()
        .header(header)
        .layout_tests(false)
        .no_default("tagMONITORINFOEXA") // Windows specific, no default [u8;40usize]
        .opaque_type("_IMAGE_TLS_DIRECTORY64") // Windows specific, error[E0588]: packed type cannot transitively contain a #[repr(align)] type
        .opaque_type("__cpu_model")
        .derive_default(true)
        .derive_copy(true)
        .derive_debug(true)
        .generate()
        .unwrap()
        .write_to_file(out)
        .unwrap();
}
