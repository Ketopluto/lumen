use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

/// Truncate the log once it passes this size, so it can't grow forever.
const MAX_BYTES: u64 = 512 * 1024;

/// One line per event in `lumen.log`, next to the database. Lumen usually runs with no window,
/// so this is the only way to see what happened at logon.
struct FileLogger {
    file: Mutex<std::fs::File>,
}

pub fn init() {
    let dir = crate::utils::app_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("lumen.log");
    if std::fs::metadata(&path).map(|m| m.len() > MAX_BYTES).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    let Ok(file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let logger: &'static FileLogger = Box::leak(Box::new(FileLogger { file: Mutex::new(file) }));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(
            file,
            "{} {:<5} {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        );
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}
