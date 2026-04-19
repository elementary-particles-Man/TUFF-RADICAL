use crate::drivers::io;
use core::{
    arch::{asm, x86_64::__cpuid},
    ffi::c_void,
    mem::size_of,
    ptr,
};
use alloc::vec::Vec;
use uefi::table::{cfg, Runtime, SystemTable};


const RSDP_SIGNATURE: [u8; 8] = *b"RSD PTR ";
const MADT_SIGNATURE: [u8; 4] = *b"APIC";
const IA32_APIC_BASE_MSR: u32 = 0x1B;
const IA32_X2APIC_EOI_MSR: u32 = 0x80B;
const LOCAL_APIC_EOI_OFFSET: usize = 0xB0;
const APIC_BASE_ADDRESS_MASK: u64 = 0xFFFF_F000;

// APIC Timer Registers
const LVT_TIMER_OFFSET: usize = 0x320;
const TIMER_INITIAL_COUNT_OFFSET: usize = 0x380;
const TIMER_DIVIDE_CONFIG_OFFSET: usize = 0x3E0;

// I/O APIC Registers
#[allow(dead_code)]
const IO_APIC_REGSEL: usize = 0x00;
#[allow(dead_code)]
const IO_APIC_IOWIN: usize = 0x10;

#[allow(dead_code)]
const IO_APIC_ID: u32 = 0x00;
const IO_APIC_VER: u32 = 0x01;
#[allow(dead_code)]
const IO_APIC_ARB: u32 = 0x02;

static mut DISCOVERED_TOPOLOGY: Option<ApicTopology> = None;
static mut TIMER_READY: bool = false;

#[derive(Debug, Clone, Copy)]
pub enum ApicMode {
    XApic,
    X2Apic,
}

#[derive(Debug, Clone)]
pub struct IoApic { 
    #[allow(dead_code)]
    pub id: u8,
    pub address: u64, 
    pub gsi_base: u32, 
    pub max_redirection_entry: u8 
}

