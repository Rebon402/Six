use std::env;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Check if zig is installed
    let zig_exists = Command::new("zig").arg("version").status().is_ok();

    if zig_exists {
        let status = Command::new("zig")
            .args(&[
                "build-lib",
                "src/native/core.zig",
                "-femit-bin",
                &format!("{}/libsix_native.a", out_dir),
                "-target",
                "x86_64-windows",
                "--cache-dir",
                &format!("{}/zig-cache", out_dir),
            ])
            .status()
            .expect("Failed to run Zig compiler");

        if status.success() {
            println!("cargo:rustc-link-lib=static=six_native");
        }
    } else {
        println!("cargo:warning=Zig compiler not found. Native Zig core will be disabled.");
    }

    // Compile C guard
    cc::Build::new()
        .file("src/native/guard.c")
        .compile("six_guard");

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=six_guard");
    println!("cargo:rerun-if-changed=src/native/core.zig");
    println!("cargo:rerun-if-changed=src/native/guard.c");
}
