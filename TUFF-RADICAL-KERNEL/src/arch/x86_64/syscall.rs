use core::ptr;
use crate::arch::x86_64::registers::Msr;
use crate::arch::x86_64::gdt;



// MSRs for SYSCALL/SYSRET
const STAR: Msr = Msr::new(0xC0000081);
const LSTAR: Msr = Msr::new(0xC0000082);
const SFMASK: Msr = Msr::new(0xC0000084);

pub unsafe fn init() {
    // 1. STAR: Segments for SYSCALL/SYSRET
    // bits 32-47: Kernel CS (data is CS+8)
    // bits 48-63: User CS (data is CS-8 / in our GDT user_code is after user_data, but standard is different. 
    // Standard: Kernel CS: 0x08, Kernel SS: 0x10, User SS: 0x1B, User CS: 0x23)
    let selectors = gdt::get_selectors();
    let star_val = (selectors.kernel_code_selector.0 as u64) << 32 
                 | (selectors.user_data_selector.0 as u64 | 3) << 48;
    STAR.write(star_val);

    // 2. LSTAR: Entry point for SYSCALL
    LSTAR.write(syscall_entry as *const () as u64);

    // 3. SFMASK: Flags to clear on syscall (IF=9, VM=17, etc.)
    SFMASK.write(1 << 9); // Mask Interrupts

    // 4. EFER: Enable SCE (System Call Extensions)
    let mut efer = crate::arch::x86_64::registers::EFER.read();
    efer |= 1; // SCE bit
    crate::arch::x86_64::registers::EFER.write(efer);

    // 5. Setup GS base for stack switching
    let kernel_stack = crate::mm::memory::allocate_contiguous_pages(5).unwrap() + (5 * 4096);
    static mut GS_STRUCT: [u64; 4] = [0; 4]; // [0]=Kernel RSP, [1]=Scratch, [2]=User RSP, [3]=Scratch
    GS_STRUCT[0] = kernel_stack;
    crate::arch::x86_64::registers::GS_BASE.write(ptr::addr_of!(GS_STRUCT) as u64);

    serial_println!("TUFF-RADICAL-SYSCALL: SCE enabled. Entry point at 0x{:x}", syscall_entry as *const () as u64);
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        "swapgs",               // Switch to Kernel GS
        "mov gs:16, rsp",       // Save user RSP (requires GS base to point to a valid struct, we'll setup soon)
        "mov rsp, gs:0",        // Load kernel RSP
        
        "push rcx",             // Save User RIP (RCX)
        "push r11",             // Save RFLAGS (R11)
        
        "call handle_syscall",  // Handle it
        
        "pop r11",              // Restore RFLAGS
        "pop rcx",              // Restore User RIP
        
        "mov rsp, gs:16",       // Restore user RSP
        "swapgs",               // Switch back to User GS
        "sysretq",              // Return to Ring 3
        
    );
}

#[no_mangle]
extern "C" fn handle_syscall(id: u64, arg1: u64) {
    serial_println!("TUFF-RADICAL-SYSCALL: Request received! ID={} Arg1={}", id, arg1);
}
