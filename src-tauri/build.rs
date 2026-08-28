fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        let output = std::process::Command::new("xcrun")
            .args(["--sdk", "macosx", "--show-sdk-path"])
            .output()
            .expect("xcrun is required to locate macOS private frameworks");
        let sdk = String::from_utf8(output.stdout).expect("macOS SDK path must be UTF-8");
        println!(
            "cargo:rustc-link-search=framework={}/System/Library/PrivateFrameworks",
            sdk.trim()
        );
    }
    tauri_build::build()
}
