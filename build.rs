fn main() {
    #[cfg(feature = "static")]
    println!("cargo:rustc-link-lib=static=blosc2");

    #[cfg(feature = "shared")]
    println!("cargo:rustc-link-lib=blosc2");
}
