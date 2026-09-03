# `libriscv`

Safe Rust wrapper around libriscv_sys, a fast RISC-V sandbox emulator.
This workspace vendors and builds libriscv v1.20 through the local
`libriscv-sys` crate.

## How to use

Add the crate to your dependencies and run a RISC-V ELF:

```rust
use libriscv::{Machine, Options, SyscallRegistry};

let elf = std::fs::read("program").unwrap();
let options = Options::builder().build().unwrap();
let registry = SyscallRegistry::empty();
let mut machine = Machine::new(&elf, options, &registry).unwrap();
machine.run(1_000_000).unwrap();
```

You can also define registries using the macros:

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

See `examples/` for more usage patterns.

The v1.20 options and machine APIs include memory-arena configuration,
single-step execution, instruction-limit queries, Linux/POSIX setup helpers,
and native arena allocation helpers. Lower-level fork, page, and callback APIs
remain available through `libriscv::sys`.

For crates.io releases, publish `libriscv_sys` 0.2.0 before publishing
`libriscv` 0.4.0 so Cargo can resolve the non-path dependency used by packaged
crates.

## Credits

The directory `examples/advanced/riscv_program` is copied from the [upstream repository](https://github.com/libriscv/libriscv).
