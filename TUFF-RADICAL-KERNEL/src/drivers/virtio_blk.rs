use core::arch::asm;
use core::ptr::{read_volatile, write_bytes, write_volatile};
use core::sync::atomic::{Ordering, compiler_fence};

use crate::mm::memory;
use crate::drivers::pci::{self, PciAddress, PciBar};

const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_BLOCK_DEVICE_ID_TRANSITIONAL: u16 = 0x1001;
const SECTOR_SIZE_BYTES: usize = 512;

const DEVICE_STATUS_ACKNOWLEDGE: u8 = 0x01;
const DEVICE_STATUS_DRIVER: u8 = 0x02;
const DEVICE_STATUS_DRIVER_OK: u8 = 0x04;
#[allow(dead_code)]
const DEVICE_STATUS_FAILED: u8 = 0x80;

const VRING_DESC_F_NEXT: u16 = 1;
const VRING_DESC_F_WRITE: u16 = 2;

const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;

const REG_DEVICE_FEATURES: u16 = 0x00;
const REG_GUEST_FEATURES: u16 = 0x04;
const REG_QUEUE_ADDRESS: u16 = 0x08;
const REG_QUEUE_SIZE: u16 = 0x0C;
const REG_QUEUE_SELECT: u16 = 0x0E;
const REG_QUEUE_NOTIFY: u16 = 0x10;
const REG_DEVICE_STATUS: u16 = 0x12;
const REG_ISR_STATUS: u16 = 0x13;
const REG_DEVICE_CONFIG_START: u16 = 0x14;

const REQUEST_PAGES: usize = 1;

#[repr(C)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct VirtioBlkReqHeader {
    req_type: u32,
    reserved: u32,
    sector: u64,
}

#[repr(C)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[derive(Clone, Copy)]
struct QueueLayout {
    base: u64,
    pages: usize,
    desc: *mut VirtqDesc,
    avail_flags: *mut u16,
    avail_idx: *mut u16,
    avail_ring: *mut u16,
    #[allow(dead_code)]
    used_flags: *mut u16,
    used_idx: *mut u16,
    #[allow(dead_code)]
    used_ring: *mut VirtqUsedElem,
}

#[derive(Clone, Copy)]
struct RequestBuffers {
    base: u64,
    header: *mut VirtioBlkReqHeader,
    data: *mut u8,
    status: *mut u8,
}

#[derive(Clone, Copy)]
pub struct VirtioBlk {
    pci_address: PciAddress,
    io_base: u16,
    capacity_sectors: u64,
    queue_size: u16,
    queue: QueueLayout,
    request: RequestBuffers,
}

impl crate::drivers::block::BlockDevice for VirtioBlk {
    fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }

    fn read_sector(&self, sector: u64, buffer: &mut [u8; crate::drivers::block::SECTOR_SIZE]) -> Result<(), &'static str> {
        unsafe { self.submit_sector_request(VIRTIO_BLK_T_IN, sector, Some(buffer), None) }
    }

    fn write_sector(&self, sector: u64, buffer: &[u8; crate::drivers::block::SECTOR_SIZE]) -> Result<(), &'static str> {
        unsafe { self.submit_sector_request(VIRTIO_BLK_T_OUT, sector, None, Some(buffer)) }
    }
}

unsafe impl Send for VirtioBlk {}
unsafe impl Sync for VirtioBlk {}

impl VirtioBlk {

    pub unsafe fn from_pci(pci_address: PciAddress) -> Option<Self> {
        let vendor_id = pci::read_config_u16(pci_address, 0x00);
        let device_id = pci::read_config_u16(pci_address, 0x02);

        if vendor_id != VIRTIO_VENDOR_ID || device_id != VIRTIO_BLOCK_DEVICE_ID_TRANSITIONAL {
            return None;
        }

        let io_base = match pci::read_bar(pci_address, 0)? {
            PciBar::Io { base } => base,
            _ => return None,
        };

        // 1. Reset device
        write_io_u8(io_base, REG_DEVICE_STATUS, 0);
        write_io_u8(io_base, REG_DEVICE_STATUS, DEVICE_STATUS_ACKNOWLEDGE | DEVICE_STATUS_DRIVER);

        // 2. Negotiate queue size
        write_io_u16(io_base, REG_QUEUE_SELECT, 0);
        let queue_size = read_io_u16(io_base, REG_QUEUE_SIZE);
        
        let queue = allocate_queue_layout(queue_size)?;
        let request = allocate_request_buffers()?;

        let mut device = Self {
            pci_address,
            io_base,
            capacity_sectors: 0,
            queue_size,
            queue,
            request,
        };

        if !device.initialize_legacy_transport() {
            return None;
        }

        Some(device)
    }

