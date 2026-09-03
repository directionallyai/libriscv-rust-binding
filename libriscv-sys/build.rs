use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "bindgen")]
fn generate_bindings(lib_dir: &Path, out_dir: &Path, libriscv_dir: &Path) {
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
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
        .expect("Couldn't write bindings!");
}

#[cfg(not(feature = "bindgen"))]
fn generate_bindings(_lib_dir: &Path, _out_dir: &Path, _libriscv_dir: &Path) {
    println!("cargo:rerun-if-changed=src/bindings.rs");
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let libriscv_dir = manifest_dir.join("libriscv-c");
    let lib_dir = libriscv_dir.join("lib");
    let libriscv_version = fs::read_to_string(libriscv_dir.join("VERSION"))
        .expect("Failed to read vendored libriscv version");
    assert_eq!(libriscv_version.trim(), "v1.20");
    println!(
        "cargo:rustc-env=LIBRISCV_VERSION={}",
        libriscv_version.trim()
    );
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let binary_translation = env::var("CARGO_FEATURE_BINARY_TRANSLATION").is_ok();

    // Generate libriscv_settings.h
    let binary_translation_define = if binary_translation {
        "#define RISCV_BINARY_TRANSLATION"
    } else {
        "/* #undef RISCV_BINARY_TRANSLATION */"
    };
    let settings_h = format!(
        r#"#ifndef LIBRISCV_SETTINGS_H
#define LIBRISCV_SETTINGS_H

/* libriscv_sys configuration */
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
#define RISCV_THREADED
/* #undef RISCV_TAILCALL_DISPATCH */
/* #undef RISCV_LIBTCC */
/* #undef RISCV_ASMJIT */

/* Version information */
#define RISCV_VERSION_MAJOR 1
#define RISCV_VERSION_MINOR 20

#endif /* LIBRISCV_SETTINGS_H */
"#,
    );
    fs::write(out_dir.join("libriscv_settings.h"), settings_h)
        .expect("Failed to write libriscv_settings.h");

    // Core source files
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
        // 64-bit support
        "lib/libriscv/rv64i.cpp",
        // Threaded dispatch (for GCC/Clang)
        "lib/libriscv/threaded_dispatch.cpp",
        "lib/libriscv/threaded_inaccurate_dispatch.cpp",
        // C API wrapper
        "c/libriscv.cpp",
    ];

    // Platform-specific system calls
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
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

    // Create a wrapper for libriscv.cpp that handles the stdout macro conflict on macOS
    // The issue is that macOS defines `stdout` as a macro, which conflicts with the
    // `stdout` field name in RISCVOptions struct
    let libriscv_cpp_path = libriscv_dir.join("c/libriscv.cpp");
    let wrapper_cpp = format!(
        r#"
// Save and undef the stdout macro before including libriscv headers
#include <cstdio>
#ifdef stdout
#define _LIBRISCV_SAVED_STDOUT stdout
#undef stdout
#endif

// Include the original implementation with absolute path
#include "{}"

// Restore stdout macro
#ifdef _LIBRISCV_SAVED_STDOUT
#define stdout _LIBRISCV_SAVED_STDOUT
#undef _LIBRISCV_SAVED_STDOUT
#endif
"#,
        libriscv_cpp_path.display()
    );
    let wrapper_cpp_path = out_dir.join("libriscv_wrapper.cpp");
    fs::write(&wrapper_cpp_path, wrapper_cpp).expect("Failed to write wrapper");

    // Remove the original C API from sources - we'll use our wrapper instead
    sources.retain(|s| *s != "c/libriscv.cpp");

    // Build the C++ library
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++20")
        .include(&lib_dir)
        .include(&out_dir)
        .include(libriscv_dir.join("c"));

    // Add all source files
    for source in &sources {
        build.file(libriscv_dir.join(source));
    }

    // Add the wrapper for the C API (handles stdout macro conflict)
    build.file(&wrapper_cpp_path);

    // Compiler-specific flags
    let compiler = build.get_compiler();
    if compiler.is_like_clang() || compiler.is_like_gnu() {
        build.flag("-Wall").flag("-Wextra");
    }

    build.compile("riscv");

    // Platform-specific linking
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

    // Link C++ standard library
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=c++");
    } else if target_os == "linux" || target_os == "freebsd" {
        println!("cargo:rustc-link-lib=stdc++");
    } else if target_os == "windows" && target_env == "gnu" {
        println!("cargo:rustc-link-lib=stdc++");
    }

    generate_bindings(&lib_dir, &out_dir, &libriscv_dir);

    // Rerun if sources change
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=libriscv-c/c/libriscv.h");
    println!("cargo:rerun-if-changed=libriscv-c/VERSION");
    for source in &sources {
        println!("cargo:rerun-if-changed=libriscv-c/{}", source);
    }
}
