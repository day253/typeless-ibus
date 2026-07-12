use crate::config::AppConfig;
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputOutcome {
    pub inserted: bool,
}

pub fn deliver(text: &str, config: &AppConfig) -> Result<OutputOutcome> {
    let mut clipboard = arboard::Clipboard::new().context("无法访问剪贴板")?;
    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(text.to_string())
        .context("无法写入剪贴板")?;

    if !config.auto_paste || is_wayland() {
        return Ok(OutputOutcome { inserted: false });
    }

    std::thread::sleep(Duration::from_millis(40));
    simulate_paste()?;

    if config.restore_clipboard {
        let inserted = text.to_string();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(900));
            let Ok(mut clipboard) = arboard::Clipboard::new() else {
                return;
            };
            if clipboard.get_text().ok().as_deref() == Some(inserted.as_str()) {
                if let Some(previous) = previous {
                    let _ = clipboard.set_text(previous);
                }
            }
        });
    }

    Ok(OutputOutcome { inserted: true })
}

pub fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[cfg(target_os = "macos")]
fn simulate_paste() -> Result<()> {
    let status = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .status()
        .context("启动 macOS 粘贴命令失败")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("macOS 辅助功能未允许自动粘贴"))
    }
}

#[cfg(target_os = "linux")]
fn simulate_paste() -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default()).context("创建键盘模拟器失败")?;
    enigo
        .key(Key::Control, Direction::Press)
        .context("按下 Ctrl 失败")?;
    let paste_result = enigo.key(Key::Unicode('v'), Direction::Click);
    let release_result = enigo.key(Key::Control, Direction::Release);
    paste_result.context("发送粘贴按键失败")?;
    release_result.context("释放 Ctrl 失败")?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn simulate_paste() -> Result<()> {
    Err(anyhow!("当前平台不支持自动粘贴"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_wayland_session_is_detected() {
        let previous = std::env::var("XDG_SESSION_TYPE").ok();
        std::env::set_var("XDG_SESSION_TYPE", "wayland");
        assert!(is_wayland());
        match previous {
            Some(value) => std::env::set_var("XDG_SESSION_TYPE", value),
            None => std::env::remove_var("XDG_SESSION_TYPE"),
        }
    }
}
