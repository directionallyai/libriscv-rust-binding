use libriscv::{
    register_syscall_handler, syscall_handler, Machine, Options, Registers, Result, SyscallContext,
    SyscallId,
};
use std::ffi::CStr;
use std::os::raw::{c_char, c_long, c_uint};
use std::sync::atomic::{AtomicU64, Ordering};

type GuestAddr = u64;

static HOST_FN_ADDR: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Strings {
    count: GuestAddr,
    strings: [GuestAddr; 32],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
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

#[syscall_handler]
fn host_function_500(ctx: &mut SyscallContext) -> Result<()> {
    println!("Hello from host function 0!");
    let addr = {
        let regs = ctx.registers()?;
        regs.x(10)?
    };
    let strings: Strings = ctx.read_pod(addr)?;
    let count = (strings.count as usize).min(strings.strings.len());
    for i in 0..count {
        let slice = match ctx.memstring(strings.strings[i], 256) {
            Ok(slice) => slice,
            Err(_) => continue,
        };
        println!("  {}", String::from_utf8_lossy(slice));
    }
    Ok(())
}

#[syscall_handler]
fn host_function_501(ctx: &mut SyscallContext) -> Result<()> {
    println!("Hello from host function 1!");
    let addr = ctx.registers()?.x(10)?;
    let mut buf: Buffers = ctx.read_pod(addr)?;
    let len = write_c_string(&mut buf.buffer, b"Hello from host function 1!");
    buf.count = len as GuestAddr;

    let another_len = buf.another_count as usize;
    if another_len == 0 {
        ctx.write_pod(addr, &buf)?;
        return Ok(());
    }
    if another_len > c_uint::MAX as usize {
        eprintln!("host_function_501: another buffer too large");
        ctx.write_pod(addr, &buf)?;
        return Ok(());
    }
    let another_slice = ctx.writable_memview(buf.another_buffer_address, another_len)?;
    let second = b"Another buffer from host function 1!";
    if second.len() >= another_len {
        eprintln!("host_function_501: another buffer too small");
        ctx.write_pod(addr, &buf)?;
        return Ok(());
    }
    let len = write_c_string(another_slice, second);
    buf.another_count = len as GuestAddr;
    ctx.write_pod(addr, &buf)?;
    Ok(())
}

#[syscall_handler]
fn host_function_502(ctx: &mut SyscallContext) -> Result<()> {
    let addr = ctx.registers()?.x(10)?;
    HOST_FN_ADDR.store(addr, Ordering::Relaxed);
    Ok(())
}

#[syscall_handler]
fn host_function_503(ctx: &mut SyscallContext) -> Result<()> {
    let mut regs = ctx.registers()?;
    let mut x = regs.f32(10)?;
    let mut y = regs.f32(11)?;
    let mut z = regs.f32(12)?;

    let len = (x * x + y * y + z * z).sqrt();
    if len > 0.0 {
        let inv = 1.0 / len;
        x *= inv;
        y *= inv;
        z *= inv;
    }

    regs.set_f32(10, x)?;
    regs.set_f32(11, y)?;
    regs.set_f32(12, z)?;
    Ok(())
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

    register_syscall_handler(SyscallId::new(500)?, host_function_500_handler())?;
    register_syscall_handler(SyscallId::new(501)?, host_function_501_handler())?;
    register_syscall_handler(SyscallId::new(502)?, host_function_502_handler())?;
    register_syscall_handler(SyscallId::new(503)?, host_function_503_handler())?;

    let options = Options::builder()
        .stdout_handler(Some(stdout_callback))
        .error_handler(Some(error_callback))
        .args(["program"])
        .build()?;

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
