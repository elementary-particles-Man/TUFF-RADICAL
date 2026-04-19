use crate::drivers::io;

use core::fmt;

pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    pub const fn new(base: u16) -> Self {
        Self { base }
    }

    pub unsafe fn init(&self) {
        // COM1 ポートの初期化シーケンス (Baud rate 115200, 8n1)
        self.out_b(1, 0x00);    // Disable all interrupts
        self.out_b(3, 0x80);    // Enable DLAB (set baud rate divisor)
        self.out_b(0, 0x03);    // Set divisor to 3 (lo byte) 38400 baud
        self.out_b(1, 0x00);    //                  (hi byte)
        self.out_b(3, 0x03);    // 8 bits, no parity, one stop bit
        self.out_b(2, 0xC7);    // Enable FIFO, clear them, with 14-byte threshold
        self.out_b(4, 0x0B);    // IRQs enabled, RTS/DSR set
    }

    unsafe fn out_b(&self, offset: u16, data: u8) {
        io::outb(self.base + offset, data);
    }

    unsafe fn in_b(&self, offset: u16) -> u8 {
        io::inb(self.base + offset)
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe { (self.in_b(5) & 0x20) != 0 }
    }

    pub fn write_byte(&self, byte: u8) {
        while !self.is_transmit_empty() {}
        unsafe { self.out_b(0, byte); }
    }

    pub fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        SerialPort::write_str(self, s);
        Ok(())
    }
}

// TUFF-RADICAL-COMMANDER: Global Serial access for emergency logging
use spin::Mutex;
pub static COM1: Mutex<SerialPort> = Mutex::new(SerialPort::new(0x3F8));

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    COM1.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ($crate::drivers::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
