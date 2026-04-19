pub mod snappy;
pub mod zram;

pub trait Compressor: Send + Sync {
    fn name(&self) -> &'static str;
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, &'static str>;
    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, &'static str>;
    fn max_compressed_len(&self, input_len: usize) -> usize;
}

pub struct SnappyCompressor;
impl Compressor for SnappyCompressor {
    fn name(&self) -> &'static str { "snappy" }
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
        snappy::compress(input, output)
    }
    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
        snappy::decompress(input, output)
    }
    fn max_compressed_len(&self, input_len: usize) -> usize {
        snappy::max_compressed_len(input_len)
    }
}

#[allow(dead_code)]
pub struct Lz4Compressor;
impl Compressor for Lz4Compressor {
    fn name(&self) -> &'static str { "lz4" }
    #[allow(unused_variables)]
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
        Err("lz4 not yet decoupled")
    }
    #[allow(unused_variables)]
    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
        Err("lz4 not yet decoupled")
    }
    fn max_compressed_len(&self, input_len: usize) -> usize {
        input_len + (input_len / 255) + 16
    }
}
