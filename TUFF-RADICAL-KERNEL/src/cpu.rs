use core::arch::x86_64::__cpuid;
use crate::serial_println;

#[derive(Debug, Default)]
pub struct CpuFeatures {
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512f: bool,
    pub has_vaes: bool,
    pub has_rdrand: bool,
}

pub fn detect_features() -> CpuFeatures {
    let mut features = CpuFeatures::default();

    // Leaf 1: Basic features
    let res1 = __cpuid(1);
    features.has_avx = (res1.ecx & (1 << 28)) != 0;
    features.has_rdrand = (res1.ecx & (1 << 30)) != 0;

    // Leaf 7, Subleaf 0: Extended features
    let res7 = __cpuid(7);
    features.has_avx2 = (res7.ebx & (1 << 5)) != 0;
    features.has_avx512f = (res7.ebx & (1 << 16)) != 0;
    features.has_vaes = (res7.ecx & (1 << 9)) != 0;

    features
}

pub fn log_features(f: &CpuFeatures) {
    serial_println!("TUFF-RADICAL-COMMANDER [CPU-01]: Feature Detection Result:");
    serial_println!("  AVX:      {}", if f.has_avx { "SUPPORTED" } else { "MISSING" });
    serial_println!("  AVX2:     {}", if f.has_avx2 { "SUPPORTED" } else { "MISSING" });
    serial_println!("  AVX-512F: {}", if f.has_avx512f { "SUPPORTED" } else { "MISSING" });
    serial_println!("  VAES:     {}", if f.has_vaes { "SUPPORTED" } else { "MISSING" });
    serial_println!("  RDRAND:   {}", if f.has_rdrand { "SUPPORTED" } else { "MISSING" });
}
