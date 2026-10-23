fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=iw4lj.rc");
    println!("cargo::rerun-if-changed=iw4lj.ico");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo::rustc-link-arg-bins=/STACK:8388608");
    }
    // The app icon, embedded into the Windows executable. A build that cannot
    // compile resources still produces a working exe, just without the icon.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("iw4lj.rc", embed_resource::NONE).manifest_optional();
    }
}
