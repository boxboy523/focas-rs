use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let lib_dir = PathBuf::from(&manifest_dir).join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    if target_os == "windows" {
        println!("cargo:rustc-link-search=native={}/lib", manifest_dir);
        println!("cargo:rustc-link-lib=static=Fwlib32"); // 또는 Fwlib64
    } else if target_os == "linux" {
        println!("cargo:rustc-link-lib=dylib=fwlib32");
    }

    let header_path = if target_os == "windows" {
        PathBuf::from(&manifest_dir).join("include/Fwlib32.h")
    } else {
        PathBuf::from(&manifest_dir).join("include/fwlib32.h")
    };
    println!("cargo:rerun-if-changed={}", header_path.display());

    let mut builder = bindgen::Builder::default()
        .header(header_path.display().to_string())
        .derive_debug(true)
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    if target_os == "windows" {
        let mut clang_args = Vec::new();

        if let Ok(xwin_cache) = env::var("XWIN_CACHE_DIR") {
            let sdk_path = PathBuf::from(&xwin_cache).join("xwin/sdk/include");
            let crt_path = PathBuf::from(&xwin_cache).join("xwin/crt/include");

            clang_args.push(format!("-I{}", sdk_path.join("shared").display()));
            clang_args.push(format!("-I{}", sdk_path.join("um").display()));
            clang_args.push(format!("-I{}", sdk_path.join("ucrt").display()));
            clang_args.push(format!("-I{}", crt_path.display()));
        }
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
