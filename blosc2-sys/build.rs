use std::path::{Path, PathBuf};
use std::process::Command;

const BLOSC2_VERSION: &'static str = "2.14.0";

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    // build blosc2 from source
    #[cfg(not(feature = "use-system-blosc2"))]
    {
        let out_dir_str = std::env::var("OUT_DIR").unwrap();
        let out_dir = Path::new(&out_dir_str);
        let build_path = out_dir.join("blosc2-build");

        let install_path_str =
            std::env::var("BLOSC2_INSTALL_PREFIX").unwrap_or(out_dir_str.to_owned());
        let install_path = Path::new(&install_path_str);

        let configure_output = Command::new("cmake")
            .arg(format!("-S{}/{}", env!("CARGO_MANIFEST_DIR"), "c-blosc2"))
            .arg(format!("-B{}", build_path.display()))
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install_path.display()))
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .arg("-DBUILD_SHARED_LIBS=ON")
            .arg("-DBUILD_FUZZERS=OFF")
            .arg("-DBUILD_BENCHMARKS=OFF")
            .arg("-DBUILD_EXAMPLES=OFF")
            .arg("-DBUILD_STATIC=ON")
            .arg("-DBUILD_SHARED=ON")
            .arg("-DBUILD_TESTS=OFF")
            .arg("-DBLOSC_INSTALL=ON")
            .output()
            .unwrap();
        if !configure_output.status.success() {
            panic!(
                "{}",
                std::str::from_utf8(configure_output.stdout.as_slice()).unwrap()
            );
        }
        let build_output = Command::new("cmake")
            .arg("--build")
            .arg(&build_path)
            .arg("--target")
            .arg("install")
            .output()
            .unwrap();

        if !build_output.status.success() {
            panic!(
                "{}",
                std::str::from_utf8(build_output.stderr.as_slice()).unwrap()
            );
        }

        for subdir in &["lib64", "lib", "bin"] {
            let search_path = install_path.join(subdir);
            println!("cargo::rustc-link-search={}", search_path.display());
        }
        println!("cargo::rustc-link-lib=static=blosc2");
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
    }

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
