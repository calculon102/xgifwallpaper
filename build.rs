fn main() {
    println!("cargo:rustc-link-lib=dylib=X11");
    println!("cargo:rustc-link-lib=dylib=Xinerama");

    // For XShm
    println!("cargo:rustc-link-lib=dylib=Xext");
}
