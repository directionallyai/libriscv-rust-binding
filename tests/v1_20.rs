use libriscv::{Error, Machine, Options, PageAttributes, SyscallRegistry, sys};
use std::os::raw::{c_int, c_void};
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_tests() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Default)]
struct ArenaCallbackState {
    free_calls: u64,
    realloc_calls: u64,
}

unsafe extern "C" fn unknown_free(_address: u64, user: *mut c_void) -> c_int {
    let state = unsafe { &mut *user.cast::<ArenaCallbackState>() };
    state.free_calls += 1;
    0
}

unsafe extern "C" fn unknown_realloc(
    _address: u64,
    _new_size: u64,
    user: *mut c_void,
) -> sys::RISCVReallocResult {
    let state = unsafe { &mut *user.cast::<ArenaCallbackState>() };
    state.realloc_calls += 1;
    sys::RISCVReallocResult {
        ptr: 0x1234,
        old_size: 32,
    }
}

#[test]
fn creates_and_fast_forks_a_machine() {
    let _guard = lock_tests();
    let registry = SyscallRegistry::empty();
    let elf = include_bytes!("fixtures/fib.rv64.elf");
    let parent = Machine::new(elf, Options::default(), &registry).unwrap();

    assert!(!parent.is_forked());
    let child = parent.fast_fork(Options::default()).unwrap();
    assert!(child.is_forked());
}

#[test]
fn exposes_v1_20_machine_state() {
    let _guard = lock_tests();
    let registry = SyscallRegistry::empty();
    let elf = include_bytes!("fixtures/fib.rv64.elf");
    let options = Options::builder()
        .use_memory_arena(true)
        .use_shared_execute_segments(true)
        .load_program(true)
        .protect_segments(true)
        .native_syscall_base(0)
        .arena_size(8 << 20)
        .build()
        .unwrap();
    let mut machine = Machine::new(elf, options, &registry).unwrap();

    assert!(!machine.is_forked());
    assert!(!machine.instruction_limit_reached());
    assert!(machine.heap_address() > 0);
    assert!(machine.initial_stack_pointer() > 0);
    *machine.max_instruction_counter().unwrap() = u64::MAX;
    assert!(machine.step(false).unwrap().is_some());
}

#[test]
fn manages_native_arena_and_callbacks() {
    let _guard = lock_tests();
    let registry = SyscallRegistry::empty();
    let elf = include_bytes!("fixtures/fib.rv64.elf");
    let mut machine = Machine::new(elf, Options::default(), &registry).unwrap();
    let arena_size = 64 << 10;
    let arena_address = machine.mmap_allocate(arena_size).unwrap();

    machine
        .setup_arena(480, arena_address.get(), arena_size)
        .unwrap();
    assert!(machine.has_arena());

    let allocation = machine.arena_malloc(64).unwrap();
    assert!(machine.arena_allocation_size(allocation.get()) >= 64);
    let reallocated = machine.arena_realloc(allocation.get(), 128);
    let reallocated_address = reallocated.address.unwrap();
    assert!(machine.arena_allocation_size(reallocated_address.get()) >= 128);
    machine.arena_free(reallocated_address.get()).unwrap();

    let mut callback_state = ArenaCallbackState::default();
    let user = (&mut callback_state as *mut ArenaCallbackState).cast();
    unsafe {
        machine.set_arena_unknown_free_handler(unknown_free, user);
        machine.set_arena_unknown_realloc_handler(unknown_realloc, user);
    }

    machine.arena_free(0xdead_beef).unwrap();
    let callback_result = machine.arena_realloc(0xdead_beef, 128);
    assert_eq!(callback_result.address.unwrap().get(), 0x1234);
    assert_eq!(callback_result.old_size, 32);
    assert_eq!(callback_state.free_calls, 1);
    assert_eq!(callback_state.realloc_calls, 1);
}

#[test]
fn inserts_non_owned_page_memory() {
    let _guard = lock_tests();
    let mut page = Box::new([0u8; sys::RISCV_PAGE_SIZE as usize]);
    page[0] = 0x2a;

    let registry = SyscallRegistry::empty();
    let elf = include_bytes!("fixtures/fib.rv64.elf");
    let mut machine = Machine::new(elf, Options::default(), &registry).unwrap();
    let destination = machine
        .mmap_allocate(u64::from(sys::RISCV_PAGE_SIZE))
        .unwrap()
        .get();
    let attributes = PageAttributes {
        read: true,
        write: true,
        ..PageAttributes::default()
    };

    let error = unsafe {
        machine.insert_non_owned_memory(
            destination + 1,
            page.as_mut_ptr().cast(),
            u64::from(sys::RISCV_PAGE_SIZE),
            attributes,
        )
    }
    .unwrap_err();
    assert!(matches!(error, Error::UnalignedPageRange { .. }));

    unsafe {
        machine
            .insert_non_owned_memory(
                destination,
                page.as_mut_ptr().cast(),
                u64::from(sys::RISCV_PAGE_SIZE),
                attributes,
            )
            .unwrap();
    }
    assert_eq!(machine.read_memory(destination, 1).unwrap(), [0x2a]);
    machine.write_memory(destination + 1, &[0x7f]).unwrap();
    assert_eq!(page[1], 0x7f);

    {
        let (parent_page, parent_attributes) = machine
            .parent_page(destination / u64::from(sys::RISCV_PAGE_SIZE))
            .unwrap();
        assert_eq!(parent_page[0], 0x2a);
        assert!(parent_attributes.non_owning);
    }

    let mut child = machine.fast_fork(Options::default()).unwrap();
    assert_eq!(child.read_memory(destination, 2).unwrap(), [0x2a, 0x7f]);
    child.write_memory(destination, &[0x55]).unwrap();
    assert_eq!(child.read_memory(destination, 1).unwrap(), [0x55]);
    assert_eq!(page[0], 0x2a);
}
