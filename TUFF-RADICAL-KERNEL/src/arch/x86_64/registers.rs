use core::arch::asm;

pub struct Cr3;
impl Cr3 {
    #[allow(dead_code)]
    pub unsafe fn read() -> u64 {
        let val: u64;
        asm!("mov {}, cr3", out(reg) val, options(nomem, nostack, preserves_flags));
        val
    }
    pub unsafe fn write(val: u64) {
        asm!("mov cr3, {}", in(reg) val, options(nomem, nostack, preserves_flags));
    }
}

pub struct Cr0;
impl Cr0 {
    #[allow(dead_code)]
    pub unsafe fn read() -> u64 {
        let val: u64;
        asm!("mov {}, cr0", out(reg) val, options(nomem, nostack, preserves_flags));
        val
    }
    pub unsafe fn write(val: u64) {
        asm!("mov cr0, {}", in(reg) val, options(nomem, nostack, preserves_flags));
    }
}

pub struct Cr4;
impl Cr4 {
    #[allow(dead_code)]
    pub unsafe fn read() -> u64 {
        let val: u64;
        asm!("mov {}, cr4", out(reg) val, options(nomem, nostack, preserves_flags));
        val
    }
    pub unsafe fn write(val: u64) {
        asm!("mov cr4, {}", in(reg) val, options(nomem, nostack, preserves_flags));
    }
}

pub struct Msr {
    reg: u32,
}
impl Msr {
    pub const fn new(reg: u32) -> Self {
        Self { reg }
    }
    pub unsafe fn read(&self) -> u64 {
        let low: u32;
        let high: u32;
        asm!("rdmsr", in("ecx") self.reg, out("eax") low, out("edx") high, options(nomem, nostack, preserves_flags));
        ((high as u64) << 32) | (low as u64)
    }
    pub unsafe fn write(&self, val: u64) {
        let low = val as u32;
        let high = (val >> 32) as u32;
        asm!("wrmsr", in("ecx") self.reg, in("eax") low, in("edx") high, options(nomem, nostack, preserves_flags));
    }
}

pub const EFER: Msr = Msr::new(0xC0000080);
#[allow(dead_code)]
pub const FS_BASE: Msr = Msr::new(0xC0000100);
#[allow(dead_code)]
pub const GS_BASE: Msr = Msr::new(0xC0000101);
#[allow(dead_code)]
pub const KERNEL_GS_BASE: Msr = Msr::new(0xC0000102);

pub unsafe fn xgetbv(index: u32) -> u64 {
    let eax: u32;
    let edx: u32;
    asm!(
        "xgetbv",
        in("ecx") index,
        out("eax") eax,
        out("edx") edx,
        options(nomem, nostack)
    );
    ((edx as u64) << 32) | eax as u64
}

pub unsafe fn xsetbv(index: u32, value: u64) {
    asm!(
        "xsetbv",
        in("ecx") index,
        in("eax") value as u32,
        in("edx") (value >> 32) as u32,
        options(nomem, nostack)
    );
}


#[allow(dead_code)]
pub struct GsBase;
#[allow(dead_code)]
impl GsBase {
    pub unsafe fn write_kernel(val: u64) {
        GS_BASE.write(val);
    }
    pub unsafe fn write_user(val: u64) {
        KERNEL_GS_BASE.write(val);
    }
}
