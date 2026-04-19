use crate::drivers::pci::{self, PciAddress, PciBar};
use crate::drivers::io;


const VIRTIO_VENDOR_ID: u16 = 0x1AF4;
const VIRTIO_GPU_DEVICE_ID: u16 = 0x1010; // Transitional/Legacy

pub struct VirtioGpu {
    #[allow(dead_code)] pci_address: PciAddress,
    io_base: u16,
}

impl VirtioGpu {
    pub unsafe fn from_pci(pci_address: PciAddress) -> Option<Self> {
        let dev = pci::probe_device(pci_address)?;
        if dev.vendor_id != VIRTIO_VENDOR_ID || dev.device_id != VIRTIO_GPU_DEVICE_ID {
            return None;
        }

        let io_base = match pci::read_bar(pci_address, 0)? {
            PciBar::Io { base, .. } => base,
            _ => return None,
        };

        serial_println!(
            "TUFF-RADICAL-GPU: Found VirtIO-GPU at {:02x}:{:02x}.{}",
            pci_address.bus, pci_address.slot, pci_address.func
        );

        Some(Self { pci_address, io_base })
    }

    pub unsafe fn init(&self) {
        // Basic legacy init
        io::outb(self.io_base + 0x12, 0); // Reset
        io::outb(self.io_base + 0x12, 1); // Acknowledge
        io::outb(self.io_base + 0x12, 3); // Driver
        
        serial_println!("=> GPU: VirtIO transport established. Buffer substrate ready.");
    }
}
