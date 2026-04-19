use crate::arch::x86_64::registers::{Cr0, Cr4, xgetbv, xsetbv};
use core::arch::x86_64::{__cpuid, __cpuid_count};

#[derive(Debug, Default, Clone, Copy)]
pub struct CpuFeatures {
    pub has_sse: bool,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512f: bool,
    pub has_vaes: bool,
    pub has_rdrand: bool,
    pub has_xsave: bool,
    pub has_osxsave: bool,
    pub logical_threads: u32,
    pub recommended_workers: u32,
    pub simd_enabled: bool,
    pub avx_enabled: bool,
    pub avx512_enabled: bool,
    pub xcr0: u64,
    pub xsave_size: u32,
}

pub fn detect_features() -> CpuFeatures {
    let mut features = CpuFeatures::default();
    let max_leaf = __cpuid(0).eax;

    // Leaf 1: Basic features and thread count
    let res1 = __cpuid(1);
    features.has_sse = (res1.edx & (1 << 25)) != 0;
    features.has_avx = (res1.ecx & (1 << 28)) != 0;
    features.has_xsave = (res1.ecx & (1 << 26)) != 0;
    features.has_osxsave = (res1.ecx & (1 << 27)) != 0;
    features.has_rdrand = (res1.ecx & (1 << 30)) != 0;
    features.logical_threads = detect_logical_threads(max_leaf, res1.ebx);
    
    // XSAVE size detection (EAX=0Dh, ECX=0)
    if features.has_xsave {
        let res13 = __cpuid_count(0x0D, 0);
        features.xsave_size = res13.ebx; // Size required by currently enabled features in XCR0
    }

    // Leaf 7, Subleaf 0: Extended features
    if max_leaf >= 7 {
        let res7 = __cpuid_count(7, 0);
        features.has_avx2 = (res7.ebx & (1 << 5)) != 0;
        features.has_avx512f = (res7.ebx & (1 << 16)) != 0;
        features.has_vaes = (res7.ecx & (1 << 9)) != 0;
    }

    features.recommended_workers = features.logical_threads.clamp(1, 8);

    features
}

pub unsafe fn init_simd() -> CpuFeatures {
    let mut features = detect_features();

    match enable_simd_hardware(&mut features) {
        Ok(()) => {
            serial_println!(
                "TUFF-RADICAL [CPU-01]: SIMD runtime enabled (XCR0={:#x}).",
                features.xcr0
            );
        }
        Err(err) => {
            serial_println!("TUFF-RADICAL [CPU-WARN]: SIMD runtime not enabled: {}", err);
        }
    }

    features
}

unsafe fn enable_simd_hardware(features: &mut CpuFeatures) -> Result<(), &'static str> {
    if !features.has_sse {
        return Err("SSE not supported");
    }
    if !features.has_xsave {
        return Err("XSAVE not supported");
    }
    if !features.has_osxsave {
        return Err("OSXSAVE not supported");
    }

    let mut cr0 = Cr0::read();
    let mut cr4 = Cr4::read();

    // CR0.MP=1, CR0.EM=0, CR4.OSFXSR=1, CR4.OSXMMEXCPT=1, CR4.OSXSAVE=1
    cr0 |= 1 << 1;
    cr0 &= !(1 << 2);
    cr4 |= (1 << 9) | (1 << 10) | (1 << 18);

    Cr0::write(cr0);
    Cr4::write(cr4);

    let mut xcr0 = xgetbv(0) | 0x1 | 0x2;
    if features.has_avx {
        xcr0 |= 0x4;
        features.avx_enabled = true;
    }
    if features.avx_enabled && features.has_avx512f {
        xcr0 |= 0x20 | 0x40 | 0x80;
        features.avx512_enabled = true;
    }

    xsetbv(0, xcr0);
    features.xcr0 = xgetbv(0);
    features.simd_enabled = true;

    Ok(())
}

pub fn log_features(f: &CpuFeatures) {
    serial_println!("  LOGICAL THREADS: {}", f.logical_threads);
    serial_println!("  WORKER TARGET: {}", f.recommended_workers);
    serial_println!("  SSE:      {}", if f.has_sse { "SUPPORTED" } else { "MISSING" });
    serial_println!("  AVX:      {}", if f.has_avx { "SUPPORTED" } else { "MISSING (Sub-optimal SIMD)" });
    serial_println!("  AVX2:     {}", if f.has_avx2 { "SUPPORTED" } else { "MISSING" });
    serial_println!("  AVX-512F: {}", if f.has_avx512f { "SUPPORTED" } else { "MISSING" });
    serial_println!("  VAES:     {}", if f.has_vaes { "SUPPORTED" } else { "MISSING" });
    serial_println!("  RDRAND:   {}", if f.has_rdrand { "SUPPORTED" } else { "MISSING" });
    serial_println!("  XSAVE:    {}", if f.has_xsave { "SUPPORTED" } else { "MISSING" });
    serial_println!("  OSXSAVE:  {}", if f.has_osxsave { "SUPPORTED" } else { "MISSING" });
    serial_println!("  SIMD ON:  {}", if f.simd_enabled { "YES" } else { "NO" });
    serial_println!("  AVX ON:   {}", if f.avx_enabled { "YES" } else { "NO" });
    serial_println!("  AVX512 ON:{}", if f.avx512_enabled { "YES" } else { "NO" });
}

fn detect_logical_threads(max_leaf: u32, fallback_ebx: u32) -> u32 {
    if max_leaf >= 0x1F {
        let threads = topology_threads_from_leaf(0x1F);
        if threads != 0 {
            return threads;
        }
    }

    if max_leaf >= 0x0B {
        let threads = topology_threads_from_leaf(0x0B);
        if threads != 0 {
            return threads;
        }
    }

    let legacy = (fallback_ebx >> 16) & 0xFF;
    legacy.max(1)
}

fn topology_threads_from_leaf(leaf: u32) -> u32 {
    let mut subleaf = 0;
    let mut threads = 0;

    loop {
        let res = __cpuid_count(leaf, subleaf);
        if res.ebx == 0 {
            break;
        }

        let level_type = (res.ecx >> 8) & 0xFF;
        if level_type == 1 || level_type == 2 {
            threads = res.ebx & 0xFFFF;
        }

        subleaf += 1;
    }

    threads.max(1)
}





#[repr(C, align(64))]
#[allow(dead_code)]
pub struct SimdState {
    pub buffer: [u8; 4096], // Max size for AVX-512 is around 2.5KB, 4KB is safe and aligned
}

impl SimdState {
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self { buffer: [0u8; 4096] }
    }

    #[allow(dead_code)]
    pub unsafe fn save(&mut self) {
        core::arch::asm!(
            "xsave [{}]",
            in(reg) self.buffer.as_mut_ptr(),
            in("eax") 0xFFFFFFFF_u32,
            in("edx") 0xFFFFFFFF_u32,
            options(nomem, nostack)
        );
    }

    #[allow(dead_code)]
    pub unsafe fn restore(&self) {
        core::arch::asm!(
            "xrstor [{}]",
            in(reg) self.buffer.as_ptr(),
            in("eax") 0xFFFFFFFF_u32,
            in("edx") 0xFFFFFFFF_u32,
            options(nomem, nostack)
        );
    }
}