    unsafe fn initialize_legacy_transport(&mut self) -> bool {
        zero_queue_memory(self.queue);
        zero_request_buffers(self.request);

        let host_features = self.read_u32(REG_DEVICE_FEATURES);
        self.write_u32(REG_GUEST_FEATURES, 0);

        self.write_u16(REG_QUEUE_SELECT, 0);
        self.write_u32(REG_QUEUE_ADDRESS, (self.queue.base >> 12) as u32);
        
        let capacity_low = self.read_u32(REG_DEVICE_CONFIG_START);
        let capacity_high = self.read_u32(REG_DEVICE_CONFIG_START + 4);
        self.capacity_sectors = ((capacity_high as u64) << 32) | capacity_low as u64;

        self.write_status(
            DEVICE_STATUS_ACKNOWLEDGE | DEVICE_STATUS_DRIVER | DEVICE_STATUS_DRIVER_OK,
        );

        crate::serial_println!(
            "TUFF-RADICAL-VIRTIO: legacy blk {:02x}:{:02x}.{} features={:#010x} queue_size={} capacity={} sectors queue_base=0x{:x}.",
            self.pci_address.bus,
            self.pci_address.slot,
            self.pci_address.func,
            host_features,
            self.queue_size,
            self.capacity_sectors,
            self.queue.base
        );
        true
    }

    unsafe fn submit_sector_request(
        &self,
        req_type: u32,
        sector: u64,
        read_buffer: Option<&mut [u8; SECTOR_SIZE_BYTES]>,
        write_buffer: Option<&[u8; SECTOR_SIZE_BYTES]>,
    ) -> Result<(), &'static str> {
        // TUFF-RADICAL: Surgical sync-reset to ensure predictable indices without complex state tracking
        zero_queue_memory(self.queue);
        zero_request_buffers(self.request);
        self.write_u16(REG_QUEUE_SELECT, 0);
        self.write_u32(REG_QUEUE_ADDRESS, (self.queue.base >> 12) as u32);

        if read_buffer.is_some() == write_buffer.is_some() {
            return Err("request buffer direction is invalid");
        }

        write_volatile(
            self.request.header,
            VirtioBlkReqHeader {
                req_type,
                reserved: 0,
                sector,
            },
        );
        write_volatile(self.request.status, 0xFF);

        if let Some(src) = write_buffer {
            core::ptr::copy_nonoverlapping(src.as_ptr(), self.request.data, SECTOR_SIZE_BYTES);
        }

        let desc = self.queue.desc;
        write_volatile(
            desc.add(0),
            VirtqDesc {
                addr: self.request.header as u64,
                len: core::mem::size_of::<VirtioBlkReqHeader>() as u32,
                flags: VRING_DESC_F_NEXT,
                next: 1,
            },
        );
        write_volatile(
            desc.add(1),
            VirtqDesc {
                addr: self.request.data as u64,
                len: SECTOR_SIZE_BYTES as u32,
                flags: if read_buffer.is_some() {
                    VRING_DESC_F_WRITE | VRING_DESC_F_NEXT
                } else {
                    VRING_DESC_F_NEXT
                },
                next: 2,
            },
        );
        write_volatile(
            desc.add(2),
            VirtqDesc {
                addr: self.request.status as u64,
                len: 1,
                flags: VRING_DESC_F_WRITE,
                next: 0,
            },
        );

        write_volatile(self.queue.avail_flags, 0);
        write_volatile(self.queue.avail_ring, 0);
        write_volatile(self.queue.avail_idx, 1);

        compiler_fence(Ordering::SeqCst);
        self.write_u16(REG_QUEUE_NOTIFY, 0);

        let mut spin_count = 0usize;
        while read_volatile(self.queue.used_idx) == 0 {
            spin_count += 1;
            if spin_count >= 10_000_000 {
                let _ = self.read_u8(REG_ISR_STATUS);
                return Err("virtqueue timeout");
            }
            core::hint::spin_loop();
        }

