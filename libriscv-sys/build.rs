use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "bindgen")]
fn generate_bindings(wrapper_h: &Path, lib_dir: &Path, out_dir: &Path, libriscv_dir: &Path) {
    let bindings = bindgen::Builder::default()
        .header(wrapper_h.to_string_lossy())
        .clang_arg(format!("-I{}", lib_dir.display()))
        .clang_arg(format!("-I{}", out_dir.display()))
        .clang_arg(format!("-I{}", libriscv_dir.join("c").display()))
        .allowlist_function("libriscv_.*")
        .allowlist_type("RISCV.*")
        .allowlist_type("riscv_.*")
        .allowlist_var("RISCV_.*")
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings");
}

#[cfg(not(feature = "bindgen"))]
fn generate_bindings(_wrapper_h: &Path, _lib_dir: &Path, _out_dir: &Path, _libriscv_dir: &Path) {
    println!("cargo:rerun-if-changed=src/bindings.rs");
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let libriscv_dir = manifest_dir.join("libriscv-c");
    let lib_dir = libriscv_dir.join("lib");
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let binary_translation = env::var("CARGO_FEATURE_BINARY_TRANSLATION").is_ok();
    let compiler = cc::Build::new().cpp(true).get_compiler();
    let threaded = compiler.is_like_clang() || compiler.is_like_gnu();

    let binary_translation_define = if binary_translation {
        "#define RISCV_BINARY_TRANSLATION"
    } else {
        "/* #undef RISCV_BINARY_TRANSLATION */"
    };
    let threaded_define = if threaded {
        "#define RISCV_THREADED"
    } else {
        "/* #undef RISCV_THREADED */"
    };
    let settings_h = format!(
        r#"#ifndef LIBRISCV_SETTINGS_H
#define LIBRISCV_SETTINGS_H

#define RISCV_EXT_A
#define RISCV_EXT_C
/* #undef RISCV_EXT_V */
/* #undef RISCV_32I */
#define RISCV_64I
/* #undef RISCV_128I */
/* #undef RISCV_FCSR */
/* #undef RISCV_DEBUG */
/* #undef RISCV_EXPERIMENTAL */
#define RISCV_MEMORY_TRAPS
/* #undef RISCV_MULTIPROCESS */
{binary_translation_define}
#define RISCV_FLAT_RW_ARENA
#define RISCV_VIRTUAL_PAGING
/* #undef RISCV_ENCOMPASSING_ARENA */
{threaded_define}
/* #undef RISCV_TAILCALL_DISPATCH */
/* #undef RISCV_LIBTCC */
/* #undef RISCV_ASMJIT */

#define RISCV_VERSION_MAJOR 1
#define RISCV_VERSION_MINOR 20

#endif
"#,
    );
    fs::write(out_dir.join("libriscv_settings.h"), settings_h)
        .expect("Failed to write libriscv_settings.h");

    let mut sources = vec![
        "lib/libriscv/cpu.cpp",
        "lib/libriscv/debug.cpp",
        "lib/libriscv/decode_bytecodes.cpp",
        "lib/libriscv/decoder_cache.cpp",
        "lib/libriscv/machine.cpp",
        "lib/libriscv/machine_defaults.cpp",
        "lib/libriscv/memory.cpp",
        "lib/libriscv/memory_elf.cpp",
        "lib/libriscv/memory_mmap.cpp",
        "lib/libriscv/memory_rw.cpp",
        "lib/libriscv/native_libc.cpp",
        "lib/libriscv/native_threads.cpp",
        "lib/libriscv/posix/minimal.cpp",
        "lib/libriscv/posix/signals.cpp",
        "lib/libriscv/posix/threads.cpp",
        "lib/libriscv/posix/socket_calls.cpp",
        "lib/libriscv/serialize.cpp",
        "lib/libriscv/shared_rodata.cpp",
        "lib/libriscv/util/crc32c.cpp",
        "lib/libriscv/rv64i.cpp",
    ];

    if threaded {
        sources.extend([
            "lib/libriscv/threaded_dispatch.cpp",
            "lib/libriscv/threaded_inaccurate_dispatch.cpp",
        ]);
    } else {
        sources.push("lib/libriscv/bytecode_dispatch.cpp");
    }

    if target_os == "windows" {
        sources.push("lib/libriscv/win32/system_calls.cpp");
    } else {
        sources.push("lib/libriscv/linux/system_calls.cpp");
    }

    if binary_translation {
        sources.extend([
            "lib/libriscv/tr_api.cpp",
            "lib/libriscv/tr_emit.cpp",
            "lib/libriscv/tr_translate.cpp",
        ]);
        if target_os == "windows" && target_env == "msvc" {
            sources.push("lib/libriscv/win32/tr_msvc.cpp");
        } else {
            sources.push("lib/libriscv/tr_compiler.cpp");
        }
        if target_os == "windows" {
            sources.push("lib/libriscv/win32/dlfcn.cpp");
        }
    }

    let wrapper_cpp = r#"#include <cstdio>
#ifdef stdout
#define _LIBRISCV_SAVED_STDOUT stdout
#undef stdout
#endif
#include "libriscv.cpp"
#ifdef _LIBRISCV_SAVED_STDOUT
#define stdout _LIBRISCV_SAVED_STDOUT
#undef _LIBRISCV_SAVED_STDOUT
#endif
"#;
    let wrapper_cpp_path = out_dir.join("libriscv_wrapper.cpp");
    fs::write(&wrapper_cpp_path, wrapper_cpp).expect("Failed to write wrapper");

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++20")
        .include(&lib_dir)
        .include(&out_dir)
        .include(libriscv_dir.join("c"));

    for source in &sources {
        build.file(libriscv_dir.join(source));
    }
    build.file(&wrapper_cpp_path);

    if compiler.is_like_clang() || compiler.is_like_gnu() {
        build.flag("-Wall").flag("-Wextra");
    }
    build.compile("riscv");

    if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=Foundation");
    } else if target_os == "windows" {
        println!("cargo:rustc-link-lib=wsock32");
        println!("cargo:rustc-link-lib=ws2_32");
    }
    if binary_translation
        && (target_os == "linux" || target_os == "freebsd" || target_os == "android")
    {
        println!("cargo:rustc-link-lib=dl");
    }
    generate_bindings(
        &manifest_dir.join("wrapper.h"),
        &lib_dir,
        &out_dir,
        &libriscv_dir,
    );

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=libriscv-c/c");
    println!("cargo:rerun-if-changed=libriscv-c/lib");
}
