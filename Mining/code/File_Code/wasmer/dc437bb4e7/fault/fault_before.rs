pub mod raw {
    use std::ffi::c_void;

    extern "C" {
        pub fn run_on_alternative_stack(stack_end: *mut u64, stack_begin: *mut u64) -> u64;
        pub fn register_preservation_trampoline(); // NOT safe to call directly
        pub fn setjmp(env: *mut c_void) -> i32;
        pub fn longjmp(env: *mut c_void, val: i32) -> !;
    }
}

use crate::codegen::{BreakpointInfo, BreakpointMap};
use crate::state::x64::{build_instance_image, read_stack, X64Register, GPR, XMM};
use crate::state::CodeVersion;
use crate::vm;
use libc::{mmap, mprotect, siginfo_t, MAP_ANON, MAP_PRIVATE, PROT_NONE, PROT_READ, PROT_WRITE};
use nix::sys::signal::{
    sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal, SIGBUS, SIGFPE, SIGILL, SIGINT,
    SIGSEGV, SIGTRAP,
};
use std::any::Any;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::ffi::c_void;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

pub(crate) unsafe fn run_on_alternative_stack(stack_end: *mut u64, stack_begin: *mut u64) -> u64 {
    raw::run_on_alternative_stack(stack_end, stack_begin)
}

const TRAP_STACK_SIZE: usize = 1048576; // 1MB

const SETJMP_BUFFER_LEN: usize = 27;
type SetJmpBuffer = [i32; SETJMP_BUFFER_LEN];

struct UnwindInfo {
    jmpbuf: SetJmpBuffer, // in
    breakpoints: Option<BreakpointMap>,
    payload: Option<Box<dyn Any>>, // out
}

#[repr(packed)]
#[derive(Default, Copy, Clone)]
pub struct BoundaryRegisterPreservation {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
}

thread_local! {
    static UNWIND: UnsafeCell<Option<UnwindInfo>> = UnsafeCell::new(None);
    static CURRENT_CTX: UnsafeCell<*mut vm::Ctx> = UnsafeCell::new(::std::ptr::null_mut());
    static CURRENT_CODE_VERSIONS: RefCell<Vec<CodeVersion>> = RefCell::new(vec![]);
    static WAS_SIGINT_TRIGGERED: Cell<bool> = Cell::new(false);
    static BOUNDARY_REGISTER_PRESERVATION: UnsafeCell<BoundaryRegisterPreservation> = UnsafeCell::new(BoundaryRegisterPreservation::default());
}

#[no_mangle]
pub unsafe extern "C" fn get_boundary_register_preservation() -> *mut BoundaryRegisterPreservation {
    BOUNDARY_REGISTER_PRESERVATION.with(|x| x.get())
}

struct InterruptSignalMem(*mut u8);
unsafe impl Send for InterruptSignalMem {}
unsafe impl Sync for InterruptSignalMem {}

const INTERRUPT_SIGNAL_MEM_SIZE: usize = 4096;

lazy_static! {
    static ref INTERRUPT_SIGNAL_MEM: InterruptSignalMem = {
        let ptr = unsafe {
            mmap(
                ::std::ptr::null_mut(),
                INTERRUPT_SIGNAL_MEM_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANON,
                -1,
                0,
            )
        };
        if ptr as isize == -1 {
            panic!("cannot allocate code memory");
        }
        InterruptSignalMem(ptr as _)
    };
}
static INTERRUPT_SIGNAL_DELIVERED: AtomicBool = AtomicBool::new(false);

pub fn was_sigint_triggered_fault() -> bool {
    WAS_SIGINT_TRIGGERED.with(|x| x.get())
}

pub unsafe fn with_ctx<R, F: FnOnce() -> R>(ctx: *mut vm::Ctx, cb: F) -> R {
    let addr = CURRENT_CTX.with(|x| x.get());
    let old = *addr;
    *addr = ctx;
    let ret = cb();
    *addr = old;
    ret
}

