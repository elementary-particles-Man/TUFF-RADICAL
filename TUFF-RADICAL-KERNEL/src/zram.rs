use alloc::vec::Vec;
use spin::Mutex;
use lz4_flex::compress_prepend_size;
use lz4_flex::decompress_size_prepended;
use crate::serial_println;

/// ベアメタル環境におけるインメモリ圧縮プール (T-RAD ZRAM)
/// TUFF-RADICAL の Unique Queue (UQ) やページキャッシュを透過的に圧縮・伸張する。
pub struct TRadZram {
    total_uncompressed_bytes: usize,
    total_compressed_bytes: usize,
    blocks: Vec<Vec<u8>>,
}

lazy_static::lazy_static! {
    pub static ref ZRAM_POOL: Mutex<TRadZram> = Mutex::new(TRadZram::new());
}

impl TRadZram {
    pub fn new() -> Self {
        Self {
            total_uncompressed_bytes: 0,
            total_compressed_bytes: 0,
            blocks: Vec::new(),
        }
    }

    /// データを圧縮してプールに格納し、ブロックID（インデックス）を返す
    pub fn store(&mut self, data: &[u8]) -> usize {
        let compressed = compress_prepend_size(data);
        
        self.total_uncompressed_bytes += data.len();
        self.total_compressed_bytes += compressed.len();
        
        let id = self.blocks.len();
        self.blocks.push(compressed);
        
        id
    }

    /// 指定されたブロックIDのデータを伸張して返す
    pub fn load(&self, id: usize) -> Option<Vec<u8>> {
        if id >= self.blocks.len() {
            return None;
        }
        
        let compressed = &self.blocks[id];
        match decompress_size_prepended(compressed) {
            Ok(decompressed) => Some(decompressed),
            Err(_) => {
                serial_println!("TUFF-RADICAL-COMMANDER [ZRAM]: FATAL Decompression Error at Block ID {}", id);
                None
            }
        }
    }

    pub fn print_stats(&self) {
        if self.total_uncompressed_bytes == 0 {
            serial_println!("TUFF-RADICAL-ZRAM: Pool is empty.");
            return;
        }
        
        let ratio = (self.total_uncompressed_bytes as f64 / self.total_compressed_bytes as f64) * 100.0;
        serial_println!(
            "TUFF-RADICAL-ZRAM STATS: {} blocks | Uncompressed: {} B | Compressed: {} B | Ratio: {:.2}%",
            self.blocks.len(),
            self.total_uncompressed_bytes,
            self.total_compressed_bytes,
            ratio
        );
    }
}

pub fn init() {
    serial_println!("TUFF-RADICAL-COMMANDER [ZRAM-01]: Initializing Bare-metal T-RAD ZRAM Pool (LZ4-Flex)...");
    
    // ZRAMの動作テスト
    let mut pool = ZRAM_POOL.lock();
    let dummy_data = [0xAA; 4096]; // 高圧縮率のダミーデータ
    
    let id = pool.store(&dummy_data);
    let retrieved = pool.load(id).unwrap();
    
    assert_eq!(dummy_data.as_slice(), retrieved.as_slice());
    serial_println!("=> ZRAM Self-test Passed. 4KB Page Compression Active.");
    pool.print_stats();
}
