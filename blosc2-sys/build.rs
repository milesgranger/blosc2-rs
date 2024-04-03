use std::path::PathBuf;
use std::process::Command;

#[cfg(feature = "use-system-blosc2")]
const BLOSC2_VERSION: &'static str = "2.14.0";

fn main() {
    let header = {
        // build blosc2 from source
        #[cfg(not(feature = "use-system-blosc2"))]
        {
            let out_dir = std::env::var("OUT_DIR").unwrap();
            let install_path = format!("{}/{}", &out_dir, "blosc2-install");
            let build_path = format!("{}/{}", &out_dir, "blosc2-build");

            let configure_output = Command::new("cmake")
                .arg(format!("-S{}/{}", env!("CARGO_MANIFEST_DIR"), "c-blosc2"))
                .arg(format!("-B{}", &build_path))
                .arg("-DCMAKE_BUILD_TYPE=Release")
                .arg(format!("-DCMAKE_INSTALL_PREFIX={}", &install_path))
                .arg("-DCMAKE_C_FLAGS=-fPIE")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .arg("-DBUILD_SHARED_LIBS=OFF")
                .arg("-DBUILD_FUZZERS=OFF")
                .arg("-DBUILD_BENCHMARKS=OFF")
                .arg("-DBUILD_SHARED_LIBS=OFF")
                .arg("-DBUILD_EXAMPLES=OFF")
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

            println!("cargo:rustc-link-search={}/lib64", &install_path);
            println!("cargo:rustc-link-search={}/lib", &install_path);
            println!("cargo:rustc-link-lib=static=blosc2");
            format!("{}/include/blosc2.h", &install_path)
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
        .derive_default(true)
        .derive_copy(true)
        .derive_debug(true)
        .generate()
        .unwrap()
        .write_to_file(out)
        .unwrap();
}
