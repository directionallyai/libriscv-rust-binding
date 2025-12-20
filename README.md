# `libriscv`

Safe Rust wrapper around libriscv_sys, a fast RISC-V sandbox emulator.

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

See `examples/` for more usage patterns.

## Credits

The directory `examples/advanced/riscv_program` is copied from the [upstream repository](https://github.com/libriscv/libriscv).
