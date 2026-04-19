use crate::drivers::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciAddress {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum PciBar {
    Memory32 { base: u64, #[allow(dead_code)] size: u64 },
    Memory64 { base: u64, #[allow(dead_code)] size: u64 },
    Io { base: u16, #[allow(dead_code)] size: u16 },
}

#[allow(dead_code)]
pub struct PciDevice {
    pub address: PciAddress,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    pub prog_if: u8,
}

pub unsafe fn read_config_u32(addr: PciAddress, offset: u8) -> u32 {
    let address = 0x8000_0000
        | ((addr.bus as u32) << 16)
        | ((addr.slot as u32) << 11)
        | ((addr.func as u32) << 8)
        | (offset as u32 & 0xFC);
    io::outl(0xCF8, address);
    io::inl(0xCFC)
}

pub unsafe fn write_config_u32(addr: PciAddress, offset: u8, value: u32) {
    let address = 0x8000_0000
        | ((addr.bus as u32) << 16)
        | ((addr.slot as u32) << 11)
        | ((addr.func as u32) << 8)
        | (offset as u32 & 0xFC);
    io::outl(0xCF8, address);
    io::outl(0xCFC, value);
}

pub unsafe fn read_config_u16(addr: PciAddress, offset: u8) -> u16 {
    let shift = (offset & 0x2) * 8;
    ((read_config_u32(addr, offset) >> shift) & 0xFFFF) as u16
}

#[allow(dead_code)]
pub unsafe fn read_config_u8(addr: PciAddress, offset: u8) -> u8 {
    let shift = (offset & 0x3) * 8;
    ((read_config_u32(addr, offset) >> shift) & 0xFF) as u8
}

pub unsafe fn read_bar(addr: PciAddress, index: u8) -> Option<PciBar> {
    if index >= 6 { return None; }
    let offset = 0x10 + index * 4;
    let original = read_config_u32(addr, offset);
    if original == u32::MAX { return None; }

    // Determine size by writing all 1s
    write_config_u32(addr, offset, 0xFFFF_FFFF);
    let size_mask = read_config_u32(addr, offset);
    write_config_u32(addr, offset, original);

    if (original & 0x1) != 0 {
        let size = (!(size_mask & 0xFFFC)).wrapping_add(1) as u16;
        return Some(PciBar::Io { base: (original & 0xFFFC) as u16, size });
    }

    let base_low = (original & 0xFFFF_FFF0) as u64;
    let size_low = (!(size_mask & 0xFFFF_FFF0)).wrapping_add(1) as u64;
    let bar_type = (original >> 1) & 0x3;

    if bar_type == 0x2 && index < 5 {
        let orig_high = read_config_u32(addr, offset + 4);
        write_config_u32(addr, offset + 4, 0xFFFF_FFFF);
        let size_high = read_config_u32(addr, offset + 4);
        write_config_u32(addr, offset + 4, orig_high);
        
        let size = ((size_high as u64) << 32) | (size_low);
        // This size calculation for 64-bit is a bit naive but works for standard alignments
        return Some(PciBar::Memory64 { 
            base: ((orig_high as u64) << 32) | base_low,
            size
        });
    }

    Some(PciBar::Memory32 { base: base_low, size: size_low })
}

pub unsafe fn probe_device(addr: PciAddress) -> Option<PciDevice> {
    let id_reg = read_config_u32(addr, 0);
    let vendor_id = (id_reg & 0xFFFF) as u16;
    if vendor_id == 0xFFFF { return None; }
    
    let device_id = (id_reg >> 16) as u16;
    let class_reg = read_config_u32(addr, 0x08);
    
    Some(PciDevice {
        address: addr,
        vendor_id,
        device_id,
        class: (class_reg >> 24) as u8,
        subclass: (class_reg >> 16) as u8,
        prog_if: (class_reg >> 8) as u8,
    })
}

#[allow(dead_code)]
pub unsafe fn find_capabilities(addr: PciAddress) -> PciCapIterator {
    let status = read_config_u16(addr, 0x06);
    let ptr = if (status & (1 << 4)) != 0 {
        read_config_u8(addr, 0x34) & 0xFC
    } else {
        0
    };
    PciCapIterator { addr, next_ptr: ptr }
}

pub struct PciCapIterator {
    addr: PciAddress,
    next_ptr: u8,
}

impl Iterator for PciCapIterator {
    type Item = (u8, u8); // (Capability ID, Offset)

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_ptr == 0 { return None; }
        let offset = self.next_ptr;
        let cap_header = unsafe { read_config_u16(self.addr, offset) };
        let cap_id = (cap_header & 0xFF) as u8;
        self.next_ptr = (cap_header >> 8) as u8 & 0xFC;
        Some((cap_id, offset))
    }
}
