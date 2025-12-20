use libriscv::{Machine, Options, Registers, Result};

fn reserve_stack(regs: &mut Registers<'_>, size: usize) -> Result<u64> {
    let sp = regs.x(2)?;
    let new_sp = sp.wrapping_sub(size as u64) & !0xFu64;
    regs.set_x(2, new_sp)?;
    Ok(new_sp)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} [program file] [arguments ...]", args[0]);
        std::process::exit(1);
    }

    let elf = std::fs::read(&args[1])?;
    let options = Options::builder()
        .max_memory(64 * 1024 * 1024)
        .args(args[1..].iter())
        .build()?;

    let mut machine = Machine::new(elf, options)?;
    machine.run(32_000_000)?;

    if let Some(addr) = machine.address_of("my_function")? {
        machine.setup_vmcall(addr)?;
        let msg = b"Hello Sandboxed World!\0";
        let str_addr = {
            let mut regs = machine.registers()?;
            reserve_stack(&mut regs, msg.len())?
        };
        machine.copy_to_guest(str_addr, msg)?;
        {
            let mut regs = machine.registers()?;
            regs.set_x(10, str_addr)?;
        }
        machine.run(u64::MAX)?;
    } else {
        eprintln!("Symbol my_function not found");
    }

    println!("Program exited with status: {}", machine.return_value());
    Ok(())
}
