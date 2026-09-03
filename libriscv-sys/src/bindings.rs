/* automatically generated from libriscv v1.20 by rust-bindgen 0.72.1 */

pub const RISCV_PAGE_SIZE: u32 = 4096;
pub const RISCV_ERROR_TYPE_GENERAL_EXCEPTION: i32 = -1;
pub const RISCV_ERROR_TYPE_MACHINE_EXCEPTION: i32 = -2;
pub const RISCV_ERROR_TYPE_MACHINE_TIMEOUT: i32 = -3;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RISCVMachine {
    _unused: [u8; 0],
}
pub type riscv_error_func_t = ::std::option::Option<
    unsafe extern "C" fn(
        opaque: *mut ::std::os::raw::c_void,
        type_: ::std::os::raw::c_int,
        msg: *const ::std::os::raw::c_char,
        data: ::std::os::raw::c_long,
    ),
>;
pub type riscv_stdout_func_t = ::std::option::Option<
    unsafe extern "C" fn(
        opaque: *mut ::std::os::raw::c_void,
        msg: *const ::std::os::raw::c_char,
        size: ::std::os::raw::c_uint,
    ),
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RISCVOptions {
    pub max_memory: u64,
    pub stack_size: u32,
    pub strict_sandbox: ::std::os::raw::c_int,
    pub argc: ::std::os::raw::c_uint,
    pub argv: *mut *const ::std::os::raw::c_char,
    pub error: riscv_error_func_t,
    pub stdout: riscv_stdout_func_t,
    pub opaque: *mut ::std::os::raw::c_void,
    pub use_memory_arena: ::std::os::raw::c_int,
    pub use_shared_execute_segments: ::std::os::raw::c_int,
    pub default_exit_function: *const ::std::os::raw::c_char,
    pub load_program: ::std::os::raw::c_int,
    pub protect_segments: ::std::os::raw::c_int,
    pub native_syscall_base: ::std::os::raw::c_uint,
    pub arena_size: u64,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of RISCVOptions"][::std::mem::size_of::<RISCVOptions>() - 96usize];
    ["Alignment of RISCVOptions"][::std::mem::align_of::<RISCVOptions>() - 8usize];
    ["Offset of field: RISCVOptions::max_memory"]
        [::std::mem::offset_of!(RISCVOptions, max_memory) - 0usize];
    ["Offset of field: RISCVOptions::stack_size"]
        [::std::mem::offset_of!(RISCVOptions, stack_size) - 8usize];
    ["Offset of field: RISCVOptions::strict_sandbox"]
        [::std::mem::offset_of!(RISCVOptions, strict_sandbox) - 12usize];
    ["Offset of field: RISCVOptions::argc"][::std::mem::offset_of!(RISCVOptions, argc) - 16usize];
    ["Offset of field: RISCVOptions::argv"][::std::mem::offset_of!(RISCVOptions, argv) - 24usize];
    ["Offset of field: RISCVOptions::error"][::std::mem::offset_of!(RISCVOptions, error) - 32usize];
    ["Offset of field: RISCVOptions::stdout"]
        [::std::mem::offset_of!(RISCVOptions, stdout) - 40usize];
    ["Offset of field: RISCVOptions::opaque"]
        [::std::mem::offset_of!(RISCVOptions, opaque) - 48usize];
    ["Offset of field: RISCVOptions::use_memory_arena"]
        [::std::mem::offset_of!(RISCVOptions, use_memory_arena) - 56usize];
    ["Offset of field: RISCVOptions::use_shared_execute_segments"]
        [::std::mem::offset_of!(RISCVOptions, use_shared_execute_segments) - 60usize];
    ["Offset of field: RISCVOptions::default_exit_function"]
        [::std::mem::offset_of!(RISCVOptions, default_exit_function) - 64usize];
    ["Offset of field: RISCVOptions::load_program"]
        [::std::mem::offset_of!(RISCVOptions, load_program) - 72usize];
    ["Offset of field: RISCVOptions::protect_segments"]
        [::std::mem::offset_of!(RISCVOptions, protect_segments) - 76usize];
    ["Offset of field: RISCVOptions::native_syscall_base"]
        [::std::mem::offset_of!(RISCVOptions, native_syscall_base) - 80usize];
    ["Offset of field: RISCVOptions::arena_size"]
        [::std::mem::offset_of!(RISCVOptions, arena_size) - 88usize];
};
unsafe extern "C" {
    pub fn libriscv_set_defaults(options: *mut RISCVOptions);
}
unsafe extern "C" {
    pub fn libriscv_new(
        elf_prog: *const ::std::os::raw::c_void,
        elf_size: ::std::os::raw::c_uint,
        o: *const RISCVOptions,
    ) -> *mut RISCVMachine;
}
unsafe extern "C" {
    pub fn libriscv_delete(m: *mut RISCVMachine) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_run(m: *mut RISCVMachine, instruction_limit: u64) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_step_one(m: *mut RISCVMachine, verbose: ::std::os::raw::c_int) -> i64;
}
unsafe extern "C" {
    pub fn libriscv_allow_file(m: *mut RISCVMachine, path: *const ::std::os::raw::c_char);
}
unsafe extern "C" {
    pub fn libriscv_strerror(return_value: ::std::os::raw::c_int) -> *const ::std::os::raw::c_char;
}
unsafe extern "C" {
    pub fn libriscv_return_value(m: *mut RISCVMachine) -> i64;
}
unsafe extern "C" {
    pub fn libriscv_address_of(m: *mut RISCVMachine, name: *const ::std::os::raw::c_char) -> u64;
}
unsafe extern "C" {
    pub fn libriscv_opaque(m: *mut RISCVMachine) -> *mut ::std::os::raw::c_void;
}
#[doc = " View and modify the RISC-V emulator state"]
#[repr(C)]
#[derive(Copy, Clone)]
pub union RISCVFloat {
    pub f32_: [f32; 2usize],
    pub f64_: f64,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of RISCVFloat"][::std::mem::size_of::<RISCVFloat>() - 8usize];
    ["Alignment of RISCVFloat"][::std::mem::align_of::<RISCVFloat>() - 8usize];
    ["Offset of field: RISCVFloat::f32_"][::std::mem::offset_of!(RISCVFloat, f32_) - 0usize];
    ["Offset of field: RISCVFloat::f64_"][::std::mem::offset_of!(RISCVFloat, f64_) - 0usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct RISCVRegisters {
    pub r: [u64; 32usize],
    pub pc: u64,
    pub fcsr: u32,
    pub fr: [RISCVFloat; 32usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of RISCVRegisters"][::std::mem::size_of::<RISCVRegisters>() - 528usize];
    ["Alignment of RISCVRegisters"][::std::mem::align_of::<RISCVRegisters>() - 8usize];
    ["Offset of field: RISCVRegisters::r"][::std::mem::offset_of!(RISCVRegisters, r) - 0usize];
    ["Offset of field: RISCVRegisters::pc"][::std::mem::offset_of!(RISCVRegisters, pc) - 256usize];
    ["Offset of field: RISCVRegisters::fcsr"]
        [::std::mem::offset_of!(RISCVRegisters, fcsr) - 264usize];
    ["Offset of field: RISCVRegisters::fr"][::std::mem::offset_of!(RISCVRegisters, fr) - 272usize];
};
unsafe extern "C" {
    pub fn libriscv_get_registers(m: *mut RISCVMachine) -> *mut RISCVRegisters;
}
unsafe extern "C" {
    pub fn libriscv_set_result_register(m: *mut RISCVMachine, value: i64);
}
unsafe extern "C" {
    pub fn libriscv_jump(m: *mut RISCVMachine, address: u64) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_copy_to_guest(
        m: *mut RISCVMachine,
        dst: u64,
        src: *const ::std::os::raw::c_void,
        len: ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_copy_from_guest(
        m: *mut RISCVMachine,
        dst: *mut ::std::os::raw::c_void,
        src: u64,
        len: ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_memstring(
        m: *mut RISCVMachine,
        src: u64,
        maxlen: ::std::os::raw::c_uint,
        length: *mut ::std::os::raw::c_uint,
    ) -> *const ::std::os::raw::c_char;
}
unsafe extern "C" {
    pub fn libriscv_memview(
        m: *mut RISCVMachine,
        src: u64,
        length: ::std::os::raw::c_uint,
    ) -> *const ::std::os::raw::c_char;
}
unsafe extern "C" {
    pub fn libriscv_writable_memview(
        m: *mut RISCVMachine,
        src: u64,
        length: ::std::os::raw::c_uint,
    ) -> *mut ::std::os::raw::c_char;
}
unsafe extern "C" {
    pub fn libriscv_stop(m: *mut RISCVMachine);
}
unsafe extern "C" {
    pub fn libriscv_instruction_counter(m: *mut RISCVMachine) -> u64;
}
unsafe extern "C" {
    pub fn libriscv_max_counter_pointer(m: *mut RISCVMachine) -> *mut u64;
}
unsafe extern "C" {
    pub fn libriscv_instruction_limit_reached(m: *mut RISCVMachine) -> ::std::os::raw::c_int;
}
#[doc = " RISC-V system call handling"]
pub type riscv_syscall_handler_t =
    ::std::option::Option<unsafe extern "C" fn(m: *mut RISCVMachine)>;
unsafe extern "C" {
    pub fn libriscv_set_syscall_handler(
        num: ::std::os::raw::c_uint,
        arg1: riscv_syscall_handler_t,
    ) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_trigger_exception(
        m: *mut RISCVMachine,
        exception: ::std::os::raw::c_uint,
        data: u64,
    );
}
unsafe extern "C" {
    #[doc = " RISC-V VM function calls"]
    pub fn libriscv_setup_vmcall(m: *mut RISCVMachine, address: u64) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_load_binary_file(
        filename: *const ::std::os::raw::c_char,
        data: *mut *mut ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RISCVPageAttributes {
    pub read: ::std::os::raw::c_int,
    pub write: ::std::os::raw::c_int,
    pub exec: ::std::os::raw::c_int,
    pub is_cow: ::std::os::raw::c_int,
    pub non_owning: ::std::os::raw::c_int,
    pub dont_fork: ::std::os::raw::c_int,
    pub user_defined: u8,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of RISCVPageAttributes"]
        [::std::mem::size_of::<RISCVPageAttributes>() - 28usize];
    ["Alignment of RISCVPageAttributes"]
        [::std::mem::align_of::<RISCVPageAttributes>() - 4usize];
};
unsafe extern "C" {
    pub fn libriscv_fast_fork(
        parent: *const RISCVMachine,
        opts: *mut RISCVOptions,
    ) -> *mut RISCVMachine;
}
unsafe extern "C" {
    pub fn libriscv_is_forked(m: *const RISCVMachine) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn libriscv_get_parent_page_data(
        parent: *const RISCVMachine,
        pageno: u64,
        attr_out: *mut RISCVPageAttributes,
    ) -> *const ::std::os::raw::c_void;
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RISCVReallocResult {
    pub ptr: u64,
    pub old_size: u64,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of RISCVReallocResult"][::std::mem::size_of::<RISCVReallocResult>() - 16usize];
    ["Alignment of RISCVReallocResult"]
        [::std::mem::align_of::<RISCVReallocResult>() - 8usize];
};
pub type riscv_arena_unknown_free_t = ::std::option::Option<
    unsafe extern "C" fn(ptr: u64, user: *mut ::std::os::raw::c_void) -> ::std::os::raw::c_int,
>;
pub type riscv_arena_unknown_realloc_t = ::std::option::Option<
    unsafe extern "C" fn(
        ptr: u64,
        new_size: u64,
        user: *mut ::std::os::raw::c_void,
    ) -> RISCVReallocResult,
>;
unsafe extern "C" {
    pub fn libriscv_setup_arena(
        m: *mut RISCVMachine,
        syscall_base: ::std::os::raw::c_uint,
        addr: u64,
        size: u64,
    ) -> ::std::os::raw::c_int;
    pub fn libriscv_has_arena(m: *const RISCVMachine) -> ::std::os::raw::c_int;
    pub fn libriscv_arena_malloc(m: *mut RISCVMachine, size: u64) -> u64;
    pub fn libriscv_arena_free(m: *mut RISCVMachine, ptr: u64) -> ::std::os::raw::c_int;
    pub fn libriscv_arena_realloc(
        m: *mut RISCVMachine,
        ptr: u64,
        new_size: u64,
    ) -> RISCVReallocResult;
    pub fn libriscv_arena_size(m: *mut RISCVMachine, ptr: u64) -> u64;
    pub fn libriscv_arena_high_watermark(m: *const RISCVMachine) -> u64;
    pub fn libriscv_arena_set_unknown_free(
        m: *mut RISCVMachine,
        handler: riscv_arena_unknown_free_t,
        user: *mut ::std::os::raw::c_void,
    );
    pub fn libriscv_arena_set_unknown_realloc(
        m: *mut RISCVMachine,
        handler: riscv_arena_unknown_realloc_t,
        user: *mut ::std::os::raw::c_void,
    );
    pub fn libriscv_transfer_arena(
        dst: *mut RISCVMachine,
        src: *const RISCVMachine,
    ) -> ::std::os::raw::c_int;
    pub fn libriscv_heap_address(m: *const RISCVMachine) -> u64;
    pub fn libriscv_mmap_allocate(m: *mut RISCVMachine, bytes: u64) -> u64;
    pub fn libriscv_stack_initial(m: *const RISCVMachine) -> u64;
    pub fn libriscv_owned_pages_active(m: *const RISCVMachine) -> u64;
    pub fn libriscv_insert_non_owned_memory(
        m: *mut RISCVMachine,
        dst: u64,
        src: *mut ::std::os::raw::c_void,
        size: u64,
        attr: *const RISCVPageAttributes,
    ) -> ::std::os::raw::c_int;
    pub fn libriscv_setup_linux_syscalls(
        m: *mut RISCVMachine,
        filesystem: ::std::os::raw::c_int,
        sockets: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
    pub fn libriscv_setup_posix_threads(m: *mut RISCVMachine) -> ::std::os::raw::c_int;
    pub fn libriscv_setup_native_memory(
        syscall_base: ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_int;
}
