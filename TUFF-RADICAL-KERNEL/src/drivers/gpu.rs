use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;

/// Vulkan 互換のコマンドをベアメタルで投げるためのリングバッファ (Ring Buffer) の概念実証
pub struct GpuCommandRing {
    mmio_base: u64,
    ring_buffer: Vec<u32>, // 本来は物理連続メモリ
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl GpuCommandRing {
    pub fn new(mmio_base: u64) -> Self {
        serial_println!("TUFF-RADICAL-COMMANDER [GPU-02]: Establishing Command Ring at MMIO 0x{:x}", mmio_base);
        Self {
            mmio_base,
            // 1024エントリのコマンドキュー
            ring_buffer: alloc::vec![0; 1024],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    /// GPUへ非同期的に計算コマンド (Compute Shader) を投入する
    pub fn submit_compute_command(&mut self, shader_id: u32, data_ptr: u64) {
        let current_tail = self.tail.load(Ordering::Relaxed);
        let next_tail = (current_tail + 2) % 1024;
        
        if next_tail == self.head.load(Ordering::Acquire) {
            serial_println!("=> GPU Ring Buffer FULL! Cannot submit.");
            return;
        }

        // コマンドの書き込み (Opcode 0x01 = Compute, Data = Ptr)
        self.ring_buffer[current_tail] = 0x01_00_00_00 | shader_id;
        self.ring_buffer[(current_tail + 1) % 1024] = (data_ptr & 0xFFFFFFFF) as u32;

        self.tail.store(next_tail, Ordering::Release);

        // GPU の Doorbell レジスタ (仮: オフセット 0x40) を叩いて起床させる
        let doorbell = (self.mmio_base + 0x40) as *mut u32;
        let _ = doorbell;

        serial_println!("TUFF-RADICAL-COMMANDER [GPU-03]: Compute Command submitted (Shader: {}, Data: 0x{:x}). Doorbell rung.", shader_id, data_ptr);
    }
}

pub unsafe fn test_draw(framebuffer_phys_addr: u64) {
    let fb_addr = framebuffer_phys_addr & !0xF;
    serial_println!("TUFF-RADICAL-COMMANDER [GPU-01]: Directly taking control of Framebuffer at 0x{:x}", fb_addr);

    let fb_ptr = fb_addr as *mut u32;
    let color: u32 = 0x00FF2222; 
    
    // Uncacheable設定による遅延を防ぐため、画面上部の「赤い帯」のみを描画
    // (1024 * 10 ピクセル)
    for i in 0..(1024 * 10) {
        fb_ptr.add(i).write_volatile(color);
    }

    serial_println!("=> Raw Framebuffer partial fill completed. Red Alert established.");
}
