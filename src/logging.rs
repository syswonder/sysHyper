use core::fmt::{self, Write};

use log::{self, Level, LevelFilter, Log, Metadata, Record};
// use crate::mutex::{Hspinlock, HspinlockGuard};
use alloc::string::String;
use spin::Mutex;

use crate::{device::uart, uart_puts_1, uart_puts_c};

// static PRINT_LOCK: Hspinlock<()> = Hspinlock::new(());
static PRINT_LOCK: Mutex<()> = Mutex::new(());

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match c {
                '\n' => {
                    uart::console_putchar(b'\r');
                    uart::console_putchar(b'\n');
                }
                _ => uart::console_putchar(c as u8),
            }
        }
        Ok(())
    }
}
// pub fn print(args: fmt::Arguments) {
//     Stdout.write_str("try get lock");
//     if let _locked = PRINT_LOCK.try_lock() {
//         Stdout.write_str("get lock success");
//         Stdout.write_fmt(args).unwrap();
//     } else {
//         Stdout.write_str("Failed to acquire lock for PRINT_LOCK.");
//     }
// }


pub fn print(args: fmt::Arguments) {
    let _locked = PRINT_LOCK.lock();
    Stdout.write_fmt(args).unwrap();
}
/// print without line breaks
#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::logging::print(format_args!($fmt $(, $($arg)+)?));
    }
}
/// print with line breaks
#[macro_export]
macro_rules! println {
    () => { print!("\n") };
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::logging::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

macro_rules! with_color {
    ($color_code:expr, $($arg:tt)*) => {{
        format_args!("\u{1B}[{}m{}\u{1B}[m", $color_code as u8, format_args!($($arg)*))
    }};
}

#[repr(u8)]
#[allow(dead_code)]
enum ColorCode {
    Black = 30,
    Red = 31,
    Green = 32,
    Yellow = 33,
    Blue = 34,
    Magenta = 35,
    Cyan = 36,
    White = 37,
    BrightBlack = 90,
    BrightRed = 91,
    BrightGreen = 92,
    BrightYellow = 93,
    BrightBlue = 94,
    BrightMagenta = 95,
    BrightCyan = 96,
    BrightWhite = 97,
}

pub fn init() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
}

struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = record.level();
        let line = record.line().unwrap_or(0);
        let target = record.target();
        let cpu_id = crate::percpu::this_cpu_data().id;
        let level_color = match level {
            Level::Error => ColorCode::BrightRed,
            Level::Warn => ColorCode::BrightYellow,
            Level::Info => ColorCode::BrightGreen,
            Level::Debug => ColorCode::BrightCyan,
            Level::Trace => ColorCode::BrightBlack,
        };
        let args_color = match level {
            Level::Error => ColorCode::Red,
            Level::Warn => ColorCode::Yellow,
            Level::Info => ColorCode::Green,
            Level::Debug => ColorCode::Cyan,
            Level::Trace => ColorCode::BrightBlack,
        };
        print(with_color!(
            ColorCode::White,
            "[{} {}] {} {}\n",
            with_color!(level_color, "{:<5}", level),
            with_color!(ColorCode::White, "{}", cpu_id),
            with_color!(ColorCode::White, "({}:{})", target, line),
            with_color!(args_color, "{}", record.args()),
        ));
    }

    fn flush(&self) {}
}
