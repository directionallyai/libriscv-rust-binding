use std::marker::PhantomData;
use std::os::raw::{c_char, c_uint, c_void};
use std::ptr::NonNull;
use std::rc::Rc;

use crate::{check_code, sys, Error, Result};

/// Context passed to safe syscall handlers.
pub struct SyscallContext {
    machine: NonNull<sys::RISCVMachine>,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl SyscallContext {
    /// Create a syscall context from a raw machine pointer.
    ///
    /// # Safety
    /// The pointer must be a valid `RISCVMachine` from libriscv and only used
    /// for the duration of the syscall callback.
    pub unsafe fn from_raw(machine: *mut sys::RISCVMachine) -> Option<Self> {
        NonNull::new(machine).map(|machine| Self {
            machine,
            _not_send_sync: PhantomData,
        })
    }

    /// Borrow the guest registers for this syscall.
    pub fn registers(&mut self) -> Result<SyscallRegisters<'_>> {
        let ptr = unsafe { sys::libriscv_get_registers(self.machine.as_ptr()) };
        let ptr = NonNull::new(ptr).ok_or(Error::NullPointer("libriscv_get_registers"))?;
        Ok(SyscallRegisters {
            ptr,
            _context: PhantomData,
        })
    }

    /// View guest memory as a read-only slice.
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

    /// View guest memory as a writable slice.
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

    /// Read a guest NUL-terminated string up to `maxlen` bytes.
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

    /// Copy guest memory into a host buffer.
    pub fn read(&mut self, src: u64, dst: &mut [u8]) -> Result<()> {
        if dst.is_empty() {
            return Ok(());
        }
        let view = self.memview(src, dst.len())?;
        dst.copy_from_slice(view);
        Ok(())
    }

    /// Copy host data into guest memory.
    pub fn write(&mut self, dst: u64, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        let view = self.writable_memview(dst, data.len())?;
        view.copy_from_slice(data);
        Ok(())
    }

    #[cfg(feature = "bytemuck")]
    /// Read a POD value from guest memory.
    pub fn read_pod<T: bytemuck::Pod>(&mut self, src: u64) -> Result<T> {
        let bytes = self.memview(src, std::mem::size_of::<T>())?;
        Ok(bytemuck::pod_read_unaligned(bytes))
    }

    #[cfg(feature = "bytemuck")]
    /// Write a POD value into guest memory.
    pub fn write_pod<T: bytemuck::Pod>(&mut self, dst: u64, value: &T) -> Result<()> {
        let bytes = self.writable_memview(dst, std::mem::size_of::<T>())?;
        bytes.copy_from_slice(bytemuck::bytes_of(value));
        Ok(())
    }

    /// Stop execution at the next opportunity.
    pub fn stop(&mut self) {
        unsafe {
            sys::libriscv_stop(self.machine.as_ptr());
        }
    }

    /// Return the current instruction counter value.
    pub fn instruction_counter(&self) -> u64 {
        unsafe { sys::libriscv_instruction_counter(self.machine.as_ptr()) }
    }

    /// Borrow the max instruction counter, if available.
    pub fn max_instruction_counter(&mut self) -> Option<&mut u64> {
        let ptr = unsafe { sys::libriscv_max_counter_pointer(self.machine.as_ptr()) };
        let mut ptr = NonNull::new(ptr)?;
        Some(unsafe { ptr.as_mut() })
    }

    /// Trigger a CPU exception (only safe during a syscall).
    pub fn trigger_exception(&mut self, exception: u32, data: u64) {
        unsafe {
            sys::libriscv_trigger_exception(self.machine.as_ptr(), exception, data);
        }
    }

    /// Return the opaque userdata pointer.
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
    /// Read an integer register by index.
    pub fn x(&self, index: usize) -> Result<u64> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_ref() };
        Ok(regs.r[index])
    }

    /// Write an integer register by index.
    pub fn set_x(&mut self, index: usize, value: u64) -> Result<()> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_mut() };
        regs.r[index] = value;
        Ok(())
    }

    /// Read the program counter.
    pub fn pc(&self) -> u64 {
        let regs = unsafe { self.ptr.as_ref() };
        regs.pc
    }

    /// Set the program counter.
    pub fn set_pc(&mut self, value: u64) {
        let regs = unsafe { self.ptr.as_mut() };
        regs.pc = value;
    }

    /// Read the floating-point control/status register.
    pub fn fcsr(&self) -> u32 {
        let regs = unsafe { self.ptr.as_ref() };
        regs.fcsr
    }

    /// Set the floating-point control/status register.
    pub fn set_fcsr(&mut self, value: u32) {
        let regs = unsafe { self.ptr.as_mut() };
        regs.fcsr = value;
    }

    /// Read a single-precision float register by index.
    pub fn f32(&self, index: usize) -> Result<f32> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_ref() };
        Ok(unsafe { regs.fr[index].f32_[0] })
    }

    /// Write a single-precision float register by index.
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

    /// Read a double-precision float register by index.
    pub fn f64(&self, index: usize) -> Result<f64> {
        if index >= 32 {
            return Err(Error::InvalidRegisterIndex { index, max: 31 });
        }
        let regs = unsafe { self.ptr.as_ref() };
        Ok(unsafe { regs.fr[index].f64_ })
    }

    /// Write a double-precision float register by index.
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
    /// Validate and construct a syscall index.
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

    /// Return the raw syscall index.
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
pub struct SyscallHandler(pub(crate) sys::riscv_syscall_handler_t);

impl SyscallHandler {
    /// Clear any previously registered handler.
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

/// Output handling for callback handlers.
pub trait SyscallHandlerOutput {
    /// Convert a handler return value into its side effects.
    fn handle(self);
}

impl SyscallHandlerOutput for () {
    fn handle(self) {}
}

/// Result wrapper for syscall handlers.
pub struct SyscallResult<T>(pub Result<T>);

impl<T> From<Result<T>> for SyscallResult<T> {
    /// Wrap a `Result` so the handler can decide how to handle it.
    fn from(value: Result<T>) -> Self {
        Self(value)
    }
}

impl<T> SyscallHandlerOutput for SyscallResult<T> {
    fn handle(self) {
        let _ = self.0;
    }
}

/// Install a global system call handler with validated syscall index.
pub fn register_syscall_handler(num: SyscallId, handler: SyscallHandler) -> Result<()> {
    let code = unsafe { sys::libriscv_set_syscall_handler(num.get(), handler.0) };
    check_code("libriscv_set_syscall_handler", code)
}
