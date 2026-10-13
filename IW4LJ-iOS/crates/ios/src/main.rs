//! The iOS executable. UIKit's run loop is started by the windowing library
//! from inside the runtime, so this only has to call in.

#[cfg(target_os = "ios")]
fn main() {
    iw4l_ios::run();
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("iw4lj is the iOS build; build it with --target aarch64-apple-ios");
    std::process::exit(2);
}
