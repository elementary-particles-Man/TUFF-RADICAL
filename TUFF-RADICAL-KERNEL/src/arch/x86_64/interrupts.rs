use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use alloc::vec::Vec;
use core::task::Waker;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::arch::x86_64::gdt;
use lazy_static::lazy_static;
use spin::Mutex;

pub const TIMER_VECTOR: u8 = 32;
pub const KEYBOARD_VECTOR: u8 = 33;
static INTERRUPT_TIMER_READY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = TIMER_VECTOR,
    Keyboard = KEYBOARD_VECTOR,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }
    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

        for i in 32..256 {
            if i == InterruptIndex::Timer.as_usize() || i == InterruptIndex::Keyboard.as_usize() { continue; }
            idt[i].set_handler_fn(generic_ignore_handler);
        }

        idt
    };
    static ref TIMER_WAITERS: Mutex<Vec<TimerWaiter>> = Mutex::new(Vec::new());
}

struct TimerWaiter {
    target_tick: u64,
    waker: Waker,
}

pub fn init_idt() {
    IDT.load();
}

pub fn current_tick() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn advance_cooperative_tick() -> u64 {
    let tick = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    wake_due_timer_waiters();
    tick
}

pub fn interrupt_timer_ready() -> bool {
    INTERRUPT_TIMER_READY.load(Ordering::SeqCst)
}

pub fn set_interrupt_timer_ready(ready: bool) {
    INTERRUPT_TIMER_READY.store(ready, Ordering::SeqCst);
}

pub fn register_timer_waker(target_tick: u64, waker: &Waker) {
    if current_tick() >= target_tick {
        waker.wake_by_ref();
        return;
    }

    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut waiters = TIMER_WAITERS.lock();
        if let Some(existing) = waiters
            .iter_mut()
            .find(|entry| entry.waker.will_wake(waker))
        {
            existing.target_tick = existing.target_tick.min(target_tick);
            existing.waker = waker.clone();
            return;
        }

        waiters.push(TimerWaiter {
            target_tick,
            waker: waker.clone(),
        });
    });
}

pub fn wake_due_timer_waiters() {
    let current = current_tick();
    let ready_wakers = x86_64::instructions::interrupts::without_interrupts(|| {
        let mut waiters = TIMER_WAITERS.lock();
        let mut ready = Vec::new();
        let mut index = 0;

        while index < waiters.len() {
            if waiters[index].target_tick <= current {
                ready.push(waiters.swap_remove(index).waker);
            } else {
                index += 1;
            }
        }

        ready
    });

    for waker in ready_wakers {
        waker.wake();
    }
}

extern "x86-interrupt" fn breakpoint_handler(_stack_frame: InterruptStackFrame) {
    serial_println!("EXCEPTION: BREAKPOINT");
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: x86_64::structures::idt::PageFaultErrorCode) {
    use x86_64::registers::control::Cr2;
    serial_println!("EXCEPTION: PAGE FAULT at {:?}", Cr2::read());
    serial_println!("Error Code: {:?}\n{:#?}", error_code, stack_frame);
    loop { unsafe { core::arch::asm!("hlt"); } }
}

extern "x86-interrupt" fn general_protection_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    serial_println!("EXCEPTION: GENERAL PROTECTION FAULT, Error Code: 0x{:x}", error_code);
    serial_println!("{:#?}", stack_frame);
    loop { unsafe { core::arch::asm!("hlt"); } }
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    serial_println!("!!!! CRITICAL: DOUBLE FAULT !!!!\n{:#?}", stack_frame);
    loop { unsafe { core::arch::asm!("hlt"); } }
}

pub static TICKS: AtomicU64 = AtomicU64::new(0);

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    wake_due_timer_waiters();
    crate::arch::x86_64::apic::end_of_interrupt();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    if scancode == 0x01 {
        serial_println!("\n[!!!] EMERGENCY STOP: Escape key detected. System frozen by Commander.");
        loop { unsafe { core::arch::asm!("hlt"); } }
    }

    serial_println!("TUFF-RADICAL-KERNEL [IRQ-33]: Keyboard scancode 0x{:02x}", scancode);

    crate::arch::x86_64::apic::end_of_interrupt();
}

extern "x86-interrupt" fn generic_ignore_handler(stack_frame: InterruptStackFrame) {
    // IDTから割り込みベクトルを特定することは難しいため、スタックフレームの情報などを出力
    serial_println!("TUFF-RADICAL-KERNEL [IRQ-IGNORE]: Unexpected interrupt occurred.");
    serial_println!("{:#?}", stack_frame);
    crate::arch::x86_64::apic::end_of_interrupt();
}
