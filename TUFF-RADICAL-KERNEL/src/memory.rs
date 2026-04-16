use uefi::table::boot::{MemoryAttribute, MemoryDescriptor, MemoryMap, MemoryType, PAGE_SIZE};
use crate::serial_println;
use linked_list_allocator::LockedHeap;
use spin::Mutex;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const HEAP_SIZE_BYTES: u64 = 64 * 1024 * 1024; // Increased to 64MB for stability
const MAX_MEMORY_DESCRIPTORS: usize = 512;
const EMPTY_DESCRIPTOR: MemoryDescriptor = MemoryDescriptor {
    ty: MemoryType::RESERVED,
    phys_start: 0,
    virt_start: 0,
    page_count: 0,
    att: MemoryAttribute::empty(),
};

pub struct PhysicalMemoryManager {
    current_descriptor_index: usize,
    next_free_phys_addr: u64,
    current_region_end: u64,
}

static PMM: Mutex<Option<PhysicalMemoryManager>> = Mutex::new(None);
static mut BOOT_MEMORY_MAP: [MemoryDescriptor; MAX_MEMORY_DESCRIPTORS] =
    [EMPTY_DESCRIPTOR; MAX_MEMORY_DESCRIPTORS];
static mut BOOT_MEMORY_MAP_LEN: usize = 0;

impl PhysicalMemoryManager {
    fn allocate_page(&mut self) -> Option<u64> {
        loop {
            if self.next_free_phys_addr + PAGE_SIZE as u64 <= self.current_region_end {
                let addr = self.next_free_phys_addr;
                self.next_free_phys_addr += PAGE_SIZE as u64;
                return Some(addr);
            }

            if !self.advance_to_next_region() {
                return None;
            }
        }
    }

    fn advance_to_next_region(&mut self) -> bool {
        for (index, desc) in boot_memory_map()
            .iter()
            .enumerate()
            .skip(self.current_descriptor_index + 1)
        {
            if let Some((start, end)) = conventional_region_bounds(desc) {
                self.current_descriptor_index = index;
                self.next_free_phys_addr = start;
                self.current_region_end = end;
                return true;
            }
        }

        false
    }
}

pub fn init_memory(memory_map: &MemoryMap<'static>) {
    serial_println!("TUFF-RADICAL-COMMANDER [MEM-02]: Asserting T-RADty over Memory...");

    let stored_descriptors = snapshot_memory_map(memory_map);
    if stored_descriptors == 0 {
        serial_println!("=> PMM: FATAL: ExitBootServices memory map snapshot is empty.");
        return;
    }

    let mut largest_region_index = None;
    let mut largest_region_start = 0;
    let mut largest_region_end = 0;
    let mut largest_region_pages = 0;
    let mut conventional_pages = 0;

    for (index, desc) in boot_memory_map().iter().enumerate() {
        if let Some((start, end)) = conventional_region_bounds(desc) {
            conventional_pages += desc.page_count;
            if desc.page_count > largest_region_pages {
                largest_region_index = Some(index);
                largest_region_start = start;
                largest_region_end = end;
                largest_region_pages = desc.page_count;
            }
        }
    }

    serial_println!(
        "=> PMM: Stored {} UEFI descriptors ({} conventional pages, ~{} MB).",
        stored_descriptors,
        conventional_pages,
        (conventional_pages * PAGE_SIZE as u64) / (1024 * 1024)
    );

    let largest_region_index = match largest_region_index {
        Some(index) => index,
        None => {
            serial_println!("=> PMM: FATAL: No conventional memory found after ExitBootServices.");
            return;
        }
    };

    if largest_region_end.saturating_sub(largest_region_start) < HEAP_SIZE_BYTES {
        serial_println!(
            "=> PMM: FATAL: Largest conventional region too small for heap (region={} bytes, heap={} bytes).",
            largest_region_end.saturating_sub(largest_region_start),
            HEAP_SIZE_BYTES
        );
        return;
    }

    serial_println!(
        "=> PMM: Largest region at 0x{:x}-0x{:x} ({} pages)",
        largest_region_start,
        largest_region_end,
        largest_region_pages
    );

    unsafe {
        let heap_start = largest_region_start;
        let heap_end = heap_start + HEAP_SIZE_BYTES;
        ALLOCATOR.lock().init(heap_start as *mut u8, HEAP_SIZE_BYTES as usize);

        *PMM.lock() = Some(PhysicalMemoryManager {
            current_descriptor_index: largest_region_index,
            next_free_phys_addr: heap_end,
            current_region_end: largest_region_end,
        });
    }

    serial_println!(
        "=> Heap: {} MiB T-RAD Heap established from ExitBootServices map.",
        HEAP_SIZE_BYTES / (1024 * 1024)
    );
}

pub fn allocate_page() -> Option<u64> {
    let mut pmm = PMM.lock();
    match pmm.as_mut() {
        Some(pmm) => pmm.allocate_page(),
        None => None,
    }
}

pub fn boot_memory_map() -> &'static [MemoryDescriptor] {
    unsafe { &BOOT_MEMORY_MAP[..BOOT_MEMORY_MAP_LEN] }
}

pub fn inspect_memory_map() {
    let mut usable_pages = 0;

    for desc in boot_memory_map() {
        if desc.ty == MemoryType::CONVENTIONAL {
            usable_pages += desc.page_count;
        }
    }

    serial_println!(
        "TUFF-RADICAL-COMMANDER [MEM-01]: Physical Free Pool: {} pages (~{} MB)",
        usable_pages,
        (usable_pages * PAGE_SIZE as u64) / (1024 * 1024)
    );
}

fn snapshot_memory_map(memory_map: &MemoryMap<'static>) -> usize {
    let mut count = 0;

    for desc in memory_map.entries() {
        if count == MAX_MEMORY_DESCRIPTORS {
            serial_println!(
                "=> PMM: WARNING: Memory descriptor snapshot truncated at {} entries.",
                MAX_MEMORY_DESCRIPTORS
            );
            break;
        }

        unsafe {
            BOOT_MEMORY_MAP[count] = *desc;
        }
        count += 1;
    }

    unsafe {
        BOOT_MEMORY_MAP_LEN = count;
    }

    count
}

fn conventional_region_bounds(desc: &MemoryDescriptor) -> Option<(u64, u64)> {
    if desc.ty != MemoryType::CONVENTIONAL || desc.page_count == 0 {
        return None;
    }

    let byte_len = desc.page_count.checked_mul(PAGE_SIZE as u64)?;
    let region_end = desc.phys_start.checked_add(byte_len)?;
    Some((desc.phys_start, region_end))
}
