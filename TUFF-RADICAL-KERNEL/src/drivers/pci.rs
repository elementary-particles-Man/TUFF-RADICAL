use crate::drivers::io;


#[derive(Clone, Copy, Debug)]
pub struct PciAddress {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
}

#[derive(Clone, Copy, Debug)]
pub enum PciBar {
    Memory32 { base: u64 },
    Memory64 { base: u64 },
    Io { base: u16 },
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
    if index >= 6 {
        return None;
    }

    let offset = 0x10 + index * 4;
    let bar = read_config_u32(addr, offset);
    if bar == 0 || bar == u32::MAX {
        return None;
    }

    if (bar & 0x1) != 0 {
        return Some(PciBar::Io {
            base: (bar & 0xFFFC) as u16,
        });
    }

    let base_low = (bar & 0xFFFF_FFF0) as u64;
    let bar_type = (bar >> 1) & 0x3;
    if bar_type == 0x2 && index < 5 {
        let high = read_config_u32(addr, offset + 4) as u64;
        return Some(PciBar::Memory64 {
            base: (high << 32) | base_low,
        });
    }

    Some(PciBar::Memory32 { base: base_low })
}

#[allow(dead_code)]
pub unsafe fn capability_pointer(addr: PciAddress) -> Option<u8> {
    let status = read_config_u16(addr, 0x06);
    if (status & (1 << 4)) == 0 {
        return None;
    }

    let ptr = read_config_u8(addr, 0x34) & 0xFC;
    if ptr == 0 {
        None
    } else {
        Some(ptr)
    }
}
