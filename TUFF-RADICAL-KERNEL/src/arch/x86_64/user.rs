use crate::arch::x86_64::gdt;

use crate::mm::memory;

use core::arch::asm;


pub unsafe fn jump_to_user(code_ptr: u64, stack_ptr: u64) -> ! {
    let selectors = gdt::get_selectors();
    
    // We use IRETQ to switch to Ring 3.
    // Stack layout for IRETQ:
    // SS (User Data)
    // RSP (User Stack)
    // RFLAGS
    // CS (User Code)
    // RIP (User Entry)
    
    let ds = selectors.user_data_selector.0 as u64 | 3;
    let cs = selectors.user_code_selector.0 as u64 | 3;
    let rflags: u64 = 0x202; // IF=1, fixed 1

    asm!(
        "mov ds, ax",
        "mov es, ax",
        "mov fs, ax",
        "mov gs, ax",
        
        "push {ss}",
        "push {rsp}",
        "push {rflags}",
        "push {cs}",
        "push {rip}",
        "iretq",
        ss = in(reg) ds,
        rsp = in(reg) stack_ptr,
        rflags = in(reg) rflags,
        cs = in(reg) cs,
        rip = in(reg) code_ptr,
        in("ax") ds,
        options(noreturn)
    );
}

pub fn spawn_user_hello() {
    serial_println!("TUFF-RADICAL-USER: Preparing Hello World app in Ring 3...");
    
    // 1. Allocate pages for code and stack
    let code_phys = memory::allocate_page().expect("Failed to allocate user code page");
    let stack_phys = memory::allocate_page().expect("Failed to allocate user stack page");
    
    // 2. Map them as USER_ACCESSIBLE
    // For now, we reuse current address space but add USER flag to these pages.
    // In a real microkernel, we'd have a separate PML4 per process.
    unsafe {
        crate::arch::x86_64::paging::map_user_code(0x1000_0000, code_phys);
        crate::arch::x86_64::paging::map_user_data(0x2000_0000, stack_phys);
    }
    
    // 3. Copy user code into the code page
    // Simple loop: SYSCALL (exit), then HLT
    let code = unsafe { core::slice::from_raw_parts_mut(0x1000_0000 as *mut u8, 4096) };
    
    // x86_64 assembly for:
    // mov rax, 0xCAFE  (Syscall ID)
    // mov rdi, 0x1337  (Arg 1)
    // syscall
    // jmp $ (loop forever)
    
    let program = [
        0x48, 0xC7, 0xC0, 0xFE, 0xCA, 0x00, 0x00, // mov rax, 0xCAFE
        0x48, 0xC7, 0xC7, 0x37, 0x13, 0x00, 0x00, // mov rdi, 0x1337
        0x0F, 0x05,                               // syscall
        0xEB, 0xFE                                // jmp -2
    ];
    code[..program.len()].copy_from_slice(&program);
    
    serial_println!("TUFF-RADICAL-USER: Handoff to Ring 3. Entering unprivileged mode.");
    unsafe {
        jump_to_user(0x1000_0000, 0x2000_0000 + 4096);
    }
}
