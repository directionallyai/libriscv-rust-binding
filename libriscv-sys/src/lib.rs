//! FFI bindings to libriscv, a fast RISC-V sandbox emulator.
//!
//! This crate provides low-level bindings to the libriscv C API.
//! For a higher-level Rust interface, consider using a wrapper crate.
//!
//! # Example
//!
//! ```no_run
//! use libriscv_sys::*;
//! use std::ptr;
//!
//! unsafe {
//!     let mut options: RISCVOptions = std::mem::zeroed();
//!     libriscv_set_defaults(&mut options);
//!
//!     // Load ELF binary...
//!     // let machine = libriscv_new(elf_data.as_ptr() as *const _, elf_data.len() as u32, &mut options);
//! }
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

/// Version of the vendored libriscv C++ library.
pub const LIBRISCV_VERSION: &str = env!("LIBRISCV_VERSION");

#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(not(feature = "bindgen"))]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_defaults() {
        unsafe {
            let mut options: RISCVOptions = std::mem::zeroed();
            libriscv_set_defaults(&mut options);

            // Check that defaults are set
            assert!(options.max_memory > 0);
            assert!(options.stack_size > 0);
            assert_eq!(options.strict_sandbox, 1); // true
            assert_eq!(options.use_memory_arena, 1);
            assert_eq!(options.use_shared_execute_segments, 1);
            assert_eq!(options.load_program, 1);
            assert_eq!(options.protect_segments, 1);
        }
    }

    #[test]
    fn test_v1_20_abi_layouts() {
        assert_eq!(LIBRISCV_VERSION, "v1.20");
        assert_eq!(std::mem::size_of::<RISCVOptions>(), 96);
        assert_eq!(std::mem::size_of::<RISCVPageAttributes>(), 28);
        assert_eq!(std::mem::size_of::<RISCVReallocResult>(), 16);
        assert_eq!(RISCV_PAGE_SIZE, 4096);
    }

    #[test]
    fn test_strerror() {
        unsafe {
            let msg = libriscv_strerror(0);
            assert!(!msg.is_null());

            let msg = libriscv_strerror(RISCV_ERROR_TYPE_MACHINE_TIMEOUT);
            assert!(!msg.is_null());
        }
    }
}
