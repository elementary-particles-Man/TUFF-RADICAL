#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::arch::asm;
use uefi::prelude::*;
use uefi::allocator::exit_boot_services;

#[macro_use]
mod drivers;
mod arch;
mod mm;
mod compression;
mod task;
mod installer;

use task::{Task, executor::Executor};
use crate::drivers::gpu::GpuCommandRing;
use crate::drivers::virtio_blk::VirtioBlk;
use core::{future::Future, pin::Pin, task::{Context, Poll}};
use core::sync::atomic::Ordering;
use crate::drivers::pci::{PciAddress, PciBar};
use crate::arch::x86_64::{interrupts, cpu, gdt, apic, paging};
use crate::mm::memory;
use crate::compression::zram;

// --- 非同期スリープの実装 ---
struct SleepFuture {
    target_tick: u64,
}

impl SleepFuture {
    fn new(ticks: u64) -> Self {
        let current = interrupts::current_tick();
        SleepFuture { target_tick: current + ticks }
    }
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let current = interrupts::current_tick();
        if current >= self.target_tick {
            Poll::Ready(())
        } else {
            interrupts::register_timer_waker(self.target_tick, cx.waker());
            Poll::Pending
        }
    }
}

#[entry]
fn main(_image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    unsafe { drivers::serial::COM1.lock().init(); }
    serial_println!("--- TUFF-RADICAL-KERNEL T-RAD REBIRTH (FINAL TUNE) ---");

    serial_println!("TUFF-RADICAL-KERNEL: Requesting UEFI ExitBootServices handoff...");
    let (runtime_table, mut memory_map) = system_table.exit_boot_services();
    exit_boot_services();
    x86_64::instructions::interrupts::disable();
    serial_println!("TUFF-RADICAL-KERNEL: ExitBootServices complete. Firmware boot services are offline.");

    memory_map.sort();
    memory::init_memory(&memory_map);
    memory::inspect_memory_map();
    unsafe { paging::init_paging(); }
    
    // CPU feature detection also wires the runtime SIMD state.
    let features = unsafe { cpu::init_simd() };
    cpu::log_features(&features);
    
    if !features.avx_enabled {
        serial_println!("TUFF-RADICAL-KERNEL: [WARNING] AVX runtime unavailable. SIMD optimization degraded.");
    }

    serial_println!("TUFF-RADICAL-KERNEL: Asserting absolute control over CPU (GDT/IDT)...");
    gdt::init();
    interrupts::init_idt();
    let _apic_topology = apic::init(&runtime_table);
    interrupts::set_interrupt_timer_ready(apic::timer_routing_ready());
    zram::init();

    let mut executor = Executor::new();
    
    // 1. Spawn base async initialization (PCIe, GPU, ZRAM) decoupled from the main thread
    executor.spawn(Task::new(async_pcie_probe_and_init()));
    executor.spawn(Task::new(async_runtime_diagnostics(features)));

    // 2. Spawn unlinked async worker modules dynamically scaled to CPU logical threads
    serial_println!(
        "TUFF-RADICAL-KERNEL: Spawning {} cooperative worker modules...",
        features.recommended_workers
    );
    for thread_id in 0..features.recommended_workers {
        executor.spawn(Task::new(async_worker_module(thread_id)));
    }

    if interrupts::interrupt_timer_ready() {
        serial_println!("TUFF-RADICAL-KERNEL: APIC timer routing online. Releasing Interrupt Seals...");
        x86_64::instructions::interrupts::enable();
    } else {
        serial_println!("TUFF-RADICAL-KERNEL: APIC timer routing pending. External IRQs stay masked; cooperative scheduler fallback active.");
    }
    serial_println!("TUFF-RADICAL-KERNEL: OS Tick Active. Entering Async Executor loop.");

    executor.run();
}

