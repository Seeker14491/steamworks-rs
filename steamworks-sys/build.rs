use std::{env, fs, path::PathBuf};

fn main() {
    let sdk_loc =
        dunce::canonicalize("steamworks_sdk").expect("The steamworks_sdk folder is missing");

    let bindings = bindgen::Builder::default()
        .header("wrapper.hpp")
        .clang_args(&[
            "-std=c++11",
            "-I",
            sdk_loc.join("public").to_str().unwrap(),
            "-Wno-deprecated-declarations",
        ])
        .generate()
        .expect("Error generating bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write the generated bindings");

    let triple = env::var("TARGET").unwrap();
    let mut lib = "steam_api";
    let (path, lib_files): (_, &[&str]) = if triple.contains("windows") {
        if triple.contains("i686") {
            (
                sdk_loc.join("redistributable_bin"),
                &["steam_api.dll", "steam_api.lib"],
            )
        } else {
            lib = "steam_api64";
            (
                sdk_loc.join("redistributable_bin/win64"),
                &["steam_api64.dll", "steam_api64.lib"],
            )
        }
    } else if triple.contains("linux") {
        if triple.contains("i686") {
            (
                sdk_loc.join("redistributable_bin/linux32"),
                &["libsteam_api.so"],
            )
        } else {
            (
                sdk_loc.join("redistributable_bin/linux64"),
                &["libsteam_api.so"],
            )
        }
    } else if triple.contains("darwin") {
        (
            sdk_loc.join("redistributable_bin/osx"),
            &["libsteam_api.dylib"],
        )
    } else {
        panic!("Unsupported OS");
    };

    for file in lib_files {
        fs::copy(path.join(file), out_path.join(file)).unwrap();
    }

    println!("cargo:rustc-link-lib=dylib={}", lib);
    println!("cargo:rustc-link-search={}", out_path.display());
}
