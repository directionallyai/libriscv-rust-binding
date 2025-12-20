# Advanced Guest Program (riscv_program)

This directory mirrors the upstream `examples/advanced/riscv_program` guest code so the Rust
example `examples/advanced_syscalls.rs` can run a real RISC-V program.

## Build

You need a RISC-V GCC toolchain that provides `riscv64-linux-gnu-gcc` (or
`riscv64-linux-gnu-gcc-12`). If your toolchain uses a different prefix, set `CC`.
On macOS, the build script uses `zig cc` instead (set `ZIG` if needed). The
default target is `riscv64-linux-musl` for a static binary.

```sh
cd examples/advanced/riscv_program
./build.sh
```

This produces `micro` (a RISC-V ELF).

## Rust guest program

The Rust equivalent lives in `examples/advanced/rust_program`.

```sh
cd examples/advanced/rust_program
./build.sh
```

This builds `target/riscv64gc-unknown-linux-musl/release/riscv_micro` on macOS
(`riscv64gc-unknown-linux-gnu` on Linux by default).
If you prefer a different target or linker, set `TARGET` and `LINKER`:

```sh
TARGET=riscv64gc-unknown-linux-musl ./build.sh
```

On macOS, install `zig` and add the Rust target before building:

```sh
rustup target add riscv64gc-unknown-linux-musl
```

## Run with the Rust host example

```sh
cargo run --example advanced_syscalls -- examples/advanced/riscv_program/micro
```

Or run the Rust guest build:

```sh
cargo run --example advanced_syscalls -- examples/advanced/rust_program/target/riscv64gc-unknown-linux-gnu/release/riscv_micro
```
