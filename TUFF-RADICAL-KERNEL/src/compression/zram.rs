
use spin::Mutex;
use alloc::vec::Vec;
use alloc::vec;
use alloc::boxed::Box;
use crate::compression::{Compressor, SnappyCompressor};

pub struct ZramPool {
    backend: Box<dyn Compressor>,
    blocks: Vec<ZramBlock>,
    total_uncompressed_bytes: usize,
    total_compressed_bytes: usize,
}

struct ZramBlock {
    data: Vec<u8>,
    #[allow(dead_code)]
    uncompressed_len: usize,
}

static ZRAM: Mutex<Option<ZramPool>> = Mutex::new(None);

pub fn init() {
    serial_println!("TUFF-RADICAL-COMMANDER [ZRAM-01]: Initializing T-RAD compression pool (default: snappy)...");
    
    let pool = ZramPool {
        backend: Box::new(SnappyCompressor),
        blocks: Vec::new(),
        total_uncompressed_bytes: 0,
        total_compressed_bytes: 0,
    };

    // Self-test
    let test_data = b"TUFF-RADICAL-ZRAM-SELF-TEST-PATTERN-BASE-0123456789ABCDEF";
    let mut compressed = vec![0u8; pool.backend.max_compressed_len(test_data.len())];
    if let Ok(c_len) = pool.backend.compress(test_data, &mut compressed) {
        let mut decompressed = vec![0u8; test_data.len()];
        if let Ok(d_len) = pool.backend.decompress(&compressed[..c_len], &mut decompressed) {
            if &decompressed[..d_len] == test_data {
                serial_println!("=> ZRAM: Backend {} initialized and verified.", pool.backend.name());
            }
        }
    }

    *ZRAM.lock() = Some(pool);
}

#[allow(dead_code)]
pub fn store(data: &[u8]) -> Result<usize, &'static str> {
    let mut pool_lock = ZRAM.lock();
    let pool = pool_lock.as_mut().ok_or("ZRAM not initialized")?;

    let mut compressed = vec![0u8; pool.backend.max_compressed_len(data.len())];
    let c_len = pool.backend.compress(data, &mut compressed)?;
    compressed.truncate(c_len);

    let id = pool.blocks.len();
    pool.blocks.push(ZramBlock {
        data: compressed,
        uncompressed_len: data.len(),
    });

    pool.total_uncompressed_bytes += data.len();
    pool.total_compressed_bytes += c_len;

    Ok(id)
}

#[allow(dead_code)]
pub fn load(id: usize, buffer: &mut [u8]) -> Result<usize, &'static str> {
    let mut pool_lock = ZRAM.lock();
    let pool = pool_lock.as_mut().ok_or("ZRAM not initialized")?;

    let block = pool.blocks.get(id).ok_or("Invalid block ID")?;
    pool.backend.decompress(&block.data, buffer)
}

#[allow(dead_code)]
pub fn log_stats() {
    if let Some(pool) = ZRAM.lock().as_ref() {
        let ratio = if pool.total_compressed_bytes > 0 {
            (pool.total_uncompressed_bytes as f32 / pool.total_compressed_bytes as f32) * 100.0
        } else {
            0.0
        };
        serial_println!(
            "TUFF-RADICAL-ZRAM: backend={} blocks={} uncompressed={} compressed={} ratio={:.2}%",
            pool.backend.name(),
            pool.blocks.len(),
            pool.total_uncompressed_bytes,
            pool.total_compressed_bytes,
            ratio
        );
    }
}
