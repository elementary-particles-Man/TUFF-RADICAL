use alloc::vec::Vec;
use core::arch::x86_64::_rdtsc;
use core::sync::atomic::{AtomicBool, Ordering};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use spin::Mutex;
use crate::snappy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressionBackend {
    Snappy,
    Lz4,
}

impl CompressionBackend {
    fn as_str(self) -> &'static str {
        match self {
            CompressionBackend::Snappy => "snappy",
            CompressionBackend::Lz4 => "lz4",
        }
    }
}

#[derive(Clone)]
struct CompressedBlock {
    backend: CompressionBackend,
    data: Vec<u8>,
}

#[derive(Clone, Copy)]
struct BackendBenchmark {
    input_bytes: usize,
    compressed_bytes: usize,
    compress_cycles: u64,
    decompress_cycles: u64,
}

/// ベアメタル環境におけるインメモリ圧縮プール
/// 既定は Google 系の Snappy backend、比較用に LZ4 fallback を持つ。
pub struct TRadZram {
    default_backend: CompressionBackend,
    total_uncompressed_bytes: usize,
    total_compressed_bytes: usize,
    blocks: Vec<CompressedBlock>,
}

lazy_static::lazy_static! {
    pub static ref ZRAM_POOL: Mutex<TRadZram> = Mutex::new(TRadZram::new(CompressionBackend::Snappy));
}

static ZRAM_INITIALIZED: AtomicBool = AtomicBool::new(false);

impl TRadZram {
    pub fn new(default_backend: CompressionBackend) -> Self {
        Self {
            default_backend,
            total_uncompressed_bytes: 0,
            total_compressed_bytes: 0,
            blocks: Vec::new(),
        }
    }

    pub fn default_backend(&self) -> CompressionBackend {
        self.default_backend
    }

    #[allow(dead_code)]
    pub fn set_default_backend(&mut self, backend: CompressionBackend) {
        self.default_backend = backend;
    }

    /// データを既定 backend で圧縮してプールに格納し、ブロックIDを返す
    pub fn store(&mut self, data: &[u8]) -> usize {
        self.store_with_backend(data, self.default_backend)
    }

    pub fn store_with_backend(&mut self, data: &[u8], backend: CompressionBackend) -> usize {
        let compressed = compress_block(backend, data);

        self.total_uncompressed_bytes += data.len();
        self.total_compressed_bytes += compressed.len();

        let id = self.blocks.len();
        self.blocks.push(CompressedBlock {
            backend,
            data: compressed,
        });

        id
    }

    /// 指定されたブロックIDのデータを伸張して返す
    pub fn load(&self, id: usize) -> Option<Vec<u8>> {
        let block = self.blocks.get(id)?;
        match decompress_block(block.backend, &block.data) {
            Ok(decompressed) => Some(decompressed),
            Err(err) => {
                serial_println!(
                    "TUFF-RADICAL-COMMANDER [ZRAM]: FATAL Decompression Error at Block ID {} using {}: {}",
                    id,
                    block.backend.as_str(),
                    err
                );
                None
            }
        }
    }

    pub fn print_stats(&self) {
        if self.total_uncompressed_bytes == 0 {
            serial_println!("TUFF-RADICAL-ZRAM: Pool is empty.");
            return;
        }

        let ratio =
            (self.total_uncompressed_bytes as f64 / self.total_compressed_bytes as f64) * 100.0;
        serial_println!(
            "TUFF-RADICAL-ZRAM STATS: backend={} | {} blocks | Uncompressed: {} B | Compressed: {} B | Ratio: {:.2}%",
            self.default_backend.as_str(),
            self.blocks.len(),
            self.total_uncompressed_bytes,
            self.total_compressed_bytes,
            ratio
        );
    }
}

fn compress_block(backend: CompressionBackend, data: &[u8]) -> Vec<u8> {
    match backend {
        CompressionBackend::Snappy => snappy::compress(data),
        CompressionBackend::Lz4 => compress_prepend_size(data),
    }
}

