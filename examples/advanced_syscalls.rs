use libriscv::{set_syscall_handler, sys, Machine, Options, Registers, Result};
use std::ffi::CStr;
use std::os::raw::{c_char, c_long, c_uint};
use std::sync::atomic::{AtomicU64, Ordering};

type GuestAddr = u64;

static HOST_FN_ADDR: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
struct Strings {
    count: GuestAddr,
    strings: [GuestAddr; 32],
}

#[repr(C)]
struct Buffers {
    count: GuestAddr,
    buffer: [u8; 256],
    another_count: GuestAddr,
    another_buffer_address: GuestAddr,
}

unsafe extern "C" fn error_callback(
    _opaque: *mut std::ffi::c_void,
    _type: i32,
    msg: *const c_char,
    data: c_long,
) {
    if msg.is_null() {
        eprintln!("Error: <null> (data: 0x{:X})", data);
        return;
    }
    let text = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    eprintln!("Error: {} (data: 0x{:X})", text, data);
}

unsafe extern "C" fn stdout_callback(_opaque: *mut std::ffi::c_void, msg: *const c_char, len: c_uint) {
    if msg.is_null() || len == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(msg as *const u8, len as usize) };
    let text = String::from_utf8_lossy(slice);
    print!("[libriscv] stdout: {}", text);
}

fn write_c_string(dst: &mut [u8], text: &[u8]) -> usize {
    if dst.is_empty() {
        return 0;
    }
    let max = dst.len() - 1;
    let len = text.len().min(max);
    dst[..len].copy_from_slice(&text[..len]);
    dst[len] = 0;
    len
}

unsafe extern "C" fn host_function_500(m: *mut sys::RISCVMachine) {
    println!("Hello from host function 0!");
    let regs = unsafe { sys::libriscv_get_registers(m) };
    if regs.is_null() {
        eprintln!("host_function_500: no registers");
        return;
    }
    let addr = unsafe { (*regs).r[10] };
    let ptr = unsafe {
        sys::libriscv_memview(
            m,
            addr,
            std::mem::size_of::<Strings>() as c_uint,
        )
    };
    if ptr.is_null() {
        eprintln!("host_function_500: bad pointer");
        return;
    }
    let strings = unsafe { &*(ptr as *const Strings) };
    let count = (strings.count as usize).min(strings.strings.len());
    for i in 0..count {
        let mut len: c_uint = 0;
        let s = unsafe { sys::libriscv_memstring(m, strings.strings[i], 256, &mut len) };
        if s.is_null() {
            continue;
        }
        let slice = unsafe { std::slice::from_raw_parts(s as *const u8, len as usize) };
        println!("  {}", String::from_utf8_lossy(slice));
    }
}

unsafe extern "C" fn host_function_501(m: *mut sys::RISCVMachine) {
    println!("Hello from host function 1!");
    let regs = unsafe { sys::libriscv_get_registers(m) };
    if regs.is_null() {
        eprintln!("host_function_501: no registers");
        return;
    }
    let addr = unsafe { (*regs).r[10] };
    let ptr = unsafe {
        sys::libriscv_writable_memview(
            m,
            addr,
            std::mem::size_of::<Buffers>() as c_uint,
        )
    };
    if ptr.is_null() {
        eprintln!("host_function_501: bad pointer");
        return;
    }
    let buf = unsafe { &mut *(ptr as *mut Buffers) };
    let len = write_c_string(&mut buf.buffer, b"Hello from host function 1!");
    buf.count = len as GuestAddr;

    let another_len = buf.another_count as usize;
    if another_len == 0 {
        return;
    }
    if another_len > c_uint::MAX as usize {
        eprintln!("host_function_501: another buffer too large");
        return;
    }
    let another_ptr =
        unsafe { sys::libriscv_writable_memview(m, buf.another_buffer_address, another_len as c_uint) };
    if another_ptr.is_null() {
        eprintln!("host_function_501: invalid another buffer");
        return;
    }
    let another_slice = unsafe { std::slice::from_raw_parts_mut(another_ptr as *mut u8, another_len) };
    let second = b"Another buffer from host function 1!";
    if second.len() >= another_len {
        eprintln!("host_function_501: another buffer too small");
        return;
    }
    let len = write_c_string(another_slice, second);
    buf.another_count = len as GuestAddr;
}

unsafe extern "C" fn host_function_502(m: *mut sys::RISCVMachine) {
    let regs = unsafe { sys::libriscv_get_registers(m) };
    if regs.is_null() {
        eprintln!("host_function_502: no registers");
        return;
    }
    let addr = unsafe { (*regs).r[10] };
    HOST_FN_ADDR.store(addr, Ordering::Relaxed);
}

unsafe extern "C" fn host_function_503(m: *mut sys::RISCVMachine) {
    let regs = unsafe { sys::libriscv_get_registers(m) };
    if regs.is_null() {
        eprintln!("host_function_503: no registers");
        return;
    }
    let regs = unsafe { &mut *regs };
    let mut x = unsafe { regs.fr[10].f32_[0] };
    let mut y = unsafe { regs.fr[11].f32_[0] };
    let mut z = unsafe { regs.fr[12].f32_[0] };

    let len = (x * x + y * y + z * z).sqrt();
    if len > 0.0 {
        let inv = 1.0 / len;
        x *= inv;
        y *= inv;
        z *= inv;
    }

    unsafe {
        regs.fr[10].f32_[0] = x;
        regs.fr[11].f32_[0] = y;
        regs.fr[12].f32_[0] = z;
    }
}

fn reserve_stack(regs: &mut Registers<'_>, size: usize) -> Result<u64> {
    let sp = regs.x(2)?;
    let new_sp = sp.wrapping_sub(size as u64) & !0xFu64;
    regs.set_x(2, new_sp)?;
    Ok(new_sp)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} [program file]", args[0]);
        std::process::exit(1);
    }

    let elf = std::fs::read(&args[1])?;

    unsafe {
        set_syscall_handler(500, Some(host_function_500))?;
        set_syscall_handler(501, Some(host_function_501))?;
        set_syscall_handler(502, Some(host_function_502))?;
        set_syscall_handler(503, Some(host_function_503))?;
    }

    let mut options = Options::new();
    options.set_stdout_handler(Some(stdout_callback));
    options.set_error_handler(Some(error_callback));
    options.set_args(["program"])?;

    let mut machine = Machine::new(elf, options)?;
    machine.run(u64::MAX)?;

    let addr = HOST_FN_ADDR.load(Ordering::Relaxed);
    if addr != 0 {
        machine.setup_vmcall(addr)?;
        let msg = b"Hello from a callback function!\0";
        let str_addr = {
            let mut regs = machine.registers()?;
            reserve_stack(&mut regs, msg.len())?
        };
        machine.copy_to_guest(str_addr, msg)?;
        {
            let mut regs = machine.registers()?;
            regs.set_x(10, str_addr)?;
        }
        machine.run(u64::MAX)?;
    } else {
        eprintln!("Host function 502 was not called");
    }

    println!("Done");
    Ok(())
}