async fn async_worker_module(thread_id: u32) {
    serial_println!("TUFF-RADICAL-ASYNC [WORKER-{}]: Online. Awaiting Vulkan/SIMD tasks.", thread_id);
    let base_sleep = 50 + (thread_id as u64 * 15); // Unlinked heartbeat timings (無動機秘連動)
    loop {
        SleepFuture::new(base_sleep).await;
        let current_tick = interrupts::TICKS.load(Ordering::Relaxed);
        if current_tick.is_multiple_of(1000) {
            serial_println!("TUFF-RADICAL-ASYNC [WORKER-{}]: Heartbeat. OS Tick: {}", thread_id, current_tick);
        }
    }
}

async fn async_pcie_probe_and_init() {
    serial_println!("TUFF-RADICAL-ASYNC [INIT]: Asynchronous PCIe probing for GPU/Storage...");
    let mut gpu_mmio_base: Option<u64> = None;
    let mut storage_device: Option<VirtioBlk> = None;

    for bus in 0..=255 {
        for slot in 0..=31 {
            let address_base = PciAddress { bus, slot, func: 0 };
            let dev_base = unsafe { drivers::pci::probe_device(address_base) };
            if dev_base.is_none() { continue; }
            
            for func in 0..=7 {
                let address = PciAddress { bus, slot, func };
                let Some(dev) = (unsafe { drivers::pci::probe_device(address) }) else { continue; };
                
                if dev.class == 0x03 { // Display Controller
                    if let Some(bar0) = unsafe { read_pci_bar0(address) } {
                        gpu_mmio_base = Some(bar0);
                        unsafe { drivers::gpu::test_draw(bar0); }
                        if let Some(vgpu) = unsafe { drivers::virtio_gpu::VirtioGpu::from_pci(address) } {
                            unsafe { vgpu.init(); }
                        }
                    }
                }

                if dev.vendor_id == 0x1AF4 { // VirtIO
                    if dev.device_id == 0x1001 { // Block
                        if let Some(device) = unsafe { VirtioBlk::from_pci(address) } {
                            storage_device = Some(device);
                        }
                    }
                }
            }
        }
        if bus % 32 == 0 { SleepFuture::new(1).await; }
    }

    if let Some(base) = gpu_mmio_base {
        serial_println!("TUFF-RADICAL-ASYNC [INIT]: GPU Active at 0x{:x}. Submitting Vulkan-compatible pipeline.", base);
        let ring = GpuCommandRing::new(base);
        async_gpu_compute_task(ring).await;
    }

    if let Some(disk) = storage_device {
        async_install_task(disk).await;
    }
}

async fn async_runtime_diagnostics(features: cpu::CpuFeatures) {
    SleepFuture::new(5).await;
    serial_println!(
        "TUFF-RADICAL-ASYNC [CPU]: workers={} simd={} avx={} avx512={} xcr0={:#x}",
        features.recommended_workers,
        features.simd_enabled,
        features.avx_enabled,
        features.avx512_enabled,
        features.xcr0
    );
}

async fn async_gpu_compute_task(mut ring: GpuCommandRing) {
    serial_println!("TUFF-RADICAL-ASYNC [GPU]: Vulkan compute sequence isolated.");
    SleepFuture::new(10).await;
    serial_println!("TUFF-RADICAL-ASYNC [GPU]: Submitting compute shader to Command Ring.");
    ring.submit_compute_command(0x70FF, 0x3000000);
}

async fn async_install_task(disk: VirtioBlk) {
    serial_println!("TUFF-RADICAL-ASYNC [INSTALL-TASK]: Beginning automated deployment...");
    SleepFuture::new(30).await; 
    installer::run_install_pipeline(&disk);
    serial_println!("TUFF-RADICAL-ASYNC [INSTALL-TASK]: Deployment finalized. System ready.");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("\n[!!!] TUFF-RADICAL-KERNEL T-RAD PANIC [!!!]");
    serial_println!("Nature: {}", info);
    serial_println!("System halted. The core remains pure.");
    loop { unsafe { asm!("hlt"); } }
}

unsafe fn read_pci_bar0(address: PciAddress) -> Option<u64> {
    match drivers::pci::read_bar(address, 0)? {
        PciBar::Memory32 { base, .. } | PciBar::Memory64 { base, .. } => Some(base),
        PciBar::Io { .. } => None,
    }
}
