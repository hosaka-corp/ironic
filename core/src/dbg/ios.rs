//! Logging for IOS syscalls.
//!
//! Note that a lot of this depends on the version being booted.
//! For now, it's sufficient to just assume we're on IOS58.

extern crate pretty_hex;
use pretty_hex::*;
use crate::cpu::Cpu;
use crate::cpu::reg::Reg;
use crate::cpu::mmu::prim::{Access, TLBReq};

/// NOTE: `skyeye-starlet` does something like this; wonder if there's a
/// better way of keeping track of the threads?
#[derive(Debug)]
pub enum ExecutionCtx {
    UNK,
    CRY,
    ES,
    FS,
    KRN,
}
impl From<u32> for ExecutionCtx {
    fn from(pc: u32) -> Self {
        use ExecutionCtx::*;
        match (pc & 0xffff_0000) >> 16 {
            0x1386 => CRY,
            0x2000 => FS,
            0x2010 => ES,
            0xffff => KRN,
            _ => UNK,
        }
    }
}


/// Typed arguments to a syscall. 
pub enum ArgType { Ptr, StrPtr, Int, Uint }

/// Format arguments for some IOS syscall.
pub struct SyscallDef {
    pub name: &'static str,
    pub arg: &'static [ArgType],
}

/// Shorthand for declaring a syscall definition.
macro_rules! scdef {
    ($name:literal, $($arg:ident),*) => {
        SyscallDef { name: $name, arg: &[$(ArgType::$arg,)*] } 
    }
}

pub fn get_syscall_desc(idx: u32) -> SyscallDef {
    match idx {
        0x02 => scdef!("ThreadCancel", ),
        0x04 => scdef!("ThreadGetPid", ),
        0x09 => scdef!("ThreadSetPrio", Int, Int),
        0x0a => scdef!("MqueueCreate", Ptr, Int),
        0x0b => scdef!("MqueueDestroy", Ptr),
        //0x0e => scdef!("MqueueRecv", Ptr, Uint),
        0x0f => scdef!("MqueueRegisterHandler", Int, Int, Uint),
        //0x10 => scdef!("MqueueDestroyHandler", Ptr, Ptr, Ptr),
        0x11 => scdef!("TimerCreate", Int, Int, Int, Uint),
        0x18 => scdef!("HeapAlloc", Int, Uint),
        0x1c => scdef!("Open", StrPtr, Int),
        0x1b => scdef!("RegisterDevice", StrPtr, Int),
        0x2a => scdef!("ResourceReply", Ptr, Uint),
        0x2b => scdef!("SetUid", Int),
        0x2d => scdef!("SetGid", Int),
        0x2f => scdef!("AhbMemFlush", Int),
        0x30 => scdef!("CcAhbMemFlush", Int),
        0x3f => scdef!("SyncBeforeRead", Ptr),
        0x4f => scdef!("VirtToPhys", Ptr),
        0x63 => scdef!("IoscGetData", Uint, Uint, Uint),
        0x6d => scdef!("IoscGenBlockMac", Uint, Uint, Uint),
        _ => panic!("Couldn't resolve syscall idx={:02x}", idx),
    }
}

/// Read a NUL-terminated string from memory.  
/// 
/// NOTE: This is not particularly rigorous or safe.
pub fn read_string(cpu: &Cpu, ptr: u32) -> String {
    let paddr = cpu.mmu.translate(TLBReq::new(ptr, Access::Debug));

    let mut line_buf = [0u8; 32];
    cpu.mmu.bus.write().unwrap().dma_read(paddr, &mut line_buf);
    //println!("{:?}", line_buf.hex_dump());

    let mut end: Option<usize> = None;
    for (i, b) in line_buf.iter().enumerate() {
        if *b == 0x00 { end = Some(i); break; } 
    }
    let s = if end.is_some() {
        std::str::from_utf8(&line_buf[..=end.unwrap()]).unwrap()
    } else {
        std::str::from_utf8(&line_buf).unwrap()
    };
    s.to_string()
}


/// Resolve information about an IOS syscall and its arguments.
pub fn resolve_syscall(cpu: &mut Cpu, opcd: u32) {
    // Get the syscall index (and ignore some
    let idx = (opcd & 0x00ff_ffe0) >> 5;
    match idx { 
        0x0e | // MqueueRecv
        0x2f | // AhbMemFlush
        0x30 | // CcAhbMemFlush
        0x3f | // SyncBeforeRead
        0x4f => return, // VirtToPhys
        _ => { },
    }

    //if idx == 0x02 { cpu.dbg_on = false; }
    //if idx == 0x6d { cpu.dbg_on = true; }

    let syscall = get_syscall_desc(idx);
    let mut arg_buf = String::new();
    for (idx, arg) in syscall.arg.iter().enumerate() {
        let val = cpu.reg[idx as u32];
        match arg {
            ArgType::Ptr => { 
                arg_buf.push_str(&format!("0x{:08x}", val));
            },
            ArgType::StrPtr => {
                let s = read_string(cpu, val);
                arg_buf.push_str(&format!("\"{}\"", s));
            },
            ArgType::Int => {
                arg_buf.push_str(&format!("{}", val));
            },
            ArgType::Uint => {
                arg_buf.push_str(&format!("0x{:x}", val));
            },
        }
        if idx < syscall.arg.len()-1 {
            arg_buf.push_str(", ");
        }
    }
    println!("IOS [{:?}] {}({}) (lr={:08x})", 
        ExecutionCtx::from(cpu.read_fetch_pc()),
        syscall.name, arg_buf, cpu.reg[Reg::Lr]
    );
}
