//! Safe wrapper for `libriscv_sys`.
//!
//! This crate owns the ELF binary and option storage so the underlying C API
//! receives stable pointers for the lifetime of the machine.

extern crate self as libriscv;

use std::ffi::{CStr, CString, NulError};
use std::marker::PhantomData;
use std::os::raw::{c_char, c_uint, c_void};
use std::path::Path;
use std::ptr::{self, NonNull};
use std::rc::Rc;

pub mod sys {
    pub use libriscv_sys::*;
}

pub use libriscv_macros::syscall_handler;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ArgsTooLarge(usize),
    ElfTooLarge(usize),
    InvalidRegisterIndex { index: usize, max: usize },
    InvalidSyscallIndex { index: usize, max: usize },
    LengthTooLarge { op: &'static str, len: usize },
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
}

impl OptionsBuilder {
    /// Create a builder initialized with libriscv defaults.
    pub fn new() -> Self {
        Self {
            raw: default_raw_options(),
            args: Vec::new(),
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

    pub fn error_handler(mut self, handler: sys::riscv_error_func_t) -> Self {
        self.raw.error = handler;
        self
    }

    pub fn stdout_handler(mut self, handler: sys::riscv_stdout_func_t) -> Self {
        self.raw.stdout = handler;
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

        let keepalive = OptionsKeepAlive {
            _args: args,
            _argv_ptrs: argv_ptrs,
        };
        self.raw.argc = keepalive._argv_ptrs.len() as c_uint;
        self.raw.argv = if keepalive._argv_ptrs.is_empty() {
            ptr::null_mut()
        } else {
            keepalive._argv_ptrs.as_ptr() as *mut *const c_char
        };

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
        }
    }
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
    pub fn new(elf: impl AsRef<[u8]>, mut options: Options) -> Result<Self> {
        let elf = elf.as_ref();
        if elf.len() > u32::MAX as usize {
            return Err(Error::ElfTooLarge(elf.len()));
        }
        let owned = elf.to_vec().into_boxed_slice();
        let ptr = unsafe {
            sys::libriscv_new(
                owned.as_ptr() as *const c_void,
                owned.len() as c_uint,
                &mut options.raw,
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

    pub fn with_defaults(elf: impl AsRef<[u8]>) -> Result<Self> {
        Self::new(elf, Options::default())
    }

    pub fn as_raw(&self) -> *mut sys::RISCVMachine {
        self.ptr.as_ptr()
    }

    pub fn run(&mut self, instruction_limit: u64) -> Result<()> {
        let code = unsafe { sys::libriscv_run(self.ptr.as_ptr(), instruction_limit) };
        check_code("libriscv_run", code)
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

/// Context passed to safe syscall handlers.
pub struct SyscallContext {
    machine: NonNull<sys::RISCVMachine>,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl SyscallContext {
    /// # Safety
    /// The pointer must be a valid `RISCVMachine` from libriscv and only used
    /// for the duration of the syscall callback.
    pub unsafe fn from_raw(machine: *mut sys::RISCVMachine) -> Option<Self> {
        NonNull::new(machine).map(|machine| Self {
            machine,
            _not_send_sync: PhantomData,
        })
    }

    pub fn registers(&mut self) -> Result<SyscallRegisters<'_>> {
        let ptr = unsafe { sys::libriscv_get_registers(self.machine.as_ptr()) };
        let ptr = NonNull::new(ptr).ok_or(Error::NullPointer("libriscv_get_registers"))?;
        Ok(SyscallRegisters {
            ptr,
            _context: PhantomData,
        })
    }

    pub fn memview(&mut self, src: u64, len: usize) -> Result<&[u8]> {
        if len > c_uint::MAX as usize {
            return Err(Error::LengthTooLarge {
                op: "libriscv_memview",
                len,
            });
        }
        let ptr = unsafe { sys::libriscv_memview(self.machine.as_ptr(), src, len as c_uint) };
        let ptr = NonNull::new(ptr as *mut c_char).ok_or(Error::NullPointer("libriscv_memview"))?;
        Ok(unsafe { std::slice::from_raw_parts(ptr.as_ptr() as *const u8, len) })
    }

    pub fn writable_memview(&mut self, src: u64, len: usize) -> Result<&mut [u8]> {
        if len > c_uint::MAX as usize {
            return Err(Error::LengthTooLarge {
                op: "libriscv_writable_memview",
                len,
            });
        }
        let ptr =
            unsafe { sys::libriscv_writable_memview(self.machine.as_ptr(), src, len as c_uint) };
        let ptr = NonNull::new(ptr as *mut c_char)
            .ok_or(Error::NullPointer("libriscv_writable_memview"))?;
        Ok(unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, len) })
    }

    pub fn memstring(&mut self, src: u64, maxlen: usize) -> Result<&[u8]> {
        if maxlen > c_uint::MAX as usize {
            return Err(Error::LengthTooLarge {
                op: "libriscv_memstring",
                len: maxlen,
            });
        }
        let mut length: c_uint = 0;
        let ptr = unsafe {
            sys::libriscv_memstring(
                self.machine.as_ptr(),
                src,
                maxlen as c_uint,
                &mut length,
            )
        };
        let ptr = NonNull::new(ptr as *mut c_char).ok_or(Error::NullPointer("libriscv_memstring"))?;
        Ok(unsafe { std::slice::from_raw_parts(ptr.as_ptr() as *const u8, length as usize) })
    }

    pub fn read_pod<T: bytemuck::Pod>(&mut self, src: u64) -> Result<T> {
        let bytes = self.memview(src, std::mem::size_of::<T>())?;
        Ok(bytemuck::pod_read_unaligned(bytes))
    }

    pub fn write_pod<T: bytemuck::Pod>(&mut self, dst: u64, value: &T) -> Result<()> {
        let bytes = self.writable_memview(dst, std::mem::size_of::<T>())?;
        bytes.copy_from_slice(bytemuck::bytes_of(value));
        Ok(())
    }

    pub fn stop(&mut self) {
        unsafe {
            sys::libriscv_stop(self.machine.as_ptr());
        }
    }

    pub fn instruction_counter(&self) -> u64 {
        unsafe { sys::libriscv_instruction_counter(self.machine.as_ptr()) }
    }

    pub fn max_instruction_counter(&mut self) -> Option<&mut u64> {
        let ptr = unsafe { sys::libriscv_max_counter_pointer(self.machine.as_ptr()) };
        let mut ptr = NonNull::new(ptr)?;
        Some(unsafe { ptr.as_mut() })
    }

    pub fn trigger_exception(&mut self, exception: u32, data: u64) {
        unsafe {
            sys::libriscv_trigger_exception(self.machine.as_ptr(), exception, data);
        }
    }

    pub fn opaque(&self) -> *mut c_void {
        unsafe { sys::libriscv_opaque(self.machine.as_ptr()) }
    }
}

/// Borrowed access to syscall registers.
pub struct SyscallRegisters<'a> {
    ptr: NonNull<sys::RISCVRegisters>,
    _context: PhantomData<&'a mut SyscallContext>,
}

impl<'a> SyscallRegisters<'a> {
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

/// Maximum number of syscall handlers supported by libriscv.
pub const SYSCALLS_MAX: u32 = 512;

/// A validated syscall index.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SyscallId(u32);

impl SyscallId {
    pub fn new(index: u32) -> Result<Self> {
        if index < SYSCALLS_MAX {
            Ok(Self(index))
        } else {
            Err(Error::InvalidSyscallIndex {
                index: index as usize,
                max: (SYSCALLS_MAX - 1) as usize,
            })
        }
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for SyscallId {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self> {
        Self::new(value)
    }
}

impl TryFrom<usize> for SyscallId {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self> {
        if value > u32::MAX as usize {
            return Err(Error::InvalidSyscallIndex {
                index: value,
                max: (SYSCALLS_MAX - 1) as usize,
            });
        }
        Self::new(value as u32)
    }
}

impl From<SyscallId> for u32 {
    fn from(value: SyscallId) -> Self {
        value.0
    }
}

/// A syscall handler that obeys libriscv's FFI invariants.
#[derive(Copy, Clone)]
pub struct SyscallHandler(sys::riscv_syscall_handler_t);

impl SyscallHandler {
    pub const fn clear() -> Self {
        Self(None)
    }

    /// # Safety
    /// The handler must be an `extern "C"` function pointer with a `'static`
    /// lifetime, must not unwind across the FFI boundary, and must only use the
    /// provided `RISCVMachine` pointer for the duration of the callback. The
    /// handler is global and affects all machines.
    pub unsafe fn new(handler: unsafe extern "C" fn(*mut sys::RISCVMachine)) -> Self {
        Self(Some(handler))
    }
}

pub trait SyscallHandlerOutput {
    fn handle(self);
}

impl SyscallHandlerOutput for () {
    fn handle(self) {}
}

impl<T> SyscallHandlerOutput for Result<T> {
    fn handle(self) {
        if let Err(err) = self {
            eprintln!("syscall handler error: {err}");
        }
    }
}

/// Install a global system call handler with validated syscall index.
pub fn register_syscall_handler(num: SyscallId, handler: SyscallHandler) -> Result<()> {
    let code = unsafe { sys::libriscv_set_syscall_handler(num.get(), handler.0) };
    check_code("libriscv_set_syscall_handler", code)
}

/// Install a global system call handler.
///
/// # Safety
/// The handler is invoked by the global libriscv runtime and must obey the
/// safety requirements of the C API (including any threading or reentrancy
/// constraints).
pub unsafe fn set_syscall_handler(
    num: u32,
    handler: sys::riscv_syscall_handler_t,
) -> Result<()> {
    let code = unsafe { sys::libriscv_set_syscall_handler(num, handler) };
    check_code("libriscv_set_syscall_handler", code)
}
