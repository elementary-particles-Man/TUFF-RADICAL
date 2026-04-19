use linked_list_allocator::LockedHeap;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_SIZE_BYTES: u64 = 64 * 1024 * 1024; // 64MB
