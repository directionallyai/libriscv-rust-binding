use libriscv::{syscall, syscall_registry, SyscallContext, SyscallResult};

#[syscall_registry]
mod host_syscalls {
    use super::*;

    #[syscall(id = 512)]
    fn host_function_512(_ctx: &mut SyscallContext) -> SyscallResult<()> {
        Ok(())
    }
}

fn main() {
    let _ = 1;
}
