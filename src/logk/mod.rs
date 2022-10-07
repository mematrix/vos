//! Kernel log utility interfaces.

use log::{Log, Metadata, Record};


/// Init kernel log impl. Currently we simply use the UART device as the log output.
pub(crate) fn init() {
    match log::set_logger(&UART_LOGGER) {
        Ok(_) => { log::set_max_level(log::LevelFilter::Trace); }
        Err(_) => { println_k!("Init set logger failed!"); }
    }
}


struct UartLogger;

impl Log for UartLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if record.level() < log::Level::Info {
                println_k!("[{}][{}:{}]: {}",
                    record.level(),
                    record.file().unwrap_or("<NONE>"),
                    record.line().unwrap_or_default(),
                    record.args());
            } else {
                println_k!("[{}]: {}", record.level(), record.args());
            }
        }
    }

    fn flush(&self) {}
}

static UART_LOGGER: UartLogger = UartLogger;
