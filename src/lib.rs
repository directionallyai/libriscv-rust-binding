//! Safe wrapper for `libriscv_sys`.
//!
//! This crate owns the ELF binary and option storage so the underlying C API
//! receives stable pointers for the lifetime of the machine.

extern crate self as libriscv;

use std::ffi::{CStr, CString, NulError};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::path::Path;
use std::ptr::{self, NonNull};
use std::rc::Rc;

mod syscall;
mod callbacks;
pub use syscall::{
    SyscallContext,
    SyscallHandler,
    SyscallHandlerOutput,
    SyscallId,
    SyscallRegistry,
    SyscallRegistryBuilder,
    SyscallResult,
    SyscallRegisters,
    SYSCALLS_MAX,
};
pub use callbacks::{
    ErrorContext,
    ErrorHandler,
    ErrorType,
    Opaque,
    StdoutHandler,
    StdoutContext,
};

pub mod sys {
    pub use libriscv_sys::*;
}

pub use libriscv_macros::{
    error_handler, stdout_handler, syscall, syscall_handler, syscall_registry,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ArgsTooLarge(usize),
    ElfTooLarge(usize),
    InvalidRegisterIndex { index: usize, max: usize },
    InvalidSyscallIndex { index: usize, max: usize },
    LengthTooLarge { op: &'static str, len: usize },
    UnalignedPageRange {
        destination: u64,
        size: u64,
    },
    Library {
        op: &'static str,
        code: i32,
        message: Option<&'static str>,
    },
    NonUtf8Path,
    NullPointer(&'static str),
    NulError(NulError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ArgsTooLarge(len) => write!(f, "too many arguments: {len}"),
            Error::ElfTooLarge(len) => write!(f, "ELF size {len} exceeds u32::MAX"),
            Error::InvalidRegisterIndex { index, max } => {
                write!(f, "register index {index} out of range (max {max})")
            }
            Error::InvalidSyscallIndex { index, max } => {
                write!(f, "syscall index {index} out of range (max {max})")
            }
            Error::LengthTooLarge { op, len } => {
                write!(f, "{op} length {len} exceeds u32::MAX")
            }
            Error::UnalignedPageRange { destination, size } => write!(
                f,
                "guest page range address {destination:#x} and size {size:#x} must be 4096-byte aligned"
            ),
            Error::Library { op, code, message } => {
                if let Some(message) = message {
                    write!(f, "{op} failed ({code}): {message}")
                } else {
                    write!(f, "{op} failed ({code})")
                }
            }
            Error::NonUtf8Path => write!(f, "path contains invalid UTF-8"),
            Error::NullPointer(op) => write!(f, "{op} returned a null pointer"),
            Error::NulError(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<NulError> for Error {
    fn from(value: NulError) -> Self {
        Error::NulError(value)
    }
}

fn error_message(code: i32) -> Option<&'static str> {
    if code >= 0 {
        return None;
    }
    unsafe {
        let ptr = sys::libriscv_strerror(code);
        if ptr.is_null() {
            None
        } else {
            CStr::from_ptr(ptr).to_str().ok()
        }
    }
}

fn check_code(op: &'static str, code: i32) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(Error::Library {
            op,
            code,
            message: error_message(code),
        })
    }
}

fn default_raw_options() -> sys::RISCVOptions {
    let mut raw = std::mem::MaybeUninit::<sys::RISCVOptions>::zeroed();
    unsafe {
        sys::libriscv_set_defaults(raw.as_mut_ptr());
    }
    let mut raw = unsafe { raw.assume_init() };
    raw.argc = 0;
    raw.argv = ptr::null_mut();
    raw.error = None;
    raw.stdout = None;
    raw.opaque = ptr::null_mut();
    raw
}

/// Configuration for creating a RISC-V machine.
pub struct Options {
    raw: sys::RISCVOptions,
    _keepalive: OptionsKeepAlive,
}

struct OptionsKeepAlive {
    _args: Vec<CString>,
    _argv_ptrs: Vec<*const c_char>,
    _default_exit_function: Option<CString>,
}

impl Options {
    /// Create options initialized with libriscv defaults.
    pub fn new() -> Self {
        Self {
            raw: default_raw_options(),
            _keepalive: OptionsKeepAlive::empty(),
        }
    }

    /// Start building a validated options struct.
    pub fn builder() -> OptionsBuilder {
        OptionsBuilder::new()
    }
}

/// Builder for [`Options`].
#[must_use]
pub struct OptionsBuilder {
    raw: sys::RISCVOptions,
    args: Vec<String>,
    default_exit_function: Option<String>,
}

impl OptionsBuilder {
    /// Create a builder initialized with libriscv defaults.
    pub fn new() -> Self {
        Self {
            raw: default_raw_options(),
            args: Vec::new(),
            default_exit_function: None,
        }
    }

    pub fn max_memory(mut self, bytes: u64) -> Self {
        self.raw.max_memory = bytes;
        self
    }

    pub fn stack_size(mut self, bytes: u32) -> Self {
        self.raw.stack_size = bytes;
        self
    }

    pub fn strict_sandbox(mut self, strict: bool) -> Self {
        self.raw.strict_sandbox = if strict { 1 } else { 0 };
        self
    }

    pub fn use_memory_arena(mut self, enabled: bool) -> Self {
        self.raw.use_memory_arena = if enabled { 1 } else { 0 };
        self
    }

    pub fn use_shared_execute_segments(mut self, enabled: bool) -> Self {
        self.raw.use_shared_execute_segments = if enabled { 1 } else { 0 };
        self
    }

    pub fn default_exit_function(mut self, symbol: impl Into<String>) -> Self {
        self.default_exit_function = Some(symbol.into());
        self
    }

    pub fn clear_default_exit_function(mut self) -> Self {
        self.default_exit_function = None;
        self
    }

    pub fn load_program(mut self, enabled: bool) -> Self {
        self.raw.load_program = if enabled { 1 } else { 0 };
        self
    }

    pub fn protect_segments(mut self, enabled: bool) -> Self {
        self.raw.protect_segments = if enabled { 1 } else { 0 };
        self
    }

    pub fn native_syscall_base(mut self, base: u32) -> Self {
        self.raw.native_syscall_base = base;
        self
    }

    pub fn arena_size(mut self, bytes: u64) -> Self {
        self.raw.arena_size = bytes;
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.args = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect();
        self
    }

    pub fn clear_args(mut self) -> Self {
        self.args.clear();
        self
    }

    pub fn error_handler(mut self, handler: ErrorHandler) -> Self {
        self.raw.error = handler.0;
        self
    }

    pub fn stdout_handler(mut self, handler: StdoutHandler) -> Self {
        self.raw.stdout = handler.0;
        self
    }

    pub fn opaque(mut self, opaque: *mut c_void) -> Self {
        self.raw.opaque = opaque;
        self
    }

    pub fn build(mut self) -> Result<Options> {
        let args_len = self.args.len();
        if args_len > c_uint::MAX as usize {
            return Err(Error::ArgsTooLarge(args_len));
        }

        let args = self
            .args
            .into_iter()
            .map(CString::new)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let argv_ptrs: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
        let default_exit_function = self
            .default_exit_function
            .map(CString::new)
            .transpose()?;

        let keepalive = OptionsKeepAlive {
            _args: args,
            _argv_ptrs: argv_ptrs,
            _default_exit_function: default_exit_function,
        };
        self.raw.argc = keepalive._argv_ptrs.len() as c_uint;
        self.raw.argv = if keepalive._argv_ptrs.is_empty() {
            ptr::null_mut()
        } else {
            keepalive._argv_ptrs.as_ptr() as *mut *const c_char
        };
        self.raw.default_exit_function = keepalive
            ._default_exit_function
            .as_ref()
            .map_or(ptr::null(), |symbol| symbol.as_ptr());

        Ok(Options {
            raw: self.raw,
            _keepalive: keepalive,
        })
    }
}

impl OptionsKeepAlive {
    const fn empty() -> Self {
        Self {
            _args: Vec::new(),
            _argv_ptrs: Vec::new(),
            _default_exit_function: None,
        }
    }
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub struct PageAttributes {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub copy_on_write: bool,
    pub non_owning: bool,
    pub dont_fork: bool,
    pub user_defined: u8,
}

impl PageAttributes {
    fn from_raw(raw: sys::RISCVPageAttributes) -> Self {
        Self {
            read: raw.read != 0,
            write: raw.write != 0,
            execute: raw.exec != 0,
            copy_on_write: raw.is_cow != 0,
            non_owning: raw.non_owning != 0,
            dont_fork: raw.dont_fork != 0,
            user_defined: raw.user_defined,
        }
    }

    fn into_raw(self) -> sys::RISCVPageAttributes {
        sys::RISCVPageAttributes {
            read: self.read as i32,
            write: self.write as i32,
            exec: self.execute as i32,
            is_cow: self.copy_on_write as i32,
            non_owning: self.non_owning as i32,
            dont_fork: self.dont_fork as i32,
            user_defined: self.user_defined,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ArenaReallocResult {
    pub address: Option<NonZeroU64>,
    pub old_size: u64,
}

impl Default for OptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

/// A running instance of the libriscv machine.
pub struct Machine {
    ptr: NonNull<sys::RISCVMachine>,
    _elf: Box<[u8]>,
    _options: Options,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl Machine {
    pub fn new(
        elf: impl AsRef<[u8]>,
        options: Options,
        _registry: &SyscallRegistry,
    ) -> Result<Self> {
        let elf = elf.as_ref();
        if elf.len() > u32::MAX as usize {
            return Err(Error::ElfTooLarge(elf.len()));
        }
        let owned = elf.to_vec().into_boxed_slice();
        let ptr = unsafe {
            sys::libriscv_new(
                owned.as_ptr() as *const c_void,
                owned.len() as c_uint,
                &options.raw,
            )
        };
        let ptr = NonNull::new(ptr).ok_or(Error::NullPointer("libriscv_new"))?;
        Ok(Self {
            ptr,
            _elf: owned,
            _options: options,
            _not_send_sync: PhantomData,
        })
    }

    pub fn with_defaults(elf: impl AsRef<[u8]>, registry: &SyscallRegistry) -> Result<Self> {
        Self::new(elf, Options::default(), registry)
    }

    pub fn as_raw(&self) -> *mut sys::RISCVMachine {
        self.ptr.as_ptr()
    }

    pub fn run(&mut self, instruction_limit: u64) -> Result<()> {
        let code = unsafe { sys::libriscv_run(self.ptr.as_ptr(), instruction_limit) };
        check_code("libriscv_run", code)
    }

    pub fn step(&mut self, verbose: bool) -> Result<Option<u64>> {
        let value = unsafe { sys::libriscv_step_one(self.ptr.as_ptr(), verbose as i32) };
        if value < 0 {
            check_code("libriscv_step_one", value as i32)?;
        }
        Ok((value > 0).then_some(value as u64))
    }

    pub fn stop(&mut self) {
        unsafe {
            sys::libriscv_stop(self.ptr.as_ptr());
        }
    }

    pub fn allow_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_str().ok_or(Error::NonUtf8Path)?;
        let c_path = CString::new(path)?;
        unsafe {
            sys::libriscv_allow_file(self.ptr.as_ptr(), c_path.as_ptr());
        }
        Ok(())
    }

    pub fn return_value(&self) -> i64 {
        unsafe { sys::libriscv_return_value(self.ptr.as_ptr()) }
    }

    pub fn set_result(&mut self, value: i64) {
        unsafe { sys::libriscv_set_result_register(self.ptr.as_ptr(), value) }
    }

    pub fn address_of(&self, name: &str) -> Result<Option<u64>> {
        let c_name = CString::new(name)?;
        let addr = unsafe { sys::libriscv_address_of(self.ptr.as_ptr(), c_name.as_ptr()) };
        if addr == 0 {
            Ok(None)
        } else {
            Ok(Some(addr))
        }
    }

    pub fn opaque(&self) -> *mut c_void {
        unsafe { sys::libriscv_opaque(self.ptr.as_ptr()) }
    }

    pub fn instruction_counter(&self) -> u64 {
        unsafe { sys::libriscv_instruction_counter(self.ptr.as_ptr()) }
    }

    pub fn max_instruction_counter(&mut self) -> Option<&mut u64> {
        let ptr = unsafe { sys::libriscv_max_counter_pointer(self.ptr.as_ptr()) };
        let mut ptr = NonNull::new(ptr)?;
        Some(unsafe { ptr.as_mut() })
    }

    pub fn instruction_limit_reached(&self) -> bool {
        unsafe { sys::libriscv_instruction_limit_reached(self.ptr.as_ptr()) != 0 }
    }

    pub fn is_forked(&self) -> bool {
        unsafe { sys::libriscv_is_forked(self.ptr.as_ptr()) != 0 }
    }

    pub fn fast_fork(&self, mut options: Options) -> Result<ForkedMachine<'_>> {
        let ptr = unsafe { sys::libriscv_fast_fork(self.ptr.as_ptr(), &mut options.raw) };
        let ptr = NonNull::new(ptr).ok_or(Error::NullPointer("libriscv_fast_fork"))?;
        Ok(ForkedMachine {
            machine: Machine {
                ptr,
                _elf: Box::new([]),
                _options: options,
                _not_send_sync: PhantomData,
            },
            _parent: PhantomData,
        })
    }

    pub fn parent_page(
        &self,
        page_number: u64,
    ) -> Option<(&[u8; sys::RISCV_PAGE_SIZE as usize], PageAttributes)> {
        let mut attributes = std::mem::MaybeUninit::<sys::RISCVPageAttributes>::uninit();
        let ptr = unsafe {
            sys::libriscv_get_parent_page_data(
                self.ptr.as_ptr(),
                page_number,
                attributes.as_mut_ptr(),
            )
        };
        let ptr = NonNull::new(ptr as *mut [u8; sys::RISCV_PAGE_SIZE as usize])?;
        let attributes = PageAttributes::from_raw(unsafe { attributes.assume_init() });
        Some((unsafe { ptr.as_ref() }, attributes))
    }

    pub fn setup_arena(&mut self, syscall_base: u32, address: u64, size: u64) -> Result<()> {
        let code = unsafe {
            sys::libriscv_setup_arena(self.ptr.as_ptr(), syscall_base, address, size)
        };
        check_code("libriscv_setup_arena", code)
    }

    pub fn has_arena(&self) -> bool {
        unsafe { sys::libriscv_has_arena(self.ptr.as_ptr()) != 0 }
    }

    pub fn arena_malloc(&mut self, size: u64) -> Option<NonZeroU64> {
        NonZeroU64::new(unsafe { sys::libriscv_arena_malloc(self.ptr.as_ptr(), size) })
    }

    pub fn arena_free(&mut self, address: u64) -> Result<()> {
        let code = unsafe { sys::libriscv_arena_free(self.ptr.as_ptr(), address) };
        check_code("libriscv_arena_free", code)
    }

    pub fn arena_realloc(&mut self, address: u64, new_size: u64) -> ArenaReallocResult {
        let result =
            unsafe { sys::libriscv_arena_realloc(self.ptr.as_ptr(), address, new_size) };
        ArenaReallocResult {
            address: NonZeroU64::new(result.ptr),
            old_size: result.old_size,
        }
    }

    pub fn arena_allocation_size(&mut self, address: u64) -> u64 {
        unsafe { sys::libriscv_arena_size(self.ptr.as_ptr(), address) }
    }

    pub fn arena_high_watermark(&self) -> u64 {
        unsafe { sys::libriscv_arena_high_watermark(self.ptr.as_ptr()) }
    }

    /// Install the callback used when the arena frees an unknown pointer.
    ///
    /// # Safety
    /// The callback must not unwind across the FFI boundary. `user` must remain
    /// valid for every callback invocation until the handler is replaced or the
    /// machine is dropped.
    pub unsafe fn set_arena_unknown_free_handler(
        &mut self,
        handler: unsafe extern "C" fn(u64, *mut c_void) -> c_int,
        user: *mut c_void,
    ) {
        unsafe {
            sys::libriscv_arena_set_unknown_free(self.ptr.as_ptr(), Some(handler), user);
        }
    }

    /// Install the callback used when the arena reallocates an unknown pointer.
    ///
    /// # Safety
    /// The callback must not unwind across the FFI boundary. `user` must remain
    /// valid for every callback invocation until the handler is replaced or the
    /// machine is dropped.
    pub unsafe fn set_arena_unknown_realloc_handler(
        &mut self,
        handler: unsafe extern "C" fn(u64, u64, *mut c_void) -> sys::RISCVReallocResult,
        user: *mut c_void,
    ) {
        unsafe {
            sys::libriscv_arena_set_unknown_realloc(self.ptr.as_ptr(), Some(handler), user);
        }
    }

    pub fn transfer_arena_from(&mut self, source: &Machine) -> Result<()> {
        let code = unsafe {
            sys::libriscv_transfer_arena(self.ptr.as_ptr(), source.ptr.as_ptr())
        };
        check_code("libriscv_transfer_arena", code)
    }

    pub fn heap_address(&self) -> u64 {
        unsafe { sys::libriscv_heap_address(self.ptr.as_ptr()) }
    }

    pub fn mmap_allocate(&mut self, bytes: u64) -> Option<NonZeroU64> {
        NonZeroU64::new(unsafe { sys::libriscv_mmap_allocate(self.ptr.as_ptr(), bytes) })
    }

    pub fn initial_stack_pointer(&self) -> u64 {
        unsafe { sys::libriscv_stack_initial(self.ptr.as_ptr()) }
    }

    pub fn owned_pages_active(&self) -> u64 {
        unsafe { sys::libriscv_owned_pages_active(self.ptr.as_ptr()) }
    }

    pub fn setup_linux_syscalls(&mut self, filesystem: bool, sockets: bool) -> Result<()> {
        let code = unsafe {
            sys::libriscv_setup_linux_syscalls(
                self.ptr.as_ptr(),
                filesystem as i32,
                sockets as i32,
            )
        };
        check_code("libriscv_setup_linux_syscalls", code)
    }

    pub fn setup_posix_threads(&mut self) -> Result<()> {
        let code = unsafe { sys::libriscv_setup_posix_threads(self.ptr.as_ptr()) };
        check_code("libriscv_setup_posix_threads", code)
    }

    /// # Safety
    /// `source` must point to at least `size` initialized bytes and remain valid
    /// and unmoved until this machine is dropped. No Rust references may access
    /// the memory while libriscv is reading or writing it.
    pub unsafe fn insert_non_owned_memory(
        &mut self,
        destination: u64,
        source: *mut c_void,
        size: u64,
        attributes: PageAttributes,
    ) -> Result<()> {
        let page_size = u64::from(sys::RISCV_PAGE_SIZE);
        let page_mask = page_size - 1;
        if (destination | size) & page_mask != 0 {
            return Err(Error::UnalignedPageRange { destination, size });
        }
        let raw_attributes = attributes.into_raw();
        let code = unsafe {
            sys::libriscv_insert_non_owned_memory(
                self.ptr.as_ptr(),
                destination,
                source,
                size,
                &raw_attributes,
            )
        };
        check_code("libriscv_insert_non_owned_memory", code)
    }

    pub fn jump(&mut self, address: u64) -> Result<()> {
        let code = unsafe { sys::libriscv_jump(self.ptr.as_ptr(), address) };
        check_code("libriscv_jump", code)
    }

    pub fn setup_vmcall(&mut self, address: u64) -> Result<()> {
        let code = unsafe { sys::libriscv_setup_vmcall(self.ptr.as_ptr(), address) };
        check_code("libriscv_setup_vmcall", code)
    }

    pub fn copy_to_guest(&mut self, dst: u64, src: &[u8]) -> Result<()> {
        if src.len() > c_uint::MAX as usize {
            return Err(Error::LengthTooLarge {
                op: "libriscv_copy_to_guest",
                len: src.len(),
            });
        }
        let code = unsafe {
            sys::libriscv_copy_to_guest(
                self.ptr.as_ptr(),
                dst,
                src.as_ptr() as *const c_void,
                src.len() as c_uint,
            )
        };
        check_code("libriscv_copy_to_guest", code)
    }

    pub fn copy_from_guest(&mut self, src: u64, dst: &mut [u8]) -> Result<()> {
        if dst.len() > c_uint::MAX as usize {
            return Err(Error::LengthTooLarge {
                op: "libriscv_copy_from_guest",
                len: dst.len(),
            });
        }
        let code = unsafe {
            sys::libriscv_copy_from_guest(
                self.ptr.as_ptr(),
                dst.as_mut_ptr() as *mut c_void,
                src,
                dst.len() as c_uint,
            )
        };
        check_code("libriscv_copy_from_guest", code)
    }

    pub fn read_memory(&mut self, src: u64, len: usize) -> Result<Vec<u8>> {
        if len > c_uint::MAX as usize {
            return Err(Error::LengthTooLarge {
                op: "read_memory",
                len,
            });
        }
        let mut buf = vec![0u8; len];
        self.copy_from_guest(src, &mut buf)?;
        Ok(buf)
    }

    pub fn write_memory(&mut self, dst: u64, data: &[u8]) -> Result<()> {
        self.copy_to_guest(dst, data)
    }

    pub fn memstring(&mut self, src: u64, maxlen: u32) -> Result<Vec<u8>> {
        let mut length: c_uint = 0;
        let ptr =
            unsafe { sys::libriscv_memstring(self.ptr.as_ptr(), src, maxlen, &mut length) };
        let ptr = NonNull::new(ptr as *mut c_char).ok_or(Error::NullPointer("libriscv_memstring"))?;
        let slice = unsafe { std::slice::from_raw_parts(ptr.as_ptr() as *const u8, length as usize) };
        Ok(slice.to_vec())
    }

    pub fn registers(&mut self) -> Result<Registers<'_>> {
        let ptr = unsafe { sys::libriscv_get_registers(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).ok_or(Error::NullPointer("libriscv_get_registers"))?;
        Ok(Registers {
            ptr,
            _machine: PhantomData,
        })
    }

    /// Only safe to call from a syscall handler.
    ///
    /// # Safety
    /// The caller must ensure this is invoked from a valid syscall handler
    /// context for the current machine instance.
    pub unsafe fn trigger_exception(&mut self, exception: u32, data: u64) {
        unsafe {
            sys::libriscv_trigger_exception(self.ptr.as_ptr(), exception, data);
        }
    }
}

pub struct ForkedMachine<'parent> {
    machine: Machine,
    _parent: PhantomData<&'parent Machine>,
}

impl Deref for ForkedMachine<'_> {
    type Target = Machine;

    fn deref(&self) -> &Self::Target {
        &self.machine
    }
}

impl DerefMut for ForkedMachine<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.machine
    }
}