pub fn push_code_version(version: CodeVersion) {
    CURRENT_CODE_VERSIONS.with(|x| x.borrow_mut().push(version));
}

pub fn pop_code_version() -> Option<CodeVersion> {
    CURRENT_CODE_VERSIONS.with(|x| x.borrow_mut().pop())
}

pub unsafe fn get_wasm_interrupt_signal_mem() -> *mut u8 {
    INTERRUPT_SIGNAL_MEM.0
}

pub unsafe fn set_wasm_interrupt_on_ctx(ctx: *mut vm::Ctx) {
    if mprotect(
        (&*ctx).internal.interrupt_signal_mem as _,
        INTERRUPT_SIGNAL_MEM_SIZE,
        PROT_NONE,
    ) < 0
    {
        panic!("cannot set PROT_NONE on signal mem");
    }
}

pub unsafe fn set_wasm_interrupt() {
    let mem: *mut u8 = INTERRUPT_SIGNAL_MEM.0;
    if mprotect(mem as _, INTERRUPT_SIGNAL_MEM_SIZE, PROT_NONE) < 0 {
        panic!("cannot set PROT_NONE on signal mem");
    }
}

pub unsafe fn clear_wasm_interrupt() {
    let mem: *mut u8 = INTERRUPT_SIGNAL_MEM.0;
    if mprotect(mem as _, INTERRUPT_SIGNAL_MEM_SIZE, PROT_READ | PROT_WRITE) < 0 {
        panic!("cannot set PROT_READ | PROT_WRITE on signal mem");
    }
}

pub unsafe fn catch_unsafe_unwind<R, F: FnOnce() -> R>(
    f: F,
    breakpoints: Option<BreakpointMap>,
) -> Result<R, Box<dyn Any>> {
    let unwind = UNWIND.with(|x| x.get());
    let old = (*unwind).take();
    *unwind = Some(UnwindInfo {
        jmpbuf: [0; SETJMP_BUFFER_LEN],
        breakpoints: breakpoints,
        payload: None,
    });

    if raw::setjmp(&mut (*unwind).as_mut().unwrap().jmpbuf as *mut SetJmpBuffer as *mut _) != 0 {
        // error
        let ret = (*unwind).as_mut().unwrap().payload.take().unwrap();
        *unwind = old;
        Err(ret)
    } else {
        let ret = f();
        // implicit control flow to the error case...
        *unwind = old;
        Ok(ret)
    }
}

pub unsafe fn begin_unsafe_unwind(e: Box<dyn Any>) -> ! {
    let unwind = UNWIND.with(|x| x.get());
    let inner = (*unwind)
        .as_mut()
        .expect("not within a catch_unsafe_unwind scope");
    inner.payload = Some(e);
    raw::longjmp(&mut inner.jmpbuf as *mut SetJmpBuffer as *mut _, 0xffff);
}

unsafe fn with_breakpoint_map<R, F: FnOnce(Option<&BreakpointMap>) -> R>(f: F) -> R {
    let unwind = UNWIND.with(|x| x.get());
    let inner = (*unwind)
        .as_mut()
        .expect("not within a catch_unsafe_unwind scope");
    f(inner.breakpoints.as_ref())
}

pub fn allocate_and_run<R, F: FnOnce() -> R>(size: usize, f: F) -> R {
    struct Context<F: FnOnce() -> R, R> {
        f: Option<F>,
        ret: Option<R>,
    }

    extern "C" fn invoke<F: FnOnce() -> R, R>(ctx: &mut Context<F, R>) {
        let f = ctx.f.take().unwrap();
        ctx.ret = Some(f());
    }

    unsafe {
        let mut ctx = Context {
            f: Some(f),
            ret: None,
        };
        assert!(size % 16 == 0);
        assert!(size >= 4096);

        let mut stack: Vec<u64> = vec![0; size / 8];
        let end_offset = stack.len();

        stack[end_offset - 4] = invoke::<F, R> as usize as u64;

        // NOTE: Keep this consistent with `image-loading-*.s`.
        stack[end_offset - 4 - 10] = &mut ctx as *mut Context<F, R> as usize as u64; // rdi
        const NUM_SAVED_REGISTERS: usize = 31;
        let stack_begin = stack
            .as_mut_ptr()
            .offset((end_offset - 4 - NUM_SAVED_REGISTERS) as isize);
        let stack_end = stack.as_mut_ptr().offset(end_offset as isize);

        raw::run_on_alternative_stack(stack_end, stack_begin);
        ctx.ret.take().unwrap()
    }
}