fn decompress_block(backend: CompressionBackend, data: &[u8]) -> Result<Vec<u8>, &'static str> {
    match backend {
        CompressionBackend::Snappy => snappy::decompress(data).map_err(|_| "snappy decode failed"),
        CompressionBackend::Lz4 => {
            decompress_size_prepended(data).map_err(|_| "lz4 decode failed")
        }
    }
}

fn benchmark_backend(
    backend: CompressionBackend,
    sample: &[u8],
) -> Result<BackendBenchmark, &'static str> {
    let compress_start = unsafe { _rdtsc() };
    let compressed = compress_block(backend, sample);
    let compress_cycles = unsafe { _rdtsc() } - compress_start;

    let decompress_start = unsafe { _rdtsc() };
    let decompressed = decompress_block(backend, &compressed)?;
    let decompress_cycles = unsafe { _rdtsc() } - decompress_start;

    if decompressed.as_slice() != sample {
        return Err("roundtrip mismatch");
    }

    Ok(BackendBenchmark {
        input_bytes: sample.len(),
        compressed_bytes: compressed.len(),
        compress_cycles,
        decompress_cycles,
    })
}

fn log_backend_benchmark(
    backend: CompressionBackend,
    benchmark: BackendBenchmark,
) {
    serial_println!(
        "=> {} benchmark: {} B -> {} B | compress={} cycles | decompress={} cycles",
        backend.as_str(),
        benchmark.input_bytes,
        benchmark.compressed_bytes,
        benchmark.compress_cycles,
        benchmark.decompress_cycles
    );
}

fn compare_backends(sample: &[u8], label: &str) {
    serial_println!(
        "TUFF-RADICAL-COMMANDER [ZRAM-BENCH]: comparing backends on {} sample ({} bytes)",
        label,
        sample.len()
    );

    match benchmark_backend(CompressionBackend::Snappy, sample) {
        Ok(snappy) => log_backend_benchmark(CompressionBackend::Snappy, snappy),
        Err(err) => serial_println!("=> snappy benchmark FAILED: {}", err),
    }

    match benchmark_backend(CompressionBackend::Lz4, sample) {
        Ok(lz4) => log_backend_benchmark(CompressionBackend::Lz4, lz4),
        Err(err) => serial_println!("=> lz4 benchmark FAILED: {}", err),
    }
}

pub fn init() {
    if ZRAM_INITIALIZED.swap(true, Ordering::SeqCst) {
        serial_println!("TUFF-RADICAL-COMMANDER [ZRAM-01]: T-RAD ZRAM pool already active.");
        return;
    }

    serial_println!(
        "TUFF-RADICAL-COMMANDER [ZRAM-01]: Initializing T-RAD compression pool (default backend: snappy, fallback: lz4)..."
    );

    let dummy_data = [0xAA; 4096];
    let patterned_data: Vec<u8> = (0..4096).map(|i| (i % 251) as u8).collect();
    let texty_data = b"TUFF-RADICAL-SNAPPY-TUFF-RADICAL-SNAPPY-TUFF-RADICAL-SNAPPY-TUFF-RADICAL-SNAPPY".repeat(64);

    match benchmark_backend(CompressionBackend::Snappy, &dummy_data) {
        Ok(benchmark) => {
            serial_println!("=> Snappy Self-test Passed.");
            log_backend_benchmark(CompressionBackend::Snappy, benchmark);
        }
        Err(err) => {
            serial_println!("=> Snappy Self-test FAILED: {}", err);
        }
    }

    match benchmark_backend(CompressionBackend::Lz4, &dummy_data) {
        Ok(benchmark) => {
            serial_println!("=> LZ4 Self-test Passed.");
            log_backend_benchmark(CompressionBackend::Lz4, benchmark);
        }
        Err(err) => {
            serial_println!("=> LZ4 Self-test FAILED: {}", err);
        }
    }

    compare_backends(&patterned_data, "patterned");
    compare_backends(&texty_data, "text");

    let mut pool = ZRAM_POOL.lock();
    let id = pool.store(&dummy_data);
    let retrieved = pool.load(id).unwrap();
    assert_eq!(dummy_data.as_slice(), retrieved.as_slice());
    serial_println!(
        "=> T-RAD Compression Pool Active. Default backend: {}",
        pool.default_backend().as_str()
    );
    pool.print_stats();
}
