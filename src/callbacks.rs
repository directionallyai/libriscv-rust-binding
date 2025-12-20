use std::ffi::CStr;
use std::marker::PhantomData;
use std::os::raw::{c_char, c_int, c_long, c_uint, c_void};
use std::rc::Rc;

use crate::sys;

/// Opaque pointer passed through the C API.
#[derive(Copy, Clone)]
pub struct Opaque {
    ptr: *mut c_void,
    _not_send_sync: PhantomData<Rc<()>>,
}

impl Opaque {
    pub(crate) fn new(ptr: *mut c_void) -> Self {
        Self {
            ptr,
            _not_send_sync: PhantomData,
        }
    }

    /// Return the raw pointer.
    pub fn as_ptr(self) -> *mut c_void {
        self.ptr
    }

    /// Return whether the pointer is null.
    pub fn is_null(self) -> bool {
        self.ptr.is_null()
    }

    /// Cast the pointer to a shared reference.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid for the desired type.
    pub unsafe fn cast_ref<T>(&self) -> Option<&T> {
        unsafe { self.ptr.cast::<T>().as_ref() }
    }

    /// Cast the pointer to a mutable reference.
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid for the desired type and
    /// uniquely owned for the duration of the borrow.
    pub unsafe fn cast_mut<T>(&mut self) -> Option<&mut T> {
        unsafe { self.ptr.cast::<T>().as_mut() }
    }
}

/// A safe wrapper for the error callback function pointer.
#[derive(Copy, Clone)]
pub struct ErrorHandler(pub(crate) sys::riscv_error_func_t);

impl ErrorHandler {
    /// Clear any previously registered handler.
    pub const fn clear() -> Self {
        Self(None)
    }

    /// # Safety
    /// The handler must be an `extern \"C\"` function pointer with a `'static`
    /// lifetime and must not unwind across the FFI boundary.
    pub unsafe fn new(
        handler: unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_long),
    ) -> Self {
        Self(Some(handler))
    }
}

/// A safe wrapper for the stdout callback function pointer.
#[derive(Copy, Clone)]
pub struct StdoutHandler(pub(crate) sys::riscv_stdout_func_t);

impl StdoutHandler {
    /// Clear any previously registered handler.
    pub const fn clear() -> Self {
        Self(None)
    }

    /// # Safety
    /// The handler must be an `extern \"C\"` function pointer with a `'static`
    /// lifetime and must not unwind across the FFI boundary.
    pub unsafe fn new(
        handler: unsafe extern "C" fn(*mut c_void, *const c_char, c_uint),
    ) -> Self {
        Self(Some(handler))
    }
}

/// Known error types reported by libriscv.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ErrorType {
    GeneralException,
    MachineException,
    MachineTimeout,
    Unknown(i32),
}

impl ErrorType {
    pub(crate) fn from_raw(code: c_int) -> Self {
        match code {
            sys::RISCV_ERROR_TYPE_GENERAL_EXCEPTION => Self::GeneralException,
            sys::RISCV_ERROR_TYPE_MACHINE_EXCEPTION => Self::MachineException,
            sys::RISCV_ERROR_TYPE_MACHINE_TIMEOUT => Self::MachineTimeout,
            _ => Self::Unknown(code),
        }
    }

    /// Return the raw error code reported by libriscv.
    pub fn as_raw(self) -> i32 {
        match self {
            Self::GeneralException => sys::RISCV_ERROR_TYPE_GENERAL_EXCEPTION,
            Self::MachineException => sys::RISCV_ERROR_TYPE_MACHINE_EXCEPTION,
            Self::MachineTimeout => sys::RISCV_ERROR_TYPE_MACHINE_TIMEOUT,
            Self::Unknown(code) => code,
        }
    }
}

/// Context passed to safe error callbacks.
pub struct ErrorContext<'a> {
    error_type: ErrorType,
    message: Option<&'a CStr>,
    data: c_long,
    opaque: Opaque,
}

impl<'a> ErrorContext<'a> {
    /// # Safety
    /// The message pointer must be a valid C string if it is non-null.
    #[doc(hidden)]
    pub unsafe fn from_raw(
        opaque: *mut c_void,
        error_type: c_int,
        msg: *const c_char,
        data: c_long,
    ) -> Self {
        let message = if msg.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(msg) })
        };
        Self {
            error_type: ErrorType::from_raw(error_type),
            message,
            data,
            opaque: Opaque::new(opaque),
        }
    }

    /// Return the error classification.
    pub fn error_type(&self) -> ErrorType {
        self.error_type
    }

    /// Return the error message if one is provided.
    pub fn message(&self) -> Option<&'a CStr> {
        self.message
    }

    /// Return the auxiliary error data.
    pub fn data(&self) -> c_long {
        self.data
    }

    /// Return the opaque userdata pointer.
    pub fn opaque(&self) -> Opaque {
        self.opaque
    }
}

/// Context passed to safe stdout callbacks.
pub struct StdoutContext<'a> {
    data: &'a [u8],
    opaque: Opaque,
}

impl<'a> StdoutContext<'a> {
    /// # Safety
    /// The buffer pointer must be valid for `len` bytes if non-null.
    #[doc(hidden)]
    pub unsafe fn from_raw(opaque: *mut c_void, msg: *const c_char, len: c_uint) -> Self {
        let data = if msg.is_null() || len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(msg as *const u8, len as usize) }
        };
        Self {
            data,
            opaque: Opaque::new(opaque),
        }
    }

    /// Return the stdout bytes for this callback.
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Return the opaque userdata pointer.
    pub fn opaque(&self) -> Opaque {
        self.opaque
    }
}