#[derive(Clone)]
pub struct ApicTopology {
    pub local_apic_address: u64,
    pub local_apic_count: usize,
    #[allow(dead_code)]
    pub io_apic_count: usize,
    pub x2apic_count: usize,
    pub interrupt_override_count: usize,
    pub io_apics: Vec<IoApic>,
    pub legacy_pic_present: bool,
    pub x2apic_capable: bool,
    pub controller_enabled: bool,
    pub mode: ApicMode,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct RsdpV1 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct RsdpV2 {
    v1: RsdpV1,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct SdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct MadtHeader {
    header: SdtHeader,
    local_apic_address: u32,
    flags: u32,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct MadtEntryHeader {
    entry_type: u8,
    length: u8,
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct MadtIoApicEntry {
    header: MadtEntryHeader,
    id: u8,
    reserved: u8,
    address: u32,
    gsi_base: u32,
}

pub fn init(system_table: &SystemTable<Runtime>) -> Option<ApicTopology> {
    match discover_topology(system_table) {
        Ok(mut topology) => {
            unsafe {
                // Initialize I/O APICs (detect max entries)
                for ioapic in topology.io_apics.iter_mut() {
                    let version = io_apic_read(ioapic.address, IO_APIC_VER);
                    ioapic.max_redirection_entry = ((version >> 16) & 0xFF) as u8;
                }

                DISCOVERED_TOPOLOGY = Some(topology.clone());
                init_timer(&topology);
            }

            serial_println!(
                "TUFF-RADICAL-APIC [DISCOVERY]: mode={:?} enabled={} x2apic_capable={}",
                topology.mode,
                topology.controller_enabled,
                topology.x2apic_capable
            );
            serial_println!(
                "TUFF-RADICAL-APIC [TOPOLOGY]: lapic=0x{:x} ioapics={} overrides={} legacy_pic={}",
                topology.local_apic_address,
                topology.io_apics.len(),
                topology.interrupt_override_count,
                topology.legacy_pic_present
            );
            
            for (i, ioapic) in topology.io_apics.iter().enumerate() {
                serial_println!(
                    "  IOAPIC[{}]: addr=0x{:x} gsi_base={} max_entries={}",
                    i, ioapic.address, ioapic.gsi_base, ioapic.max_redirection_entry
                );
            }
            
            unsafe {
                if TIMER_READY {
                    serial_println!("TUFF-RADICAL-APIC [STATUS]: Timer initialized. Preemptive multitasking ENABLED.");
                } else {
                    serial_println!("TUFF-RADICAL-APIC [STATUS]: Timer init failed. Cooperative mode remains active.");
                }
            }

            Some(topology)
        }
        Err(err) => {
            serial_println!(
                "TUFF-RADICAL-APIC [WARN]: ACPI/APIC discovery failed: {}. Staying in cooperative mode.",
                err
            );
            None
        }
    }
}

unsafe fn init_timer(topology: &ApicTopology) {
    if !topology.controller_enabled { return; }

    match topology.mode {
        ApicMode::XApic => {
            let base = topology.local_apic_address as usize;
            ptr::write_volatile((base + TIMER_DIVIDE_CONFIG_OFFSET) as *mut u32, 0x03);
            ptr::write_volatile((base + LVT_TIMER_OFFSET) as *mut u32, 32 | (1 << 17));
            ptr::write_volatile((base + TIMER_INITIAL_COUNT_OFFSET) as *mut u32, 0x1000000);
            TIMER_READY = true;
        }
        ApicMode::X2Apic => {
            write_msr(0x83E, 0x03); 
            write_msr(0x832, 32 | (1 << 17)); 
            write_msr(0x838, 0x1000000); 
            TIMER_READY = true;
        }
    }
}

pub fn timer_routing_ready() -> bool {
    unsafe { TIMER_READY }
}

pub fn end_of_interrupt() {
    let topology = unsafe { DISCOVERED_TOPOLOGY.as_mut() };
    let Some(topology) = topology else { return; };
    if !topology.controller_enabled { return; }

    unsafe {
        match topology.mode {
            ApicMode::XApic => {
                let eoi = (topology.local_apic_address as usize + LOCAL_APIC_EOI_OFFSET) as *mut u32;
                ptr::write_volatile(eoi, 0);
            }
            ApicMode::X2Apic => write_msr(IA32_X2APIC_EOI_MSR, 0),
        }
    }
}

fn discover_topology(system_table: &SystemTable<Runtime>) -> Result<ApicTopology, &'static str> {
    let cpuid = __cpuid(1);
    let has_apic = (cpuid.edx & (1 << 9)) != 0;
    let has_x2apic = (cpuid.ecx & (1 << 21)) != 0;
    if !has_apic { return Err("CPUID reports no local APIC support"); }

    let apic_base = unsafe { read_msr(IA32_APIC_BASE_MSR) };
    let controller_enabled = (apic_base & (1 << 11)) != 0;
    let x2apic_enabled = (apic_base & (1 << 10)) != 0;
    
    let rsdp = find_rsdp(system_table).ok_or("RSDP not found in UEFI config table")?;
    let madt = find_madt(rsdp)?;
    let madt_header = read_unaligned::<MadtHeader>(madt.cast());

    let mut topology = ApicTopology {
        local_apic_address: if madt_header.local_apic_address != 0 {
            madt_header.local_apic_address as u64
        } else {
            apic_base & APIC_BASE_ADDRESS_MASK
        },
        local_apic_count: 0,
        io_apic_count: 0,
        x2apic_count: 0,
        interrupt_override_count: 0,
        io_apics: Vec::new(),
        legacy_pic_present: (madt_header.flags & 1) != 0,
        x2apic_capable: has_x2apic,
        controller_enabled,
        mode: if x2apic_enabled { ApicMode::X2Apic } else { ApicMode::XApic },
    };

    let madt_len = madt_header.header.length as usize;
    let mut cursor = unsafe { (madt as *const u8).add(size_of::<MadtHeader>()) };
    let end = unsafe { (madt as *const u8).add(madt_len) };

    while cursor < end {
        let entry = read_unaligned::<MadtEntryHeader>(cursor.cast());
        match entry.entry_type {
            0 => topology.local_apic_count += 1,
            1 => {
                let io_apic_entry = read_unaligned::<MadtIoApicEntry>(cursor.cast());
                topology.io_apics.push(IoApic {
                    id: io_apic_entry.id,
                    address: io_apic_entry.address as u64,
                    gsi_base: io_apic_entry.gsi_base,
                    max_redirection_entry: 0,
                });
            },
            2 => topology.interrupt_override_count += 1,
            9 => topology.x2apic_count += 1,
            _ => {}
        }
        cursor = unsafe { cursor.add(entry.length as usize) };
    }

    Ok(topology)
}

fn find_rsdp(system_table: &SystemTable<Runtime>) -> Option<*const c_void> {
    for entry in system_table.config_table() {
        if entry.guid == cfg::ACPI2_GUID { return Some(entry.address); }
    }
    for entry in system_table.config_table() {
        if entry.guid == cfg::ACPI_GUID { return Some(entry.address); }
    }
    None
}

fn find_madt(rsdp_ptr: *const c_void) -> Result<*const c_void, &'static str> {
    let rsdp_v1 = read_unaligned::<RsdpV1>(rsdp_ptr.cast());
    if rsdp_v1.signature != RSDP_SIGNATURE { return Err("RSDP signature mismatch"); }
    
    let (root_sdt, entry_size) = if rsdp_v1.revision >= 2 {
        let rsdp_v2 = read_unaligned::<RsdpV2>(rsdp_ptr.cast());
        (rsdp_v2.xsdt_address as usize, size_of::<u64>())
    } else {
        (rsdp_v1.rsdt_address as usize, size_of::<u32>())
    };

    let root_header = read_unaligned::<SdtHeader>(root_sdt as *const SdtHeader);
    let root_len = root_header.length as usize;
    let entry_count = (root_len - size_of::<SdtHeader>()) / entry_size;
    let entry_base = unsafe { (root_sdt as *const u8).add(size_of::<SdtHeader>()) };

    for index in 0..entry_count {
        let table_address = if entry_size == size_of::<u64>() {
            read_unaligned::<u64>(unsafe { entry_base.add(index * entry_size) }.cast()) as usize
        } else {
            read_unaligned::<u32>(unsafe { entry_base.add(index * entry_size) }.cast()) as usize
        };

        if table_address == 0 { continue; }
        let header = read_unaligned::<SdtHeader>(table_address as *const SdtHeader);
        if header.signature == MADT_SIGNATURE { return Ok(table_address as *const c_void); }
    }

    Err("MADT not found")
}

fn read_unaligned<T: Copy>(ptr: *const T) -> T { unsafe { ptr::read_unaligned(ptr) } }

unsafe fn read_msr(msr: u32) -> u64 {
    let low: u32; let high: u32;
    asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nomem, nostack));
    ((high as u64) << 32) | (low as u64)
}

unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32; let high = (value >> 32) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem, nostack));
}

pub unsafe fn io_apic_read(base: u64, reg: u32) -> u32 {
    let sel = base as *mut u32;
    let win = (base + 0x10) as *mut u32;
    ptr::write_volatile(sel, reg);
    ptr::read_volatile(win)
}

#[allow(dead_code)]
pub unsafe fn io_apic_write(base: u64, reg: u32, data: u32) {
    let sel = base as *mut u32;
    let win = (base + 0x10) as *mut u32;
    ptr::write_volatile(sel, reg);
    ptr::write_volatile(win, data);
}

#[allow(dead_code)]
pub unsafe fn io_apic_set_redirection(base: u64, index: u8, vector: u8, dest_apic_id: u8) {
    let low_reg = 0x10 + (index as u32 * 2);
    let high_reg = low_reg + 1;
    io_apic_write(base, low_reg, vector as u32);
    io_apic_write(base, high_reg, (dest_apic_id as u32) << 24);
}

pub unsafe fn disable_8259_pic() {
    // Mask all interrupts on the legacy PIC
    io::outb(0x21, 0xFF);
    io::outb(0xA1, 0xFF);
}