extern "C" fn signal_trap_handler(
    signum: ::nix::libc::c_int,
    siginfo: *mut siginfo_t,
    ucontext: *mut c_void,
) {
    unsafe {
        let fault = get_fault_info(siginfo as _, ucontext);

        let mut unwind_result: Box<dyn Any> = Box::new(());

        let should_unwind = allocate_and_run(TRAP_STACK_SIZE, || {
            let mut is_suspend_signal = false;

            WAS_SIGINT_TRIGGERED.with(|x| x.set(false));

            match Signal::from_c_int(signum) {
                Ok(SIGTRAP) => {
                    // breakpoint
                    let out: Option<Result<(), Box<dyn Any>>> = with_breakpoint_map(|bkpt_map| {
                        bkpt_map.and_then(|x| x.get(&(fault.ip as usize))).map(|x| {
                            x(BreakpointInfo {
                                fault: Some(&fault),
                            })
                        })
                    });
                    match out {
                        Some(Ok(())) => {
                            return false;
                        }
                        Some(Err(e)) => {
                            unwind_result = e;
                            return true;
                        }
                        None => {}
                    }
                }
                Ok(SIGSEGV) | Ok(SIGBUS) => {
                    if fault.faulting_addr as usize == get_wasm_interrupt_signal_mem() as usize {
                        is_suspend_signal = true;
                        clear_wasm_interrupt();
                        if INTERRUPT_SIGNAL_DELIVERED.swap(false, Ordering::SeqCst) {
                            WAS_SIGINT_TRIGGERED.with(|x| x.set(true));
                        }
                    }
                }
                _ => {}
            }

            let ctx: &mut vm::Ctx = &mut **CURRENT_CTX.with(|x| x.get());
            let rsp = fault.known_registers[X64Register::GPR(GPR::RSP).to_index().0].unwrap();

            let es_image = CURRENT_CODE_VERSIONS.with(|versions| {
                let versions = versions.borrow();
                read_stack(
                    || versions.iter(),
                    rsp as usize as *const u64,
                    fault.known_registers,
                    Some(fault.ip as usize as u64),
                )
            });

            if is_suspend_signal {
                let image = build_instance_image(ctx, es_image);
                unwind_result = Box::new(image);
            } else {
                if es_image.frames.len() > 0 {
                    eprintln!(
                        "\n{}",
                        "Wasmer encountered an error while running your WebAssembly program."
                    );
                    es_image.print_backtrace_if_needed();
                }
                // Just let the error propagate otherrwise
            }

            true
        });

        if should_unwind {
            begin_unsafe_unwind(unwind_result);
        }
    }
}

extern "C" fn sigint_handler(
    _signum: ::nix::libc::c_int,
    _siginfo: *mut siginfo_t,
    _ucontext: *mut c_void,
) {
    if INTERRUPT_SIGNAL_DELIVERED.swap(true, Ordering::SeqCst) {
        eprintln!("Got another SIGINT before trap is triggered on WebAssembly side, aborting");
        process::abort();
    }
    unsafe {
        set_wasm_interrupt();
    }
}

pub fn ensure_sighandler() {
    INSTALL_SIGHANDLER.call_once(|| unsafe {
        install_sighandler();
    });
}

static INSTALL_SIGHANDLER: Once = Once::new();

