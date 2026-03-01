use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let lib_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    if target_os == "windows" {
        println!("cargo:rustc-link-lib=static=Fwlib32"); // 또는 Fwlib64
    } else if target_os == "linux" {
        println!("cargo:rustc-link-lib=dylib=fwlib32");
    }

    let header_path = if target_os == "windows" {
        "include/Fwlib32.h"
    } else {
        "include/fwlib32.h"
    };
    println!("cargo:rerun-if-changed={}", header_path);

    let mut builder = bindgen::Builder::default()
        .header(header_path)
        .derive_debug(true)
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    if target_os == "windows" {
        builder = builder
            .allowlist_function("cnc_.*")
            .allowlist_function("pmc_.*")
            .allowlist_type("ODB.*")
            .allowlist_type("IODB.*")
            .allowlist_var("EW_.*")
            .allowlist_var("MAX_.*")
    } else if target_os == "linux" {
        builder = builder
            .clang_arg("-DTCHAR=char")
            .clang_arg("-D_TCHAR_DEFINED");
    }
    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from("src");
    bindings
        .write_to_file(out_path.join(format!("bindings_{}.rs", target_os)))
        .expect("Couldn't write bindings!");
}
