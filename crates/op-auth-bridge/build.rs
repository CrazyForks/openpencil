//! Link the prebuilt proprietary auth library when one exists for the
//! current target; otherwise the crate compiles its stub and the account
//! UI stays hidden. Open-source checkouts therefore always build.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(op_auth_prebuilt)");
    println!("cargo:rerun-if-changed=prebuilt");

    let target = env::var("TARGET").unwrap_or_default();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let prebuilt_dir = manifest_dir.join("prebuilt").join(&target);
    // MSVC static libraries follow the `<name>.lib` convention; every
    // other target uses the Unix `lib<name>.a` archive name.
    let artifact = if target.ends_with("-pc-windows-msvc") {
        "op_auth.lib"
    } else {
        "libop_auth.a"
    };
    if !prebuilt_dir.join(artifact).is_file() {
        return;
    }

    println!("cargo:rustc-cfg=op_auth_prebuilt");
    println!("cargo:rustc-link-search=native={}", prebuilt_dir.display());
    // `-bundle`: keep the archive out of this crate's rlib and hand it to
    // the final link instead. Bundled foreign objects would otherwise be
    // fed to thin-LTO in release builds, which fails with "failed to get
    // bitcode from object file for LTO".
    println!("cargo:rustc-link-lib=static:-bundle=op_auth");

    // System libraries the static library's TLS/network stack expects.
    if target.contains("apple-darwin") {
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    } else if target.contains("windows-msvc") {
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=bcrypt");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=ntdll");
    }
}