        compiler_fence(Ordering::SeqCst);
        let status = read_volatile(self.request.status);
        let _ = self.read_u8(REG_ISR_STATUS);

        if status != 0 {
            return Err("device returned non-zero status");
        }

        let data = core::slice::from_raw_parts(self.request.data, SECTOR_SIZE_BYTES);
        if let Some(dst) = read_buffer {
            dst.copy_from_slice(data);
        }

        Ok(())
    }

    unsafe fn write_status(&self, value: u8) {
        self.write_u8(REG_DEVICE_STATUS, value);
    }

    unsafe fn read_u8(&self, offset: u16) -> u8 { read_io_u8(self.io_base, offset) }
    #[allow(dead_code)]
    unsafe fn read_u16(&self, offset: u16) -> u16 { read_io_u16(self.io_base, offset) }
    unsafe fn read_u32(&self, offset: u16) -> u32 { read_io_u32(self.io_base, offset) }
    unsafe fn write_u16(&self, offset: u16, value: u16) { write_io_u16(self.io_base, offset, value) }
    unsafe fn write_u32(&self, offset: u16, value: u32) { write_io_u32(self.io_base, offset, value) }
    unsafe fn write_u8(&self, offset: u16, value: u8) { write_io_u8(self.io_base, offset, value) }
}

unsafe fn read_io_u8(base: u16, offset: u16) -> u8 {
    let value: u8;
    asm!("in al, dx", out("al") value, in("dx") base + offset, options(nostack, nomem));
    value
}

unsafe fn read_io_u16(base: u16, offset: u16) -> u16 {
    let value: u16;
    asm!("in ax, dx", out("ax") value, in("dx") base + offset, options(nostack, nomem));
    value
}

unsafe fn read_io_u32(base: u16, offset: u16) -> u32 {
    let value: u32;
    asm!("in eax, dx", out("eax") value, in("dx") base + offset, options(nostack, nomem));
    value
}

unsafe fn write_io_u8(base: u16, offset: u16, value: u8) {
    asm!("out dx, al", in("dx") base + offset, in("al") value, options(nostack, nomem));
}

unsafe fn write_io_u16(base: u16, offset: u16, value: u16) {
    asm!("out dx, ax", in("dx") base + offset, in("ax") value, options(nostack, nomem));
}

unsafe fn write_io_u32(base: u16, offset: u16, value: u32) {
    asm!("out dx, eax", in("dx") base + offset, in("eax") value, options(nostack, nomem));
}

unsafe fn allocate_queue_layout(size: u16) -> Option<QueueLayout> {
    let size_val = size as usize;
    let desc_size = 16 * size_val;
    let avail_size = 2 + 2 + 2 * size_val + 2;
    let used_offset = (desc_size + avail_size + 4095) & !4095;
    let used_size = 2 + 2 + 8 * size_val + 2;
    let total_size = used_offset + ((used_size + 4095) & !4095);
    let pages = total_size / 4096;

    let base = memory::allocate_contiguous_pages(pages)?;
    let base_ptr = base as *mut u8;
    
    let desc = base_ptr as *mut VirtqDesc;
    let avail_ptr = base_ptr.add(desc_size) as *mut u16;
    let used_ptr = base_ptr.add(used_offset) as *mut u16;

    Some(QueueLayout {
        base,
        pages,
        desc,
        avail_flags: avail_ptr,
        avail_idx: avail_ptr.add(1),
        avail_ring: avail_ptr.add(2),
        used_flags: used_ptr,
        used_idx: used_ptr.add(1),
        used_ring: used_ptr.add(2) as *mut VirtqUsedElem,
    })
}

unsafe fn allocate_request_buffers() -> Option<RequestBuffers> {
    let base = memory::allocate_contiguous_pages(REQUEST_PAGES)?;
    let base_ptr = base as *mut u8;
    let header = base_ptr as *mut VirtioBlkReqHeader;
    let data = base_ptr.add(64);
    let status = base_ptr.add(64 + SECTOR_SIZE_BYTES);

    Some(RequestBuffers {
        base,
        header,
        data,
        status,
    })
}

unsafe fn zero_queue_memory(queue: QueueLayout) {
    write_bytes(queue.base as *mut u8, 0, queue.pages * 4096);
}

unsafe fn zero_request_buffers(request: RequestBuffers) {
    write_bytes(request.base as *mut u8, 0, REQUEST_PAGES * 4096);
}
