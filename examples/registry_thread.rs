use libriscv::{syscall_handler, SyscallContext, SyscallId, SyscallRegistryBuilder, SyscallResult};

#[syscall_handler]
fn nop(_ctx: &mut SyscallContext) -> SyscallResult<()> {
    Ok(())
}

fn main() {
    let mut registry_builder = SyscallRegistryBuilder::new();
    registry_builder
        .register(SyscallId::new(500).unwrap(), nop_handler())
        .unwrap();
    let _registry = registry_builder.build();

    let handle = std::thread::spawn(|| 1 + 1);
    let _ = handle.join();

    #[cfg(any())]
    {
        // Remove the cfg to see the compile-time error: SyscallRegistryBuilder is !Send.
        let mut registry_builder = SyscallRegistryBuilder::new();
        std::thread::spawn(move || {
            registry_builder
                .register(SyscallId::new(501).unwrap(), nop_handler())
                .unwrap();
        });
    }
}
