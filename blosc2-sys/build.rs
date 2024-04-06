use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    // build blosc2 from source
    #[cfg(not(feature = "use-system-blosc2"))]
    {
        let out_dir_str = std::env::var("OUT_DIR").unwrap();

        let install_path_str =
            std::env::var("BLOSC2_INSTALL_PREFIX").unwrap_or(out_dir_str.to_owned());
        let install_path = Path::new(&install_path_str);

        let mut cmake_conf = cmake::Config::new("c-blosc2");
        cmake_conf
            .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
            .define("BUILD_SHARED_LIBS", "ON")
            .define("BUILD_FUZZERS", "OFF")
            .define("BUILD_BENCHMARKS", "OFF")
            .define("BUILD_EXAMPLES", "OFF")
            .define("BUILD_STATIC", "ON")
            .define("BUILD_SHARED", "ON")
            .define("BUILD_TESTS", "OFF")
            .define("BUILD_PLUGINS", "OFF")
            .always_configure(true);

        if std::env::var("BLOSC2_INSTALL_PREFIX").is_ok() {
            let install_prefix = format!("{}", install_path.display());
            cmake_conf
                .define("CMAKE_INSTALL_PREFIX", install_prefix)
                .define("BLOSC_INSTALL", "ON");
        }

        cmake_conf.build();

        for subdir in &["lib64", "lib", "bin"] {
            let search_path = install_path.join(subdir);
            println!("cargo::rustc-link-search={}", search_path.display());
        }
    }

    // Use system blosc2
    #[cfg(feature = "use-system-blosc2")]
    {
        match std::env::var("BLOSC2_INSTALL_PREFIX") {
            Ok(prefix) => {
                let install_path = Path::new(&prefix);
                for subdir in &["lib64", "lib", "bin"] {
                    let search_path = install_path.join(subdir);
                    println!("cargo::rustc-link-search={}", search_path.display());
                }
            }

            // Fall back to try and locate w/ pkg-config
            // TODO: 3rd option, just assume it's discoverable in the current environment?
            Err(_) => {
                let lib = pkg_config::Config::new()
                    .exactly_version("2.14.0")
                    .probe("blosc2")
                    .unwrap();
                for linkpath in lib.link_paths {
                    println!("cargo:rustc-link-search={}", linkpath.display());
                }
            }
        }
    }

    #[cfg(feature = "static")]
    println!("cargo:rustc-link-lib=static=blosc2");

    #[cfg(not(feature = "static"))]
    println!("cargo:rustc-link-lib=blosc2");

    let out = PathBuf::from(&(format!("{}/bindings.rs", std::env::var("OUT_DIR").unwrap())));
    bindgen::Builder::default()
        .header(format!(
            "{}/c-blosc2/include/blosc2.h",
            env!("CARGO_MANIFEST_DIR")
        ))
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
