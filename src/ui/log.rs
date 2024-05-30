use std::sync::OnceLock;

use log::{LevelFilter, Log, Metadata, Record, set_logger, set_max_level};

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        eprintln!("[{}] {}", record.metadata().level(), record.args())
    }

    fn flush(&self) {
        // noop
    }
}

impl Logger {
    pub fn init() {
        set_logger(LOGGER.get_or_init(|| Logger)).expect("Logger already set");
        set_max_level(LevelFilter::Trace);
    }
}