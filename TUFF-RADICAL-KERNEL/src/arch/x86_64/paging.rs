use crate::arch::x86_64::registers::{Cr3, EFER};
use crate::mm::memory;


/// ページテーブルのエントリ属性
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const HUGE_PAGE: u64 = 1 << 7; 

pub const NORMAL_MEMORY: u64 = PRESENT | WRITABLE;

pub unsafe fn init_paging() {
    serial_println!("TUFF-RADICAL-COMMANDER [PAG-OPT]: Enabling NXE capability and rebuilding identity map...");

    // CPU の NXE (No-Execute Enable) ビットを有効化
    enable_nx_bit();

    let pml4_phys = memory::allocate_page().expect("Failed to allocate PML4");
    let pml4 = pml4_phys as *mut u64;
    for i in 0..512 { *pml4.add(i) = 0; }

    let pdpt_phys = memory::allocate_page().expect("Failed to allocate PDPT");
    let pdpt = pdpt_phys as *mut u64;
    for i in 0..512 { *pdpt.add(i) = 0; }

    *pml4.add(0) = pdpt_phys | NORMAL_MEMORY;

    for pdpt_idx in 0..4 {
        let pd_phys = memory::allocate_page().expect("Failed to allocate PD");
        let pd = pd_phys as *mut u64;
        for i in 0..512 { *pd.add(i) = 0; }

        *pdpt.add(pdpt_idx) = pd_phys | NORMAL_MEMORY;

        for pd_idx in 0..512 {
            let phys_addr = (pdpt_idx as u64 * 1024 * 1024 * 1024) + (pd_idx as u64 * 2 * 1024 * 1024);
            
            // QEMU エミュレータ環境での安全な描画のため、MMIO領域も一時的に通常メモリとしてマップ
            let flags = NORMAL_MEMORY | HUGE_PAGE;
            *pd.add(pd_idx) = phys_addr | flags;
        }
    }

    Cr3::write(pml4_phys);
    serial_println!("=> Paging: 4GB identity map active. NXE is enabled, page-level execute policy is pending finer-grained mappings.");
}

unsafe fn enable_nx_bit() {
    // EFER (Extended Feature Enable Register) の 11ビット目 (NXE) を立てる
    let mut efer = EFER.read();
    efer |= 1 << 11;
    EFER.write(efer);
}
