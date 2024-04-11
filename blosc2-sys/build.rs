use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    // build blosc2 from source, use to use cmake crate to mimic c-blosc2 builds
    // but this new bit is inspired from maiteko/blosc2-src-rs after a terrible experience w/ cmake
    #[cfg(not(feature = "use-system-blosc2"))]
    {
        let cblosc2_str = format!("{}/c-blosc2", env!("CARGO_MANIFEST_DIR"));
        let cblosc2 = Path::new(&cblosc2_str);
        let complibs = cblosc2.join("internal-complibs");
        let lz4 = complibs.join("lz4-1.9.4");
        let _zstd = complibs.join("zstd-1.5.5");

        let mut build = cc::Build::new();
        build
            // these flags don't do anything
            // xref: https://github.com/rust-lang/cc-rs/issues/594
            .shared_flag(true)
            .static_flag(true)
            .flag("-fPIC")
            .include("c-blosc2/include")
            .files(files(&cblosc2.join("blosc")))
            .include(&lz4)
            .files(files(&lz4))
            .define("HAVE_LZ4", None);
        // TODO: zstd fails w/ "hidden symbol `HUF_decompress4X2_usingDTable_internal_fast_asm_loop' isn't defined"
        // .include(&zstd)
        // .files(files(&zstd.join("dictBuilder")))
        // .files(files(&zstd.join("common")))
        // .files(files(&zstd.join("compress")))
        // .files(files(&zstd.join("decompress")))
        // .define("HAVE_ZSTD", None);

        if cfg!(target_feature = "sse2") {
            build.define("SHUFFLE_SSE2_ENABLED", "1");
            if cfg!(target_env = "msvc") {
                if cfg!(target_pointer_width = "32") {
                    build.flag("/arch:SSE2");
                }
            } else {
                build.flag("-msse2");
            }
        }

        if cfg!(target_feature = "avx2") {
            build.define("SHUFFLE_AVX2_ENABLED", "1");
            if cfg!(target_env = "msvc") {
                build.flag("/arch:AVX2");
            } else {
                build.flag("-mavx2");
            }
        }

        build.compile("blosc2");
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

fn files(dir: &PathBuf) -> impl Iterator<Item = PathBuf> {
    fs::read_dir(dir)
        .expect(&format!("Not a directory: {:?}", dir))
        .filter_map(|entry| entry.map(|entry| entry.path()).ok())
        .filter(|path| match path.extension() {
            Some(ext) => ext == "c" || ext == "cpp" || (cfg!(target_env = "msvc") && ext == "S"),
            None => false,
        })
}
