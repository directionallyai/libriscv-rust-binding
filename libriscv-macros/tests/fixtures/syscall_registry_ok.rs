use libriscv::{syscall, syscall_registry, SyscallContext, SyscallRegistryBuilder, SyscallResult};

#[syscall_registry]
mod host_syscalls {
    use super::*;

    #[syscall(id = 500)]
    fn host_function_500(_ctx: &mut SyscallContext) -> SyscallResult<()> {
        Ok(())
    }
}

fn main() {
    let mut builder = SyscallRegistryBuilder::new();
    host_syscalls::register(&mut builder).unwrap();
    let _registry = host_syscalls::registry().unwrap();
}
