fn main() {
    println!("cargo:rustc-link-lib=static=blosc2");
    println!("cargo:rustc-link-lib=blosc2");
}
