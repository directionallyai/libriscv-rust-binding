//! Low-level FFI bindings to the libriscv v1.20 C API.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(not(feature = "bindgen"))]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_20_option_defaults_are_initialized() {
        let mut options = std::mem::MaybeUninit::<RISCVOptions>::zeroed();
        unsafe { libriscv_set_defaults(options.as_mut_ptr()) };
        let options = unsafe { options.assume_init() };

        assert!(options.max_memory > 0);
        assert!(options.stack_size > 0);
        assert_eq!(options.strict_sandbox, 1);
        assert_eq!(options.use_memory_arena, 1);
        assert_eq!(options.use_shared_execute_segments, 1);
        assert!(options.default_exit_function.is_null());
        assert_eq!(options.load_program, 1);
        assert_eq!(options.protect_segments, 1);
        assert_eq!(options.native_syscall_base, 0);
        assert_eq!(options.arena_size, 8 << 20);
    }
}
