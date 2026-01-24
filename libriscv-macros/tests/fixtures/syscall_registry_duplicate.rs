#![allow(unused_imports)]

use libriscv::{syscall, syscall_registry, SyscallContext, SyscallResult};

#[syscall_registry]
mod host_syscalls {
    use super::*;

    #[syscall(id = 500)]
    fn host_function_500(_ctx: &mut SyscallContext) -> SyscallResult<()> {
        Ok(())
    }

    #[syscall(id = 500)]
    fn host_function_500_dupe(_ctx: &mut SyscallContext) -> SyscallResult<()> {
        Ok(())
    }
}

fn main() {
    let _ = 1;
}