pub fn setup_native_memory(syscall_base: u32) -> Result<()> {
    let code = unsafe { sys::libriscv_setup_native_memory(syscall_base) };
    check_code("libriscv_setup_native_memory", code)
}

impl Drop for Machine {
    fn drop(&mut self) {
        unsafe {
            sys::libriscv_delete(self.ptr.as_ptr());
        }
    }
}

/// Borrowed access to the machine registers.
pub struct Registers<'a> {
    ptr: NonNull<sys::RISCVRegisters>,
    _machine: PhantomData<&'a mut Machine>,
}

impl<'a> Registers<'a> {
    pub fn x(&self, index: usize) -> Result<u64> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_ref() };
        Ok(regs.r[index])
    }

    pub fn set_x(&mut self, index: usize, value: u64) -> Result<()> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_mut() };
        regs.r[index] = value;
        Ok(())
    }

    pub fn pc(&self) -> u64 {
        let regs = unsafe { self.ptr.as_ref() };
        regs.pc
    }

    pub fn set_pc(&mut self, value: u64) {
        let regs = unsafe { self.ptr.as_mut() };
        regs.pc = value;
    }

    pub fn fcsr(&self) -> u32 {
        let regs = unsafe { self.ptr.as_ref() };
        regs.fcsr
    }

    pub fn set_fcsr(&mut self, value: u32) {
        let regs = unsafe { self.ptr.as_mut() };
        regs.fcsr = value;
    }

    pub fn f32(&self, index: usize) -> Result<f32> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_ref() };
        Ok(unsafe { regs.fr[index].f32_[0] })
    }

    pub fn set_f32(&mut self, index: usize, value: f32) -> Result<()> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_mut() };
        unsafe {
            regs.fr[index].f32_[0] = value;
        }
        Ok(())
    }

    pub fn f64(&self, index: usize) -> Result<f64> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_ref() };
        Ok(unsafe { regs.fr[index].f64_ })
    }

    pub fn set_f64(&mut self, index: usize, value: f64) -> Result<()> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_mut() };
        regs.fr[index].f64_ = value;
        Ok(())
    }
}
