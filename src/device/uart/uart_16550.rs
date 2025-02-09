use tock_registers::interfaces::*;
use tock_registers::register_structs;
use tock_registers::registers::*;
use crate::memory::addr::{PhysAddr, VirtAddr};


/// Register struct representing the UART registers.
register_structs! {
    /// Representation of the UART registers.
    #[allow(non_snake_case)]
    pub Ns16550a {
        (0x00 => pub THR_RBR_DLL: ReadWrite<u32>),
        (0x04 => pub IER_DLH: ReadWrite<u32>),
        (0x08 => pub IIR_FCR: ReadWrite<u32>),
        (0x0c => pub LCR: ReadWrite<u32>),
        (0x10 => pub MCR: ReadWrite<u32>),
        (0x14 => pub LSR: ReadOnly<u32>),
        (0x18 => pub MSR: ReadWrite<u32>),
        (0x1c => pub SR: ReadWrite<u32>),
        (0x20 => @END),
    }
}

#[allow(dead_code)]
pub struct Uart16550 {
    base_addr: usize,
}


impl Uart16550 {
    pub const fn new(base_addr: VirtAddr) -> Self {
        Self {
            base_addr
        }
    }
    
    #[inline]
    pub const fn regs(&self) -> &Ns16550a {
        unsafe { &*(self.base_addr as *const _) }
    }

    pub fn init(&mut self) {

        // self.regs().LCR.set(0x1 << 7);
        // self.regs().LCR.set(self.regs().LCR.get() & !(1 << 7));

        self.regs().LCR.set(0x3 << 0);
        self.regs().IER_DLH.set(0);
        self.regs().MCR.set(0);

        self.regs().LSR.get();
        self.regs().MSR.set(0);

        self.regs().IIR_FCR.set(0x1 << 0);
    }
    #[inline]
    pub fn putchar(&mut self, c: u8) {
        while self.regs().LSR.get() & (1 << 5) == 0 {}
        self.regs().THR_RBR_DLL.set(c as u32);

    }
    #[inline]
    fn getchar(&mut self) -> Option<u8> {
        todo!()
    }
}

// lazy_static! {
//     static ref UART: Mutex<Uart16550> = {
//         let mut uart = Uart16550::new(0xfe660000);
//         uart.init();
//         Mutex::new(uart)
//     };
// }

// pub fn console_putchar(c: u8) {
//     unsafe { UART.lock().putchar(c) }
// }

// pub fn console_getchar() -> Option<u8> {
//     unsafe { UART.lock().getchar() }
// }

#[cfg(feature = "platform_rk3568")]
pub const UART_BASE_PHYS: PhysAddr = 0xfe660000;

#[cfg(feature = "platform_rk3588")]
pub const UART_BASE_PHYS: PhysAddr = 0xfeb50000;

static mut UART: Uart16550 = {
    Uart16550::new(UART_BASE_PHYS)
};

#[inline]
pub fn console_putchar(c: u8) {
    unsafe { UART.putchar(c) }
}

#[inline]
pub fn console_getchar() -> Option<u8> {
    unsafe { UART.getchar() }
}


// use tock_registers::interfaces::{Readable, Writeable};
// use tock_registers::register_structs;
// use tock_registers::registers::{ReadOnly, ReadWrite, WriteOnly};
// use core::ptr;
// use crate::memory::addr::PhysAddr;

// pub const UART_BASE_PHYS: PhysAddr = 0xfe660000;

// pub fn console_init() {
//     unsafe {
//         ptr::write_volatile((UART_BASE_PHYS + 0x08) as *mut u32, 0x01);
//     }
// }

// pub fn console_putchar(c: u8) {
//     unsafe {
//         while ptr::read_volatile((UART_BASE_PHYS + 0x14) as *const u32) & (1<<5) == 0 {}

//         ptr::write_volatile(UART_BASE_PHYS as *mut u32, c as _);
//     }
// }

// pub fn console_getchar() -> Option<u8> {
//     todo!();
// }