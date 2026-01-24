# libriscv-macros

Proc-macro helpers for libriscv to declare syscall, stdout, and error handlers,
including the syscall registry macros.

## How to use

Add this crate as a dependency (usually via `libriscv`) and apply the attribute
macros to your handler functions. See the `libriscv` crate for signatures and
examples.

Short example using the syscall registry macros:

```rust
use libriscv::{syscall, syscall_registry, SyscallContext, SyscallResult};

#[syscall_registry]
mod host_syscalls {
    use super::*;

    #[syscall(id = 1)]
    fn write(_ctx: &mut SyscallContext) -> SyscallResult<()> {
        Ok(())
    }
}

let registry = host_syscalls::registry().unwrap();
```
