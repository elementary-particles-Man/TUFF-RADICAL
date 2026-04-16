use core::arch::x86_64::{__cpuid};
use crate::serial_println;

#[derive(Debug, Default)]
pub struct CpuFeatures {
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512f: bool,
    pub has_vaes: bool,
    pub has_rdrand: bool,
    pub has_xsave: bool,
    pub logical_threads: u32,
}

pub fn detect_features() -> CpuFeatures {
    let mut features = CpuFeatures::default();

    // Leaf 1: Basic features and thread count
    let res1 = __cpuid(1);
    features.has_avx = (res1.ecx & (1 << 28)) != 0;
    features.has_xsave = (res1.ecx & (1 << 26)) != 0;
    features.has_rdrand = (res1.ecx & (1 << 30)) != 0;

    features.logical_threads = (res1.ebx >> 16) & 0xFF;
    if features.logical_threads == 0 {
        features.logical_threads = 1;
    }

    // Leaf 7, Subleaf 0: Extended features
    let res7 = __cpuid(7);
    features.has_avx2 = (res7.ebx & (1 << 5)) != 0;
    features.has_avx512f = (res7.ebx & (1 << 16)) != 0;
    features.has_vaes = (res7.ecx & (1 << 9)) != 0;

    features
}

#[allow(dead_code)]
pub unsafe fn enable_simd_hardware() {
    let features = detect_features();
    if !features.has_xsave {
        serial_println!("TUFF-RADICAL [CPU-ERR]: XSAVE NOT SUPPORTED. ABORTING SIMD OPTIMIZATION.");
        return;
    }

    // Enable OSXSAVE in CR4
    let mut cr4: u64;
    core::arch::asm!("mov {}, cr4", out(reg) cr4);
    cr4 |= 1 << 18; // OSXSAVE bit
    core::arch::asm!("mov cr4, {}", in(reg) cr4);

    // XCR0 (XFEATURE_ENABLED_MASK) configuration
    // Enable x87 (bit 0), SSE (bit 1), AVX (bit 2)
    let mut xcr0: u64 = 0x1 | 0x2 | 0x4;
    if features.has_avx512f {
        // Enable Opmask (bit 5), ZMM_Hi256 (bit 6), Hi16_ZMM (bit 7)
        xcr0 |= 0x20 | 0x40 | 0x80;
    }

    // Write to XCR0 using xsetbv (ECX=0)
    let eax = (xcr0 & 0xFFFFFFFF) as u32;
    let edx = (xcr0 >> 32) as u32;

    core::arch::asm!(
        "xor rcx, rcx",
        "xsetbv",
        in("eax") eax,
        in("edx") edx,
    );

    serial_println!("TUFF-RADICAL [CPU-01]: SIMD Hardware (XCR0={:#x}) Enabled.", xcr0);
}

pub fn log_features(f: &CpuFeatures) {
    serial_println!("  LOGICAL THREADS: {}", f.logical_threads);
    serial_println!("  AVX:      {}", if f.has_avx { "SUPPORTED" } else { "MISSING (Sub-optimal SIMD)" });
    serial_println!("  AVX2:     {}", if f.has_avx2 { "SUPPORTED" } else { "MISSING" });
    serial_println!("  AVX-512F: {}", if f.has_avx512f { "SUPPORTED" } else { "MISSING" });
    serial_println!("  VAES:     {}", if f.has_vaes { "SUPPORTED" } else { "MISSING" });
    serial_println!("  RDRAND:   {}", if f.has_rdrand { "SUPPORTED" } else { "MISSING" });
}
