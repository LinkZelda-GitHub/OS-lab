use log::{Metadata, Record, LevelFilter, SetLoggerError};

pub fn init(log_level: &str) -> Result<(), SetLoggerError> {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER)?;

    // FIXED: Configure the logger
    match log_level {
        "Info" => log::set_max_level(log::LevelFilter::Info),
        "Debug" => log::set_max_level(log::LevelFilter::Debug),
        "Error" => log::set_max_level(log::LevelFilter::Error),
        "Off" => log::set_max_level(log::LevelFilter::Off),
        "Trace" => log::set_max_level(log::LevelFilter::Trace),
        "Warn" => log::set_max_level(log::LevelFilter::Warn),
        _ => log::set_max_level(log::LevelFilter::Info)
    }
    // 按照传入的参数设置 levelfilter, 默认为 Info

    info!("Logger Initialized.");
    Ok(())
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level() 
    }

    fn log(&self, record: &Record) {
        // FIXED: Implement the logger with serial output
        if self.enabled(record.metadata()) {
            // 格式化输出：级别 + 目标 + 消息内容
            println!("[{}] {} - {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}
