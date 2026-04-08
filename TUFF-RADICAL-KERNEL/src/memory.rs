use uefi::prelude::*;
use uefi::table::boot::MemoryType;
use crate::serial_println;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub struct PhysicalMemoryManager {
    next_free_phys_addr: u64,
    end_phys_addr: u64,
}

static mut PMM: Option<PhysicalMemoryManager> = None;

pub fn init_memory(system_table: &SystemTable<Boot>) {
    serial_println!("TUFF-RADICAL-COMMANDER [MEM-02]: Asserting Sovereignty over Memory...");

    let bt = system_table.boot_services();
    let mut map_buffer = [0u8; 16384];

    match bt.memory_map(&mut map_buffer) {
        Ok(descriptors) => {
            let mut largest_region_start = 0;
            let mut largest_region_pages = 0;
            
            for desc in descriptors.entries() {
                if desc.ty == MemoryType::CONVENTIONAL && desc.page_count > largest_region_pages {
                    largest_region_start = desc.phys_start;
                    largest_region_pages = desc.page_count;
                }
            }

            if largest_region_pages > 0 {
                serial_println!("=> PMM: Largest region at 0x{:x} ({} pages)", 
                      largest_region_start, largest_region_pages);
                
                unsafe {
                    PMM = Some(PhysicalMemoryManager {
                        next_free_phys_addr: largest_region_start,
                        end_phys_addr: largest_region_start + (largest_region_pages * 4096),
                    });

                    let heap_start = largest_region_start;
                    let heap_size = 32 * 1024 * 1024; // 32 MB
                    ALLOCATOR.lock().init(heap_start as *mut u8, heap_size as usize);
                    
                    if let Some(ref mut pmm) = PMM {
                        pmm.next_free_phys_addr += heap_size;
                    }
                }
                serial_println!("=> Heap: 32MB Sovereign Heap established.");
            } else {
                serial_println!("=> PMM: FATAL: No conventional memory found!");
            }
        },
        Err(e) => {
            serial_println!("=> PMM: FATAL: Failed to retrieve memory map: {:?}", e);
        }
    }
}

pub fn allocate_page() -> Option<u64> {
    unsafe {
        if let Some(ref mut pmm) = PMM {
            if pmm.next_free_phys_addr + 4096 <= pmm.end_phys_addr {
                let addr = pmm.next_free_phys_addr;
                pmm.next_free_phys_addr += 4096;
                return Some(addr);
            }
        }
    }
    None
}

pub fn inspect_memory_map(system_table: &SystemTable<Boot>) {
    let bt = system_table.boot_services();
    let mut buffer = [0u8; 16384];

    if let Ok(descriptors) = bt.memory_map(&mut buffer) {
        let mut usable_pages = 0;
        
        for desc in descriptors.entries() {
            if desc.ty == MemoryType::CONVENTIONAL {
                usable_pages += desc.page_count;
            }
        }
        serial_println!("TUFF-RADICAL-COMMANDER [MEM-01]: Physical Free Pool: {} pages (~{} MB)", 
                        usable_pages, (usable_pages * 4096) / (1024 * 1024));
    }
}
