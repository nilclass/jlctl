use std::io::Write;
use std::sync::{Arc, Mutex};
use std::{fs::File, path::Path};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

/// Trait for logging device activity
pub trait DeviceLogger: Clone + Send + 'static {
    /// Serial port was opened
    fn open(&self, path: &str);

    /// Received a line from the board
    fn received(&self, line: &str);

    /// Sent a line to the board
    fn sent(&self, line: &str);
}

/// A [`DeviceLogger`] that discards all events
#[derive(Copy, Clone)]
pub struct NullLogger;

impl DeviceLogger for NullLogger {
    fn open(&self, _path: &str) {}

    fn received(&self, _line: &str) {}

    fn sent(&self, _line: &str) {}
}

/// A [`DeviceLogger`] that writes events to a file
#[derive(Clone)]
pub struct FileLogger(Arc<Mutex<File>>);

impl FileLogger {
    /// Construct a new FileLogger
    ///
    /// Opens the file at given `path`, and appends any logged events to it.
    pub fn new<P: AsRef<Path>>(path: P) -> std::io::Result<FileLogger> {
        let file = File::options().create(true).append(true).open(path)?;
        Ok(FileLogger(Arc::new(Mutex::new(file))))
    }

    fn write_line(&self, tag: &str, arg: &str) {
        let file = &mut self.0.lock().unwrap();
        writeln!(
            file,
            "[{}] {} {}",
            OffsetDateTime::now_utc().format(&Iso8601::DEFAULT).unwrap(),
            tag,
            arg,
        )
        .expect("Write to log file");
    }
}

impl DeviceLogger for FileLogger {
    fn open(&self, path: &str) {
        self.write_line("OPEN", path);
    }

    fn received(&self, line: &str) {
        self.write_line("RECV", line);
    }

    fn sent(&self, line: &str) {
        self.write_line("SEND", line);
    }
}
