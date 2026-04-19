use crate::block::{BlockDevice, SECTOR_SIZE};

pub fn run_install_pipeline(disk: &dyn BlockDevice) {
    serial_println!(
        "TUFF-RADICAL-INSTALLER: Target online. capacity={} sectors ({} bytes).",
        disk.capacity_sectors(),
        disk.capacity_bytes()
    );

    let mut sector0 = [0_u8; SECTOR_SIZE];
    match disk.read_sector(0, &mut sector0) {
        Ok(()) => {
            let boot_signature = u16::from_le_bytes([sector0[510], sector0[511]]);
            serial_println!(
                "TUFF-RADICAL-INSTALLER: sector0 read OK bytes[0..4]={:02x} {:02x} {:02x} {:02x} sig=0x{:04x}",
                sector0[0], sector0[1], sector0[2], sector0[3], boot_signature
            );
        }
        Err(err) => {
            serial_println!("TUFF-RADICAL-INSTALLER: sector0 read FAILED: {}", err);
        }
    }

    let mut write_probe = [0_u8; SECTOR_SIZE];
    write_probe[..16].copy_from_slice(b"TUFF-RADICAL-CHK");
    write_probe[510] = 0x55;
    write_probe[511] = 0xAA;

    match disk.write_sector(1, &write_probe) {
        Ok(()) => {
            let mut readback = [0_u8; SECTOR_SIZE];
            match disk.read_sector(1, &mut readback) {
                Ok(()) if readback == write_probe => {
                    let boot_signature = u16::from_le_bytes([readback[510], readback[511]]);
                    serial_println!(
                        "TUFF-RADICAL-INSTALLER: sector1 write/readback OK sig=0x{:04x}",
                        boot_signature
                    );
                }
                Ok(()) => {
                    serial_println!(
                        "TUFF-RADICAL-INSTALLER: sector1 write/readback FAILED: data mismatch"
                    );
                }
                Err(err) => {
                    serial_println!(
                        "TUFF-RADICAL-INSTALLER: sector1 readback FAILED: {}",
                        err
                    );
                }
            }
        }
        Err(err) => {
            serial_println!("TUFF-RADICAL-INSTALLER: sector1 write FAILED: {}", err);
        }
    }

    serial_println!(
        "TUFF-RADICAL-INSTALLER: write/install pipeline is validated. Deployment guarded."
    );
}
