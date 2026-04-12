#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use uefi::prelude::*;
use core::arch::asm;

mod memory;
mod paging;
mod gpu;
mod cpu;
#[macro_use]
mod serial;
mod gdt;
mod interrupts;
mod task;
mod zram;
mod virtio_blk;

use task::{Task, executor::Executor};
use crate::gpu::GpuCommandRing;
use crate::virtio_blk::VirtioBlk;
use core::{future::Future, pin::Pin, task::{Context, Poll}};
use core::sync::atomic::Ordering;

// --- 非同期スリープの実装 ---
struct SleepFuture {
    target_tick: u64,
}

impl SleepFuture {
    fn new(ticks: u64) -> Self {
        let current = interrupts::TICKS.load(Ordering::Relaxed);
        SleepFuture { target_tick: current + ticks }
    }
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        let current = interrupts::TICKS.load(Ordering::Relaxed);
        if current >= self.target_tick {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[entry]
fn main(_image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    unsafe { serial::COM1.lock().init(); }
    serial_println!("--- TUFF-RADICAL-KERNEL SOVEREIGN REBIRTH (FINAL TUNE) ---");

    memory::init_memory(&system_table);
    unsafe { paging::init_paging(); }
    
    // CPU Feature detection is required to determine the async scale topology
    let features = cpu::detect_features();
    cpu::log_features(&features);
    
    if !features.has_avx {
        serial_println!("TUFF-RADICAL-KERNEL: [WARNING] AVX missing. SIMD optimization degraded.");
    }

    serial_println!("TUFF-RADICAL-KERNEL: Asserting absolute control over CPU (GDT/IDT)...");
    x86_64::instructions::interrupts::disable();

    gdt::init();
    interrupts::init_idt();
    unsafe { 
        let mut pics = interrupts::PICS.lock();
        pics.initialize(); 
        pics.write_masks(0xFE, 0xFF);
    }

    let mut executor = Executor::new();
    
    // 1. Spawn base async initialization (PCIe, GPU, ZRAM) decoupled from the main thread
    executor.spawn(Task::new(async_pcie_probe_and_init()));

    // 2. Spawn unlinked async worker modules dynamically scaled to CPU logical threads
    serial_println!("TUFF-RADICAL-KERNEL: Spawning {} Unlinked Asynchronous Modules...", features.logical_threads);
    for thread_id in 0..features.logical_threads {
        executor.spawn(Task::new(async_worker_module(thread_id)));
    }

    serial_println!("TUFF-RADICAL-KERNEL: Sovereign Executive Stable. Releasing Interrupt Seals...");
    x86_64::instructions::interrupts::enable(); 
    serial_println!("TUFF-RADICAL-KERNEL: OS Tick Active. Entering Async Executor loop.");

    executor.run();
}

async fn async_worker_module(thread_id: u32) {
    serial_println!("TUFF-RADICAL-ASYNC [WORKER-{}]: Online. Awaiting Vulkan/SIMD tasks.", thread_id);
    let base_sleep = 50 + (thread_id as u64 * 15); // Unlinked heartbeat timings (無動機秘連動)
    loop {
        SleepFuture::new(base_sleep).await;
        let current_tick = interrupts::TICKS.load(Ordering::Relaxed);
        if current_tick % 1000 == 0 {
            serial_println!("TUFF-RADICAL-ASYNC [WORKER-{}]: Heartbeat. OS Tick: {}", thread_id, current_tick);
        }
    }
}

async fn async_pcie_probe_and_init() {
    serial_println!("TUFF-RADICAL-ASYNC [INIT]: Decoupled ZRAM pool initialization.");
    zram::init();

    serial_println!("TUFF-RADICAL-ASYNC [INIT]: Asynchronous PCIe probing for GPU/Storage...");
    let mut gpu_mmio_base: Option<u64> = None;
    let mut storage_device: Option<VirtioBlk> = None;

    for bus in 0..=255 {
        for slot in 0..=31 {
            let vendor_id = unsafe { read_pci_config(bus, slot, 0, 0) & 0xFFFF };
            if vendor_id == 0xFFFF { continue; }
            for func in 0..=7 {
                let id_reg = unsafe { read_pci_config(bus, slot, func, 0) };
                if (id_reg & 0xFFFF) == 0xFFFF { continue; }
                let class_reg = unsafe { read_pci_config(bus, slot, func, 0x08) };
                let class = (class_reg >> 24) & 0xFF;
                
                if class == 0x03 {
                    let bar0 = unsafe { read_pci_config(bus, slot, func, 0x10) };
                    gpu_mmio_base = Some((bar0 as u64) & !0xF);
                    unsafe { gpu::test_draw(gpu_mmio_base.unwrap()); }
                }

                // VirtIO (QEMU Storage)
                if (id_reg & 0xFFFF) == 0x1AF4 {
                    storage_device = Some(VirtioBlk::new(id_reg as u64));
                }
            }
        }
        if bus % 32 == 0 { SleepFuture::new(1).await; } // Yield control to workers
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

async fn async_gpu_compute_task(mut ring: GpuCommandRing) {
    serial_println!("TUFF-RADICAL-ASYNC [GPU]: Vulkan compute sequence isolated.");
    SleepFuture::new(10).await;
    serial_println!("TUFF-RADICAL-ASYNC [GPU]: Submitting compute shader to Command Ring.");
    ring.submit_compute_command(0x70FF, 0x3000000);
}

async fn async_install_task(disk: VirtioBlk) {
    serial_println!("TUFF-RADICAL-ASYNC [INSTALL-TASK]: Beginning automated deployment...");
    SleepFuture::new(30).await; 
    disk.perform_installation();
    serial_println!("TUFF-RADICAL-ASYNC [INSTALL-TASK]: Deployment finalized. System ready.");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("\n[!!!] TUFF-RADICAL-KERNEL SOVEREIGN PANIC [!!!]");
    serial_println!("Nature: {}", info);
    serial_println!("System halted. The core remains pure.");
    loop { unsafe { asm!("hlt"); } }
}

unsafe fn read_pci_config(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = 0x80000000 | ((bus as u32) << 16) | ((slot as u32) << 11) | ((func as u32) << 8) | (offset as u32 & 0xFC);
    asm!("out dx, eax", in("dx") 0xCF8_u16, in("eax") address);
    let mut data: u32;
    asm!("in eax, dx", out("eax") data, in("dx") 0xCFC_u16);
    data
}
