use std::path::Path;

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

        if cfg!(target_feature = "sse2") {
            cmake_conf.define("SHUFFLE_SSE2_ENABLED", "1");
            if cfg!(target_env = "msvc") {
                if cfg!(target_pointer_width = "32") {
                    cmake_conf.cflag("/arch:SSE2");
                }
            } else if cfg!(target_arch = "x86_64")
                || cfg!(target_arch = "x86")
                || cfg!(target_arch = "i686")
            {
                cmake_conf.cflag("-msse2");
            }
        }

        if cfg!(target_feature = "avx2") {
            cmake_conf.define("SHUFFLE_AVX2_ENABLED", "1");
            if cfg!(target_env = "msvc") {
                cmake_conf.cflag("/arch:AVX2");
            } else if cfg!(target_arch = "x86_64")
                || cfg!(target_arch = "x86")
                || cfg!(target_arch = "i686")
            {
                cmake_conf.cflag("-mavx2");
            }
        }

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
                    .exactly_version("2.14.3")
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

    #[cfg(feature = "shared")]
    println!("cargo:rustc-link-lib=blosc2");

    #[cfg(feature = "regenerate-bindings")]
    {
        let out = PathBuf::from(&(format!("{}/bindings.rs", std::env::var("OUT_DIR").unwrap())));
        bindgen::Builder::default()
            // The input header we would like to generate
            // bindings for.
            .header("c-blosc2/include/blosc2.h")
            // Tell cargo to invalidate the built crate whenever any of the
            // included header files changed.
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .blocklist_type("__uint64_t_")
            .blocklist_type("__size_t")
            .blocklist_item("BLOSC2_[C|D]PARAMS_DEFAULTS")
            .allowlist_type(".*BLOSC.*")
            .allowlist_type(".*blosc2.*")
            .allowlist_function(".*blosc.*")
            .allowlist_var(".*BLOSC.*")
            // Replaced by libc::FILE
            .blocklist_type("FILE")
            .blocklist_type("_IO_FILE")
            .blocklist_type("_IO_codecvt")
            .blocklist_type("_IO_wide_data")
            .blocklist_type("_IO_marker")
            .blocklist_type("_IO_lock_t")
            // Replaced by i64
            .blocklist_type("LARGE_INTEGER")
            // Replaced by libc::timespec
            .blocklist_type("timespec")
            // etc
            .blocklist_type("__time_t")
            .blocklist_type("__syscall_slong_t")
            .blocklist_type("__off64_t")
            .blocklist_type("__off_t")
            .size_t_is_usize(true)
            .no_default("blosc2_[c|d]params")
            .generate()
            .expect("Unable to generate bindings")
            .write_to_file(out)
            .unwrap();
    }
}
