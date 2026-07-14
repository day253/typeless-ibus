use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use xkeysym::{Keysym, key};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TriggerMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    pub trigger_key: String,
    pub trigger_mode: TriggerMode,
    pub input_device: Option<String>,
    pub max_recording_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trigger_key: "XF86_Fn".to_string(),
            trigger_mode: TriggerMode::Hold,
            input_device: None,
            max_recording_seconds: 120,
        }
    }
}

impl Config {
    pub fn load_or_create(path: &Path) -> Result<Self> {
        let config = match fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content)
                .with_context(|| format!("解析配置文件失败：{}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                config.save(path)?;
                config
            }
            Err(error) => {
                return Err(error).with_context(|| format!("读取配置文件失败：{}", path.display()));
            }
        };
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("创建配置目录失败：{}", parent.display()))?;
        }
        let content = format!("{}\n", serde_json::to_string_pretty(self)?);
        fs::write(path, content)
            .with_context(|| format!("写入配置文件失败：{}", path.display()))?;

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("设置配置权限失败：{}", path.display()))?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        parse_trigger_key(&self.trigger_key)?;
        if !(1..=600).contains(&self.max_recording_seconds) {
            bail!("maxRecordingSeconds 必须在 1 到 600 之间");
        }
        Ok(())
    }

    pub fn trigger_keysym(&self) -> Result<Keysym> {
        parse_trigger_key(&self.trigger_key)
    }
}

pub fn config_path() -> Result<PathBuf> {
    let base = match env::var_os("XDG_CONFIG_HOME") {
        Some(path) => PathBuf::from(path),
        None => home_dir()?.join(".config"),
    };
    Ok(base.join("typeless-ibus/config.json"))
}

pub fn credentials_path() -> Result<PathBuf> {
    let base = match env::var_os("XDG_DATA_HOME") {
        Some(path) => PathBuf::from(path),
        None => home_dir()?.join(".local/share"),
    };
    Ok(base.join("typeless-ibus/credentials.json"))
}

fn home_dir() -> Result<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME 环境变量不存在")
}

fn parse_trigger_key(value: &str) -> Result<Keysym> {
    let raw = match value.trim() {
        "Fn" | "XF86_Fn" | "XF86XK_Fn" => key::XF86_Fn,
        "Control_L" | "Ctrl_L" | "LeftCtrl" => key::Control_L,
        "Control_R" | "Ctrl_R" | "RightCtrl" => key::Control_R,
        "Alt_L" | "LeftAlt" => key::Alt_L,
        "Alt_R" | "RightAlt" => key::Alt_R,
        "Shift_L" | "LeftShift" => key::Shift_L,
        "Shift_R" | "RightShift" => key::Shift_R,
        "F8" => key::F8,
        "F9" => key::F9,
        "F10" => key::F10,
        "space" | "Space" => key::space,
        value if value.starts_with("0x") => u32::from_str_radix(&value[2..], 16)
            .with_context(|| format!("无法解析十六进制按键：{value}"))?,
        value => bail!(
            "不支持的 triggerKey：{value}；可用 XF86_Fn、Control_R、Control_L、F8、F9、F10、Space 或 0x十六进制键值"
        ),
    };
    Ok(Keysym::new(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_hold_fn() {
        let config = Config::default();
        assert_eq!(config.trigger_mode, TriggerMode::Hold);
        assert_eq!(config.trigger_keysym().unwrap(), Keysym::XF86_Fn);
    }

    #[test]
    fn parses_configurable_keys() {
        assert_eq!(parse_trigger_key("RightCtrl").unwrap(), Keysym::Control_R);
        assert_eq!(parse_trigger_key("F8").unwrap(), Keysym::F8);
        assert_eq!(parse_trigger_key("0xffc5").unwrap(), Keysym::F8);
    }

    #[test]
    fn rejects_unbounded_recording_time() {
        let config = Config {
            max_recording_seconds: 0,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }
}
