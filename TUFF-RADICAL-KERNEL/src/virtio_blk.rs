use core::arch::asm;
use core::ptr::{read_volatile, write_bytes, write_volatile};
use core::sync::atomic::{Ordering, compiler_fence};

use crate::memory;
use crate::pci::{self, PciAddress, PciBar};

const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_BLOCK_DEVICE_ID_TRANSITIONAL: u16 = 0x1001;
const SECTOR_SIZE_BYTES: usize = 512;

const DEVICE_STATUS_ACKNOWLEDGE: u8 = 0x01;
const DEVICE_STATUS_DRIVER: u8 = 0x02;
const DEVICE_STATUS_DRIVER_OK: u8 = 0x04;
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

const QUEUE_SIZE: u16 = 8;
const QUEUE_PAGES: usize = 2;
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
    desc: *mut VirtqDesc,
    avail_flags: *mut u16,
    avail_idx: *mut u16,
    avail_ring: *mut u16,
    used_flags: *mut u16,
    used_idx: *mut u16,
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

impl crate::block::BlockDevice for VirtioBlk {
    fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }

    fn read_sector(&self, sector: u64, buffer: &mut [u8; crate::block::SECTOR_SIZE]) -> Result<(), &'static str> {
        unsafe { self.submit_sector_request(VIRTIO_BLK_T_IN, sector, Some(buffer), None) }
    }

    fn write_sector(&self, sector: u64, buffer: &[u8; crate::block::SECTOR_SIZE]) -> Result<(), &'static str> {
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
            PciBar::Memory32 { .. } | PciBar::Memory64 { .. } => {
                crate::serial_println!(
                    "TUFF-RADICAL-VIRTIO: device {:02x}:{:02x}.{} is block but BAR0 is not legacy I/O.",
                    pci_address.bus,
                    pci_address.slot,
                    pci_address.func
                );
                return None;
            }
        };

        let queue = allocate_queue_layout()?;
        let request = allocate_request_buffers()?;

        let mut device = Self {
            pci_address,
            io_base,
            capacity_sectors: 0,
            queue_size: QUEUE_SIZE,
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

        self.write_status(0);
        self.write_status(DEVICE_STATUS_ACKNOWLEDGE);
        self.write_status(DEVICE_STATUS_ACKNOWLEDGE | DEVICE_STATUS_DRIVER);

        let host_features = self.read_u32(REG_DEVICE_FEATURES);
        self.write_u32(REG_GUEST_FEATURES, 0);

        self.write_u16(REG_QUEUE_SELECT, 0);
        let advertised_queue_size = self.read_u16(REG_QUEUE_SIZE);
        if advertised_queue_size < self.queue_size {
            crate::serial_println!(
                "TUFF-RADICAL-VIRTIO: queue 0 too small on {:02x}:{:02x}.{} (host={}, need={}).",
                self.pci_address.bus,
                self.pci_address.slot,
                self.pci_address.func,
                advertised_queue_size,
                self.queue_size
            );
            self.fail_device();
            return false;
        }

        self.write_u32(REG_QUEUE_ADDRESS, (self.queue.base >> 12) as u32);
        let capacity_low = self.read_u32(REG_DEVICE_CONFIG_START);
        let capacity_high = self.read_u32(REG_DEVICE_CONFIG_START + 4);
        self.capacity_sectors = ((capacity_high as u64) << 32) | capacity_low as u64;

        let _ = self.read_u8(REG_ISR_STATUS);

        self.write_status(
            DEVICE_STATUS_ACKNOWLEDGE | DEVICE_STATUS_DRIVER | DEVICE_STATUS_DRIVER_OK,
        );

        crate::serial_println!(
            "TUFF-RADICAL-VIRTIO: legacy blk {:02x}:{:02x}.{} features={:#010x} queue_size={} capacity={} sectors queue_base=0x{:x}.",
            self.pci_address.bus,
            self.pci_address.slot,
            self.pci_address.func,
            host_features,
            advertised_queue_size,
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
        zero_queue_memory(self.queue);
        zero_request_buffers(self.request);

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
            if spin_count >= 5_000_000 {
                let _ = self.read_u8(REG_ISR_STATUS);
                return Err("virtqueue timeout");
            }
            core::hint::spin_loop();
        }

        compiler_fence(Ordering::SeqCst);
        let used_elem = read_volatile(self.queue.used_ring);
        let status = read_volatile(self.request.status);
        let _ = self.read_u8(REG_ISR_STATUS);
        let _used_flags = read_volatile(self.queue.used_flags);

        if status != 0 {
            return Err("device returned non-zero status");
        }

        if used_elem.id != 0 {
            return Err("used ring returned unexpected descriptor id");
        }

        if used_elem.len < 1 {
            return Err("used ring reported invalid length");
        }

        let data = core::slice::from_raw_parts(self.request.data, SECTOR_SIZE_BYTES);
        if let Some(dst) = read_buffer {
            dst.copy_from_slice(data);
        }

        Ok(())

    }

    unsafe fn fail_device(&self) {
        self.write_status(DEVICE_STATUS_FAILED);
    }

    unsafe fn write_status(&self, value: u8) {
        self.write_u8(REG_DEVICE_STATUS, value);
    }

    unsafe fn read_u8(&self, offset: u16) -> u8 {
        let mut value: u8;
        asm!(
            "in al, dx",
            out("al") value,
            in("dx") self.io_base.wrapping_add(offset),
            options(nostack, nomem)
        );
        value
    }

    unsafe fn read_u16(&self, offset: u16) -> u16 {
        let mut value: u16;
        asm!(
            "in ax, dx",
            out("ax") value,
            in("dx") self.io_base.wrapping_add(offset),
            options(nostack, nomem)
        );
        value
    }

    unsafe fn read_u32(&self, offset: u16) -> u32 {
        let mut value: u32;
        asm!(
            "in eax, dx",
            out("eax") value,
            in("dx") self.io_base.wrapping_add(offset),
            options(nostack, nomem)
        );
        value
    }

    unsafe fn write_u8(&self, offset: u16, value: u8) {
        asm!(
            "out dx, al",
            in("dx") self.io_base.wrapping_add(offset),
            in("al") value,
            options(nostack, nomem)
        );
    }

    unsafe fn write_u16(&self, offset: u16, value: u16) {
        asm!(
            "out dx, ax",
            in("dx") self.io_base.wrapping_add(offset),
            in("ax") value,
            options(nostack, nomem)
        );
    }

    unsafe fn write_u32(&self, offset: u16, value: u32) {
        asm!(
            "out dx, eax",
            in("dx") self.io_base.wrapping_add(offset),
            in("eax") value,
            options(nostack, nomem)
        );
    }
}


unsafe fn allocate_queue_layout() -> Option<QueueLayout> {
    let base = memory::allocate_contiguous_pages(QUEUE_PAGES)?;
    let base_ptr = base as *mut u8;
    let desc = base_ptr as *mut VirtqDesc;
    let avail_offset = core::mem::size_of::<VirtqDesc>() * QUEUE_SIZE as usize;
    let avail_ptr = base_ptr.add(avail_offset) as *mut u16;
    let used_ptr = base_ptr.add(4096) as *mut u16;

    Some(QueueLayout {
        base,
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
    write_bytes(queue.base as *mut u8, 0, QUEUE_PAGES * 4096);
}

unsafe fn zero_request_buffers(request: RequestBuffers) {
    write_bytes(request.base as *mut u8, 0, REQUEST_PAGES * 4096);
}
