use crate::mm::memory;
use crate::arch::x86_64::registers::{Cr3, EFER};
use uefi::table::boot::{MemoryType, PAGE_SIZE};

/// ページテーブルのエントリ属性
const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
#[allow(dead_code)]
const HUGE_PAGE: u64 = 1 << 7; 
const NO_EXECUTE: u64 = 1 << 63;
const CACHE_DISABLE: u64 = 1 << 4;

#[allow(dead_code)]
pub const NORMAL_MEMORY: u64 = PRESENT | WRITABLE | NO_EXECUTE;
pub const CODE_MEMORY: u64 = PRESENT; // NX=0
pub const MMIO_MEMORY: u64 = PRESENT | WRITABLE | NO_EXECUTE | CACHE_DISABLE;

pub unsafe fn init_paging() {
    serial_println!("TUFF-RADICAL-COMMANDER [PAG-01]: Establishing fine-grained page protection...");

    enable_nx_bit();

    let pml4_phys = memory::allocate_page().expect("Failed to allocate PML4");
    let pml4 = pml4_phys as *mut u64;
    for i in 0..512 { *pml4.add(i) = 0; }

    // Phase 1: Identity map first 4GB using 2MB Huge Pages (Baseline)
    // This ensures stack, GDT, IDT, and UEFI data are all accessible.
    let pdpt_phys = memory::allocate_page().expect("Failed to allocate PDPT");
    let pdpt = pdpt_phys as *mut u64;
    for i in 0..512 { *pdpt.add(i) = 0; }
    *pml4.add(0) = pdpt_phys | PRESENT | WRITABLE;

    for pdpt_idx in 0..4 {
        let pd_phys = memory::allocate_page().expect("Failed to allocate PD");
        let pd = pd_phys as *mut u64;
        for i in 0..512 { *pd.add(i) = 0; }
        *pdpt.add(pdpt_idx) = pd_phys | PRESENT | WRITABLE;

        for pd_idx in 0..512 {
            let phys_addr = (pdpt_idx as u64 * 1024 * 1024 * 1024) + (pd_idx as u64 * 2 * 1024 * 1024);
            // Default to RW/NX
            *pd.add(pd_idx) = phys_addr | PRESENT | WRITABLE | HUGE_PAGE | NO_EXECUTE;
        }
    }

    // Phase 2: Refine protections using the Memory Map
    // We override 2MB entries with 4KB page tables for specific regions.
    for desc in memory::boot_memory_map() {
        if desc.ty == MemoryType::LOADER_CODE {
            // Mark kernel code as Executable (NX=0)
            map_range(pml4, desc.phys_start, desc.phys_start, desc.page_count, CODE_MEMORY | WRITABLE);
        } else if desc.ty == MemoryType::MMIO || desc.ty == MemoryType::MMIO_PORT_SPACE {
            map_range(pml4, desc.phys_start, desc.phys_start, desc.page_count, MMIO_MEMORY);
        }
    }

    Cr3::write(pml4_phys);
    serial_println!("=> Paging: Huge-page baseline with 4KB section refinement active.");
}

unsafe fn enable_nx_bit() {
    let mut efer = EFER.read();
    efer |= 1 << 11;
    EFER.write(efer);
}

unsafe fn map_range(pml4: *mut u64, virt: u64, phys: u64, count: u64, flags: u64) {
    for i in 0..count {
        map_page(pml4, virt + (i * PAGE_SIZE as u64), phys + (i * PAGE_SIZE as u64), flags);
    }
}

unsafe fn map_page(pml4: *mut u64, virt: u64, phys: u64, flags: u64) {
    let pml4_idx = (virt >> 39) & 0x1FF;
    let pdpt_idx = (virt >> 30) & 0x1FF;
    let pd_idx = (virt >> 21) & 0x1FF;
    let pt_idx = (virt >> 12) & 0x1FF;

    let pdpt = get_or_create_table(pml4, pml4_idx);
    let pd = get_or_create_table(pdpt, pdpt_idx);
    
    // If PD entry is a HUGE_PAGE, we need to split it or handle it.
    // For simplicity, if it's already mapped as huge, we replace it with a new PT.
    let pd_entry = *pd.add(pd_idx as usize);
    let pt = if (pd_entry & PRESENT) != 0 && (pd_entry & HUGE_PAGE) == 0 {
        (pd_entry & !0xFFF & !0x8000_0000_0000_0000) as *mut u64
    } else {
        let new_table_phys = memory::allocate_page().expect("Failed to allocate PT");
        let new_table = new_table_phys as *mut u64;
        
        if (pd_entry & HUGE_PAGE) != 0 {
            // Split huge page: populate new PT with 512 entries
            let base_phys = pd_entry & !0xFFF & !HUGE_PAGE & !0x8000_0000_0000_0000;
            let old_flags = pd_entry & 0x8000_0000_0000_0FFF & !HUGE_PAGE;
            for i in 0..512 {
                *new_table.add(i) = base_phys + (i as u64 * PAGE_SIZE as u64) | old_flags;
            }
        } else {
            for i in 0..512 { *new_table.add(i) = 0; }
        }
        
        *pd.add(pd_idx as usize) = new_table_phys | PRESENT | WRITABLE;
        new_table
    };

    *pt.add(pt_idx as usize) = (phys & !0xFFF) | flags;
}

unsafe fn get_or_create_table(parent: *mut u64, index: u64) -> *mut u64 {
    let entry = *parent.add(index as usize);
    if (entry & PRESENT) != 0 && (entry & HUGE_PAGE) == 0 {
        (entry & !0xFFF & !0x8000_0000_0000_0000) as *mut u64
    } else {
        let new_table_phys = memory::allocate_page().expect("Failed to allocate table");
        let new_table = new_table_phys as *mut u64;
        for i in 0..512 { *new_table.add(i) = 0; }
        // We don't support splitting huge pages at PML4/PDPT level here as they are unlikely.
        *parent.add(index as usize) = new_table_phys | PRESENT | WRITABLE;
        new_table
    }
}
