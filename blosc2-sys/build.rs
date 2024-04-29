use std::path::Path;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    // build blosc2 from source
    #[cfg(not(feature = "use-system-blosc2"))]
    {
        let out_dir_str = std::env::var("OUT_DIR").unwrap();
        let out_dir = Path::new(&out_dir_str);

        // The configure step modifies the config.h.in file into c-blosc2/config.h
        // which violates the cargo publishing/build as it modifies things outside
        // of the crate's out dir; so we'll copy everything into out/c-blosc2
        let src_dir = out_dir.join("c-blosc2");
        copy_dir::copy_dir("c-blosc2", &src_dir).unwrap();

        let install_path_str =
            std::env::var("BLOSC2_INSTALL_PREFIX").unwrap_or(out_dir_str.to_owned());
        let install_path = Path::new(&install_path_str);

        let cmake_c_flags = std::env::var("CFLAGS").unwrap_or("".to_string());
        let mut cmake_conf = cmake::Config::new(src_dir);
        cmake_conf
            .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
            .define("BUILD_SHARED_LIBS", "ON")
            .define("BUILD_FUZZERS", "OFF")
            .define("BUILD_BENCHMARKS", "OFF")
            .define("BUILD_EXAMPLES", "OFF")
            .define("BUILD_STATIC", "OFF")
            .define("BUILD_SHARED", "OFF")
            .define("BUILD_TESTS", "OFF")
            .define("BUILD_PLUGINS", "OFF")
            .define("CMAKE_C_FLAGS", cmake_c_flags)
            .always_configure(true);

        if cfg!(feature = "static") {
            cmake_conf.define("BUILD_STATIC", "ON");
        }
        if cfg!(feature = "shared") {
            cmake_conf.define("BUILD_SHARED", "ON");
        }

        if std::env::var("BLOSC2_INSTALL_PREFIX").is_ok() {
            let install_prefix = format!("{}", install_path.display());
            cmake_conf
                .define("CMAKE_INSTALL_PREFIX", install_prefix)
                .define("BLOSC_INSTALL", "ON");
        }

        // Solves undefined reference to __cpu_model when using __builtin_cpu_supports() in shuffle.c
        if let Ok(true) = std::env::var("CARGO_CFG_TARGET_ENV").map(|v| v == "musl") {
            // TODO: maybe not always libgcc? I'm not sure.
            println!("cargo:rustc-link-lib=gcc");
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