unsafe fn install_sighandler() {
    let sa_trap = SigAction::new(
        SigHandler::SigAction(signal_trap_handler),
        SaFlags::SA_ONSTACK,
        SigSet::empty(),
    );
    sigaction(SIGFPE, &sa_trap).unwrap();
    sigaction(SIGILL, &sa_trap).unwrap();
    sigaction(SIGSEGV, &sa_trap).unwrap();
    sigaction(SIGBUS, &sa_trap).unwrap();
    sigaction(SIGTRAP, &sa_trap).unwrap();

    let sa_interrupt = SigAction::new(
        SigHandler::SigAction(sigint_handler),
        SaFlags::SA_ONSTACK,
        SigSet::empty(),
    );
    sigaction(SIGINT, &sa_interrupt).unwrap();
}

pub struct FaultInfo {
    pub faulting_addr: *const c_void,
    pub ip: *const c_void,
    pub known_registers: [Option<u64>; 32],
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub unsafe fn get_fault_info(siginfo: *const c_void, ucontext: *const c_void) -> FaultInfo {
    use libc::{
        _libc_xmmreg, ucontext_t, REG_R10, REG_R11, REG_R12, REG_R13, REG_R14, REG_R15, REG_R8,
        REG_R9, REG_RAX, REG_RBP, REG_RBX, REG_RCX, REG_RDI, REG_RDX, REG_RIP, REG_RSI, REG_RSP,
    };

    fn read_xmm(reg: &_libc_xmmreg) -> u64 {
        (reg.element[0] as u64) | ((reg.element[1] as u64) << 32)
    }

    #[allow(dead_code)]
    #[repr(C)]
    struct siginfo_t {
        si_signo: i32,
        si_errno: i32,
        si_code: i32,
        si_addr: u64,
        // ...
    }

    let siginfo = siginfo as *const siginfo_t;
    let si_addr = (*siginfo).si_addr;

    let ucontext = ucontext as *const ucontext_t;
    let gregs = &(*ucontext).uc_mcontext.gregs;
    let fpregs = &*(*ucontext).uc_mcontext.fpregs;

    let mut known_registers: [Option<u64>; 32] = [None; 32];
    known_registers[X64Register::GPR(GPR::R15).to_index().0] = Some(gregs[REG_R15 as usize] as _);
    known_registers[X64Register::GPR(GPR::R14).to_index().0] = Some(gregs[REG_R14 as usize] as _);
    known_registers[X64Register::GPR(GPR::R13).to_index().0] = Some(gregs[REG_R13 as usize] as _);
    known_registers[X64Register::GPR(GPR::R12).to_index().0] = Some(gregs[REG_R12 as usize] as _);
    known_registers[X64Register::GPR(GPR::R11).to_index().0] = Some(gregs[REG_R11 as usize] as _);
    known_registers[X64Register::GPR(GPR::R10).to_index().0] = Some(gregs[REG_R10 as usize] as _);
    known_registers[X64Register::GPR(GPR::R9).to_index().0] = Some(gregs[REG_R9 as usize] as _);
    known_registers[X64Register::GPR(GPR::R8).to_index().0] = Some(gregs[REG_R8 as usize] as _);
    known_registers[X64Register::GPR(GPR::RSI).to_index().0] = Some(gregs[REG_RSI as usize] as _);
    known_registers[X64Register::GPR(GPR::RDI).to_index().0] = Some(gregs[REG_RDI as usize] as _);
    known_registers[X64Register::GPR(GPR::RDX).to_index().0] = Some(gregs[REG_RDX as usize] as _);
    known_registers[X64Register::GPR(GPR::RCX).to_index().0] = Some(gregs[REG_RCX as usize] as _);
    known_registers[X64Register::GPR(GPR::RBX).to_index().0] = Some(gregs[REG_RBX as usize] as _);
    known_registers[X64Register::GPR(GPR::RAX).to_index().0] = Some(gregs[REG_RAX as usize] as _);

    known_registers[X64Register::GPR(GPR::RBP).to_index().0] = Some(gregs[REG_RBP as usize] as _);
    known_registers[X64Register::GPR(GPR::RSP).to_index().0] = Some(gregs[REG_RSP as usize] as _);

    known_registers[X64Register::XMM(XMM::XMM0).to_index().0] = Some(read_xmm(&fpregs._xmm[0]));
    known_registers[X64Register::XMM(XMM::XMM1).to_index().0] = Some(read_xmm(&fpregs._xmm[1]));
    known_registers[X64Register::XMM(XMM::XMM2).to_index().0] = Some(read_xmm(&fpregs._xmm[2]));
    known_registers[X64Register::XMM(XMM::XMM3).to_index().0] = Some(read_xmm(&fpregs._xmm[3]));
    known_registers[X64Register::XMM(XMM::XMM4).to_index().0] = Some(read_xmm(&fpregs._xmm[4]));
    known_registers[X64Register::XMM(XMM::XMM5).to_index().0] = Some(read_xmm(&fpregs._xmm[5]));
    known_registers[X64Register::XMM(XMM::XMM6).to_index().0] = Some(read_xmm(&fpregs._xmm[6]));
    known_registers[X64Register::XMM(XMM::XMM7).to_index().0] = Some(read_xmm(&fpregs._xmm[7]));
    known_registers[X64Register::XMM(XMM::XMM8).to_index().0] = Some(read_xmm(&fpregs._xmm[8]));
    known_registers[X64Register::XMM(XMM::XMM9).to_index().0] = Some(read_xmm(&fpregs._xmm[9]));
    known_registers[X64Register::XMM(XMM::XMM10).to_index().0] = Some(read_xmm(&fpregs._xmm[10]));
    known_registers[X64Register::XMM(XMM::XMM11).to_index().0] = Some(read_xmm(&fpregs._xmm[11]));
    known_registers[X64Register::XMM(XMM::XMM12).to_index().0] = Some(read_xmm(&fpregs._xmm[12]));
    known_registers[X64Register::XMM(XMM::XMM13).to_index().0] = Some(read_xmm(&fpregs._xmm[13]));
    known_registers[X64Register::XMM(XMM::XMM14).to_index().0] = Some(read_xmm(&fpregs._xmm[14]));
    known_registers[X64Register::XMM(XMM::XMM15).to_index().0] = Some(read_xmm(&fpregs._xmm[15]));

    FaultInfo {
        faulting_addr: si_addr as usize as _,
        ip: gregs[REG_RIP as usize] as _,
        known_registers,
    }
}

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub unsafe fn get_fault_info(siginfo: *const c_void, ucontext: *const c_void) -> FaultInfo {
    #[allow(dead_code)]
    #[repr(C)]
    struct ucontext_t {
        uc_onstack: u32,
        uc_sigmask: u32,
        uc_stack: libc::stack_t,
        uc_link: *const ucontext_t,
        uc_mcsize: u64,
        uc_mcontext: *const mcontext_t,
    }
    #[repr(C)]
    struct exception_state {
        trapno: u16,
        cpu: u16,
        err: u32,
        faultvaddr: u64,
    }
    #[repr(C)]
    struct regs {
        rax: u64,
        rbx: u64,
        rcx: u64,
        rdx: u64,
        rdi: u64,
        rsi: u64,
        rbp: u64,
        rsp: u64,
        r8: u64,
        r9: u64,
        r10: u64,
        r11: u64,
        r12: u64,
        r13: u64,
        r14: u64,
        r15: u64,
        rip: u64,
        rflags: u64,
        cs: u64,
        fs: u64,
        gs: u64,
    }
    #[repr(C)]
    struct fpstate {
        _cwd: u16,
        _swd: u16,
        _ftw: u16,
        _fop: u16,
        _rip: u64,
        _rdp: u64,
        _mxcsr: u32,
        _mxcr_mask: u32,
        _st: [[u16; 8]; 8],
        xmm: [[u64; 2]; 16],
        _padding: [u32; 24],
    }
    #[allow(dead_code)]
    #[repr(C)]
    struct mcontext_t {
        es: exception_state,
        ss: regs,
        fs: fpstate,
    }

    let siginfo = siginfo as *const siginfo_t;
    let si_addr = (*siginfo).si_addr;

    let ucontext = ucontext as *const ucontext_t;
    let ss = &(*(*ucontext).uc_mcontext).ss;
    let fs = &(*(*ucontext).uc_mcontext).fs;

    let mut known_registers: [Option<u64>; 32] = [None; 32];

    known_registers[X64Register::GPR(GPR::R15).to_index().0] = Some(ss.r15);
    known_registers[X64Register::GPR(GPR::R14).to_index().0] = Some(ss.r14);
    known_registers[X64Register::GPR(GPR::R13).to_index().0] = Some(ss.r13);
    known_registers[X64Register::GPR(GPR::R12).to_index().0] = Some(ss.r12);
    known_registers[X64Register::GPR(GPR::R11).to_index().0] = Some(ss.r11);
    known_registers[X64Register::GPR(GPR::R10).to_index().0] = Some(ss.r10);
    known_registers[X64Register::GPR(GPR::R9).to_index().0] = Some(ss.r9);
    known_registers[X64Register::GPR(GPR::R8).to_index().0] = Some(ss.r8);
    known_registers[X64Register::GPR(GPR::RSI).to_index().0] = Some(ss.rsi);
    known_registers[X64Register::GPR(GPR::RDI).to_index().0] = Some(ss.rdi);
    known_registers[X64Register::GPR(GPR::RDX).to_index().0] = Some(ss.rdx);
    known_registers[X64Register::GPR(GPR::RCX).to_index().0] = Some(ss.rcx);
    known_registers[X64Register::GPR(GPR::RBX).to_index().0] = Some(ss.rbx);
    known_registers[X64Register::GPR(GPR::RAX).to_index().0] = Some(ss.rax);

    known_registers[X64Register::GPR(GPR::RBP).to_index().0] = Some(ss.rbp);
    known_registers[X64Register::GPR(GPR::RSP).to_index().0] = Some(ss.rsp);

    known_registers[X64Register::XMM(XMM::XMM0).to_index().0] = Some(fs.xmm[0][0]);
    known_registers[X64Register::XMM(XMM::XMM1).to_index().0] = Some(fs.xmm[1][0]);
    known_registers[X64Register::XMM(XMM::XMM2).to_index().0] = Some(fs.xmm[2][0]);
    known_registers[X64Register::XMM(XMM::XMM3).to_index().0] = Some(fs.xmm[3][0]);
    known_registers[X64Register::XMM(XMM::XMM4).to_index().0] = Some(fs.xmm[4][0]);
    known_registers[X64Register::XMM(XMM::XMM5).to_index().0] = Some(fs.xmm[5][0]);
    known_registers[X64Register::XMM(XMM::XMM6).to_index().0] = Some(fs.xmm[6][0]);
    known_registers[X64Register::XMM(XMM::XMM7).to_index().0] = Some(fs.xmm[7][0]);
    known_registers[X64Register::XMM(XMM::XMM8).to_index().0] = Some(fs.xmm[8][0]);
    known_registers[X64Register::XMM(XMM::XMM9).to_index().0] = Some(fs.xmm[9][0]);
    known_registers[X64Register::XMM(XMM::XMM10).to_index().0] = Some(fs.xmm[10][0]);
    known_registers[X64Register::XMM(XMM::XMM11).to_index().0] = Some(fs.xmm[11][0]);
    known_registers[X64Register::XMM(XMM::XMM12).to_index().0] = Some(fs.xmm[12][0]);
    known_registers[X64Register::XMM(XMM::XMM13).to_index().0] = Some(fs.xmm[13][0]);
    known_registers[X64Register::XMM(XMM::XMM14).to_index().0] = Some(fs.xmm[14][0]);
    known_registers[X64Register::XMM(XMM::XMM15).to_index().0] = Some(fs.xmm[15][0]);

    FaultInfo {
        faulting_addr: si_addr,
        ip: ss.rip as _,
        known_registers,
    }
}
