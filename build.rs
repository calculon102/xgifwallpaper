fn main() {
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-lib=Xinerama");

    // For XShm
    println!("cargo:rustc-link-lib=Xext");
}
