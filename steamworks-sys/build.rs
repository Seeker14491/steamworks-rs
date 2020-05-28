use std::{env, fs, path::PathBuf};

fn main() {
    let sdk_loc = dunce::canonicalize("steamworks_sdk").expect("steamworks_sdk folder is missing");

    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .clang_args(&[
            "-std=c++11",
            "-I",
            sdk_loc.join("public/steam").to_str().unwrap(),
        ])
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let triple = env::var("TARGET").unwrap();
    let mut lib = "steam_api";
    let (path, runtime_dependency) = if triple.contains("windows") {
        if triple.contains("i686") {
            (sdk_loc.join("redistributable_bin"), "steam_api.dll")
        } else {
            lib = "steam_api64";
            (sdk_loc.join("redistributable_bin/win64"), "steam_api64.dll")
        }
    } else if triple.contains("linux") {
        if triple.contains("i686") {
            (
                sdk_loc.join("redistributable_bin/linux32"),
                "libsteam_api.so",
            )
        } else {
            (
                sdk_loc.join("redistributable_bin/linux64"),
                "libsteam_api.so",
            )
        }
    } else if triple.contains("darwin") {
        (
            sdk_loc.join("redistributable_bin/osx32"),
            "libsteam_api.dylib",
        )
    } else {
        panic!("Unsupported OS");
    };

    println!("cargo:rustc-link-lib=dylib={}", lib);
    println!("cargo:rustc-link-search={}", path.display());

    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-std=c++11")
        .include(sdk_loc.join("public/steam"))
        .file("src/lib.cpp")
        .compile("steamrust");

    fs::copy(
        path.join(runtime_dependency),
        out_path.join(runtime_dependency),
    )
    .unwrap();
}
