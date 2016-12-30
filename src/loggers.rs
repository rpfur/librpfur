use log::{Log, LogRecord, LogMetadata, LogLevel};

/// The maximum log level all `setup_default()` funcions will be using
pub const DEFAULT_MAX_LOG_LEVEL: LogLevel = LogLevel::Trace;

/// Formats a LogRecord into a String
fn _log_format(message: &LogRecord) -> String{
    format!("[{}][{}:{}][{}] {}",
        message.level(),
        message.location().file(),
        message.location().line(),
        message.target(),
        message.args()
    )
}

/// A basic logger that will output logs of any level to stderr
pub struct StdLogger(LogLevel);
impl Log for StdLogger{
    fn enabled(&self, metadata: &LogMetadata) -> bool{
        // Print the message if its log level is higher than self.0
        metadata.level() >= self.0
    }
    fn log(&self, message: &LogRecord){
        use std::io::{self, Write};
        let mut target = io::stderr();

        // Try to write to the standard error stream, and in case of failure write the log directly to standard output
        if let Err(what) = writeln!(target, "{}", _log_format(message)){
            println!("Stderr is unreachable! Please consider using another logger: {:?}", what);
            println!("{}", _log_format(message));
        }
    }
}
impl StdLogger{
    /// Set this logger as a the current one
    pub fn setup(max_level: LogLevel){
        use log;
        use std::boxed::Box;
        if let Err(what) = log::set_logger(|max_log_level|{
            max_log_level.set(max_level.clone().to_log_level_filter());
            Box::new(StdLogger(max_level))
        }){ println!("Failed to seup StdLogger: {:?}", what); }
    }

    pub fn setup_default(){
        StdLogger::setup(DEFAULT_MAX_LOG_LEVEL);
    }
}

/// A logger that will output to a given file
use std::fs::File;
use std::sync::Mutex;
pub struct FileLogger(LogLevel, Mutex<File>);
impl Log for FileLogger{
    fn enabled(&self, metadata: &LogMetadata) -> bool{
        // Print the message if its log level is higher than self.0
        metadata.level() >= self.0
    }
    fn log(&self, message: &LogRecord){
        use std::io::Write;

        /// Format the massage and write its bytes onto the file
        match self.1.lock(){
            Ok(mut stream) => if let Err(what) = writeln!(stream, "{}", _log_format(message)){
                println!("Could not write to logfile: {:?}", what);
                println!("{}", _log_format(message));
            },
            Err(what) => {
                println!("Could not write to logfile: {:?}", what);
                println!("{}", _log_format(message));
            }
        }
    }
}
impl FileLogger{
    /// Set this logger as a the current one
    pub fn setup(file: File, max_level: LogLevel){
        use log;
        use std::boxed::Box;
        if let Err(what) = log::set_logger(|max_log_level|{
            max_log_level.set(max_level.clone().to_log_level_filter());
            Box::new(FileLogger(max_level, Mutex::new(file)))
        }){ println!("Failed to seup FileLogger: {:?}", what); }
    }

    pub fn setup_default(file: File){
        FileLogger::setup(file, DEFAULT_MAX_LOG_LEVEL);
    }
}
