use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::mem::MaybeUninit;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::prelude::*;

pub const LATEST_LOG_NAME: &str = "typeless-ibus.latest.jsonl";
const LOG_FILE_PREFIX: &str = "typeless-ibus";
const LOG_FILE_SUFFIX: &str = "jsonl";
const MAX_LOG_FILES: usize = 7;

pub struct LoggingGuard;

#[derive(Clone)]
struct DailyLogWriter {
    state: Arc<Mutex<DailyLogState>>,
}

struct DailyLogState {
    directory: PathBuf,
    date: String,
    file: File,
}

pub fn init() -> Result<LoggingGuard> {
    // This process only creates private configuration, credential, and log files.
    // Keeping a restrictive umask also protects files opened later by daily rotation.
    unsafe {
        libc::umask(0o077);
    }
    let directory = log_directory()?;
    fs::create_dir_all(&directory)
        .with_context(|| format!("创建日志目录失败：{}", directory.display()))?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
        .with_context(|| format!("设置日志目录权限失败：{}", directory.display()))?;

    protect_log_files(&directory)?;
    let file_writer = DailyLogWriter::new(directory.clone())
        .with_context(|| format!("初始化日志文件失败：{}", directory.display()))?;
    let stderr_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "typeless_ibus=info,typeless_ibus_engine=info".into());
    let file_filter =
        tracing_subscriber::EnvFilter::new("typeless_ibus=info,typeless_ibus_engine=info");
    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(stderr_filter);
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(false)
        .with_ansi(false)
        .with_writer(file_writer)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
        .context("初始化日志订阅器失败")?;

    tracing::info!(
        schema_version = 1,
        event = "application.logging_initialized",
        log_directory = %directory.display(),
        latest_log = %directory.join(LATEST_LOG_NAME).display(),
        retention_files = MAX_LOG_FILES,
        "local structured logging initialized"
    );
    Ok(LoggingGuard)
}

pub fn log_directory() -> Result<PathBuf> {
    log_directory_from(env::var_os("XDG_STATE_HOME"), env::var_os("HOME"))
}

pub fn latest_log_path() -> Result<PathBuf> {
    Ok(log_directory()?.join(LATEST_LOG_NAME))
}

fn log_directory_from(
    xdg_state_home: Option<impl Into<PathBuf>>,
    home: Option<impl Into<PathBuf>>,
) -> Result<PathBuf> {
    let base = match xdg_state_home {
        Some(path) => path.into(),
        None => home
            .map(Into::into)
            .context("HOME 环境变量不存在")?
            .join(".local/state"),
    };
    Ok(base.join("typeless-ibus/logs"))
}

fn protect_log_files(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("读取日志目录失败：{}", directory.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(LOG_FILE_PREFIX) && name.ends_with(LOG_FILE_SUFFIX) {
            fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

impl DailyLogWriter {
    fn new(directory: PathBuf) -> io::Result<Self> {
        let date = utc_date()?;
        let file = open_log_file(&directory, &date)?;
        update_latest_symlink(&directory, &date)?;
        prune_old_logs(&directory)?;
        Ok(Self {
            state: Arc::new(Mutex::new(DailyLogState {
                directory,
                date,
                file,
            })),
        })
    }
}

impl<'a> MakeWriter<'a> for DailyLogWriter {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Write for DailyLogWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.rotate_if_needed()?;
        state.file.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.file.flush()
    }
}

impl DailyLogState {
    fn rotate_if_needed(&mut self) -> io::Result<()> {
        let date = utc_date()?;
        if date == self.date {
            return Ok(());
        }
        self.file = open_log_file(&self.directory, &date)?;
        self.date = date;
        update_latest_symlink(&self.directory, &self.date)?;
        prune_old_logs(&self.directory)
    }
}

fn log_file_name(date: &str) -> String {
    format!("{LOG_FILE_PREFIX}.{date}.{LOG_FILE_SUFFIX}")
}

fn open_log_file(directory: &Path, date: &str) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(directory.join(log_file_name(date)))
}

fn update_latest_symlink(directory: &Path, date: &str) -> io::Result<()> {
    let latest = directory.join(LATEST_LOG_NAME);
    match fs::symlink_metadata(&latest) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(&latest)?,
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} 不是软链接", latest.display()),
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    symlink(log_file_name(date), latest)
}

fn prune_old_logs(directory: &Path) -> io::Result<()> {
    let mut logs = fs::read_dir(directory)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| is_log_file_name(&entry.file_name().to_string_lossy()))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    logs.sort_unstable();
    let remove_count = logs.len().saturating_sub(MAX_LOG_FILES);
    for path in logs.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn is_log_file_name(name: &str) -> bool {
    let Some(date) = name
        .strip_prefix(&format!("{LOG_FILE_PREFIX}."))
        .and_then(|name| name.strip_suffix(&format!(".{LOG_FILE_SUFFIX}")))
    else {
        return false;
    };
    date.len() == 10
        && date.bytes().enumerate().all(|(index, value)| match index {
            4 | 7 => value == b'-',
            _ => value.is_ascii_digit(),
        })
}

fn utc_date() -> io::Result<String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_secs();
    let seconds = libc::time_t::try_from(seconds)
        .map_err(|_| io::Error::other("系统时间超出 time_t 范围"))?;
    let mut value = MaybeUninit::<libc::tm>::uninit();
    let result = unsafe { libc::gmtime_r(&seconds, value.as_mut_ptr()) };
    if result.is_null() {
        return Err(io::Error::last_os_error());
    }
    let value = unsafe { value.assume_init() };
    Ok(format!(
        "{:04}-{:02}-{:02}",
        value.tm_year + 1900,
        value.tm_mon + 1,
        value.tm_mday
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_xdg_state_home_when_present() {
        assert_eq!(
            log_directory_from(Some("/tmp/state"), Some("/home/test")).unwrap(),
            PathBuf::from("/tmp/state/typeless-ibus/logs")
        );
    }

    #[test]
    fn falls_back_to_home_state_directory() {
        assert_eq!(
            log_directory_from(None::<&str>, Some("/home/test")).unwrap(),
            PathBuf::from("/home/test/.local/state/typeless-ibus/logs")
        );
    }

    #[test]
    fn recognizes_only_daily_jsonl_files_for_retention() {
        assert!(is_log_file_name("typeless-ibus.2026-07-18.jsonl"));
        assert!(!is_log_file_name(LATEST_LOG_NAME));
        assert!(!is_log_file_name("unrelated.2026-07-18.jsonl"));
    }
}
