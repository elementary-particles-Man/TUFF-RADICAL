use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::gdt;
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
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

        for i in 32..256 {
            if i == InterruptIndex::Timer.as_usize() { continue; }
            idt[i].set_handler_fn(generic_ignore_handler);
        }

        idt
    };
}

pub fn init_idt() {
    IDT.load();
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

use core::sync::atomic::{AtomicU64, Ordering};

pub static TICKS: AtomicU64 = AtomicU64::new(0);

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TICKS.fetch_add(1, Ordering::Relaxed);
    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn generic_ignore_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        let mut pics = PICS.lock();
        pics.notify_end_of_interrupt(PIC_2_OFFSET); // Slave
        pics.notify_end_of_interrupt(PIC_1_OFFSET); // Master 
    }
}
