pub const SECTOR_SIZE: usize = 512;

pub trait BlockDevice: Send + Sync {
    fn capacity_sectors(&self) -> u64;
    fn capacity_bytes(&self) -> u64 {
        self.capacity_sectors().saturating_mul(SECTOR_SIZE as u64)
    }

    fn read_sector(&self, sector: u64, buffer: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str>;
    fn write_sector(&self, sector: u64, buffer: &[u8; SECTOR_SIZE]) -> Result<(), &'static str>;
}
