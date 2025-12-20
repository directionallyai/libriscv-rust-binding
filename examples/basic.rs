use libriscv::{Machine, Options, SyscallRegistry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} [program file] [arguments ...]", args[0]);
        std::process::exit(1);
    }

    let elf = std::fs::read(&args[1])?;
    let options = Options::builder()
        .max_memory(256 * 1024 * 1024)
        .args(args[1..].iter())
        .build()?;

    let registry = SyscallRegistry::empty();
    let mut machine = Machine::new(elf, options, &registry)?;
    machine.run(u64::MAX)?;
    println!("Program exited with status: {}", machine.return_value());
    Ok(())
}
