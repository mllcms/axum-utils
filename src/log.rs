use std::{
    fmt::{Debug, Display},
    fs::File,
    io::Write,
    panic::Location,
    sync::mpsc::{channel, Sender},
    thread::{self, JoinHandle},
};

use chrono::{DateTime, Local};
use colored::{ColoredString, Colorize};
use once_cell::sync::Lazy;

use crate::utils::create_log_file;

static mut LOG: Lazy<Log> = Lazy::new(|| {
    let config = LogConfig {
        file_out: false,
        stdout: true,
        debug_dir: "logs/debug/%Y-%m-%d.log".into(),
        info_dir: "logs/info/%Y-%m-%d.log".into(),
        warn_dir: "logs/warn/%Y-%m-%d.log".into(),
        error_dir: "logs/error/%Y-%m-%d.log".into(),
    };
    let (sender, executor) = Log::create_executor(config.clone());
    Log {
        sender,
        executor,
        config,
    }
});

pub struct Log {
    sender: Sender<LogMsg>,
    executor: JoinHandle<()>,
    config: LogConfig,
}

unsafe impl Sync for Log {}

impl Log {
    #[track_caller]
    pub fn debug<M: Debug>(msg: M) {
        Self::send(Level::DEBUG, format!("{msg:?}"), Location::caller())
    }

    #[track_caller]
    pub fn info<M: Display>(msg: M) {
        Self::send(Level::INFO, msg.to_string(), Location::caller())
    }

    #[track_caller]
    pub fn warn<M: Display>(msg: M) {
        Self::send(Level::WARN, msg.to_string(), Location::caller())
    }

    #[track_caller]
    pub fn error<M: Display>(msg: M) {
        Self::send(Level::ERROR, msg.to_string(), Location::caller())
    }

    /// 修改日志配置
    /// # Examples
    /// ```no_run
    /// Log::config(|c| {
    ///     c.file_out = true;
    /// });
    /// ```
    pub fn config(f: fn(&mut LogConfig)) {
        unsafe {
            f(&mut LOG.config);
            let (sender, executor) = Self::create_executor(LOG.config.clone());
            LOG.sender = sender;
            LOG.executor = executor
        }
    }

    fn create_executor(config: LogConfig) -> (Sender<LogMsg>, JoinHandle<()>) {
        let mut time = Local::now();
        let mut log_file = config.file_out.then(|| LogFile::new(&config, &time));

        let (sender, rx) = channel::<LogMsg>();
        let executor = thread::spawn(move || {
            for log_msg in rx {
                let now = Local::now();

                if config.stdout {
                    log_msg.stdout()
                }

                if let Some(file) = log_file.as_mut() {
                    if time.date_naive() != now.date_naive() {
                        time = now;
                        *file = LogFile::new(&config, &time)
                    }
                    match log_msg.level {
                        Level::DEBUG => log_msg.file_out(&mut file.debug),
                        Level::INFO => log_msg.file_out(&mut file.info),
                        Level::WARN => log_msg.file_out(&mut file.warn),
                        Level::ERROR => log_msg.file_out(&mut file.error),
                    };
                }
            }
        });
        (sender, executor)
    }

    fn send(level: Level, msg: String, location: &'static Location<'static>) {
        let log_msg = LogMsg {
            msg,
            level,
            time: Local::now(),
            location,
        };
        if let Err(err) = unsafe { LOG.sender.send(log_msg) } {
            println!("日志记录失败: {err}")
        }
    }
}

#[derive(Clone)]
pub struct LogConfig {
    /// 是否输出到文件
    pub file_out: bool,

    /// 是否输出到控制台
    pub stdout: bool,

    /// debug 文件位置
    /// # Examples
    /// "logs/debog/%Y-%m-%d.log"
    pub debug_dir: String,

    /// info 文件位置
    /// # Examples
    /// "logs/info/%Y-%m-%d.log"
    pub info_dir: String,

    /// warn 文件位置
    /// # Examples
    /// "logs/warn/%Y-%m-%d.log"
    pub warn_dir: String,

    /// error 文件位置
    /// # Examples
    /// "logs/error/%Y-%m-%d.log"
    pub error_dir: String,
}

#[allow(dead_code)]
struct LogFile {
    debug: File,
    info: File,
    warn: File,
    error: File,
}

impl LogFile {
    fn new(config: &LogConfig, time: &DateTime<Local>) -> Self {
        Self {
            debug: create_log_file(time.format(&config.debug_dir).to_string()),
            info: create_log_file(time.format(&config.info_dir).to_string()),
            warn: create_log_file(time.format(&config.warn_dir).to_string()),
            error: create_log_file(time.format(&config.error_dir).to_string()),
        }
    }
}

/// 日志信息
struct LogMsg {
    msg: String,
    level: Level,
    time: DateTime<Local>,
    location: &'static Location<'static>,
}

impl LogMsg {
    fn stdout(&self) {
        println!(
            "[{}] {} {} {}",
            self.time
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .truecolor(127, 132, 142),
            self.level.color_string(),
            self.location.to_string().blue().underline(),
            self.msg
        )
    }

    fn file_out(&self, file: &mut File) {
        let msg = format!(
            "[{}] [{:<7?}] {} {}\n",
            self.time.format("%Y-%m-%d %H:%M:%S"),
            self.level,
            self.location,
            self.msg
        );

        if let Err(err) = file.write_all(msg.as_bytes()) {
            println!("日志写入文件时出错 -> {err}")
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum Level {
    DEBUG,
    INFO,
    WARN,
    ERROR,
}

impl Level {
    fn color_string(&self) -> ColoredString {
        match self {
            Level::DEBUG => "[DEBUG]".to_string().purple(),
            Level::INFO => "[INFO] ".to_string().blue(),
            Level::WARN => "[WARN] ".to_string().yellow(),
            Level::ERROR => "[ERROR]".to_string().red(),
        }
    }
}

#[test]
fn is_works() {
    Log::config(|c| c.file_out = true);
    Log::info("test");
    Log::debug("test");
    Log::warn("test");
    Log::error("test");
    for _ in 0..u32::MAX {}
}
