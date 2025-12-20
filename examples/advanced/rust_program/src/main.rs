use std::arch::asm;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

#[repr(C)]
struct Strings {
    count: u64,
    strings: [*const u8; 32],
}

#[repr(C)]
struct Buffer {
    count: u64,
    buffer: [u8; 256],
    another_count: u64,
    another_buffer: *mut u8,
}

type HostFunction = extern "C" fn(*const u8);

#[no_mangle]
extern "C" fn my_function(ptr: *const u8) {
    if ptr.is_null() {
        return;
    }
    let text = unsafe { CStr::from_ptr(ptr as *const c_char) }.to_string_lossy();
    println!("Host says: {}", text);
}

unsafe fn host_function_500(strings: *const Strings) {
    asm!("ecall", in("a0") strings, in("a7") 500u64);
}

unsafe fn host_function_501(buffer: *mut Buffer) {
    asm!("ecall", in("a0") buffer, in("a7") 501u64);
}

unsafe fn host_function_502(func: HostFunction) {
    asm!("ecall", in("a0") func as usize, in("a7") 502u64);
}

unsafe fn host_function_503(mut x: f32, mut y: f32, mut z: f32) -> (f32, f32, f32) {
    asm!(
        "ecall",
        inlateout("fa0") x,
        inlateout("fa1") y,
        inlateout("fa2") z,
        in("a7") 503u64
    );
    (x, y, z)
}

fn main() {
    println!("Hello, Micro RISC-V World!");

    static HELLO: [u8; 6] = *b"Hello\0";
    static WORLD: [u8; 6] = *b"World\0";

    let mut strings = Strings {
        count: 2,
        strings: [ptr::null(); 32],
    };
    strings.strings[0] = HELLO.as_ptr();
    strings.strings[1] = WORLD.as_ptr();

    unsafe {
        host_function_500(&strings);
    }

    let mut another_buf = [0u8; 256];
    let mut buffer = Buffer {
        count: 0,
        buffer: [0u8; 256],
        another_count: another_buf.len() as u64,
        another_buffer: another_buf.as_mut_ptr(),
    };
    unsafe {
        host_function_501(&mut buffer);
    }

    let inline_len = buffer.count as usize;
    if inline_len > 0 {
        let text = String::from_utf8_lossy(&buffer.buffer[..inline_len]);
        println!("Buffer: {}", text);
    }
    let another_len = buffer.another_count as usize;
    if another_len > 0 {
        let text = String::from_utf8_lossy(&another_buf[..another_len]);
        println!("Another Buffer: {}", text);
    }

    unsafe {
        host_function_502(my_function);
    }

    let (x, y, z) = unsafe { host_function_503(0.0, 3.0, 0.0) };
    println!("Normalized vector: {:.1}, {:.1}, {:.1}", x, y, z);
}
