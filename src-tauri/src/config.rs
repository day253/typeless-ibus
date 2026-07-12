use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub hotkey: String,
    pub auto_paste: bool,
    pub restore_clipboard: bool,
    pub input_device: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+Space".to_string(),
            auto_paste: true,
            restore_clipboard: true,
            input_device: None,
        }
    }
}

pub fn load(path: &Path) -> AppConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, config: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("创建配置目录失败")?;
    }
    let json = serde_json::to_string_pretty(config).context("序列化配置失败")?;
    fs::write(path, format!("{json}\n")).context("写入配置失败")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .context("设置配置文件权限失败")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_frontend_contract() {
        let config = AppConfig::default();
        assert_eq!(config.hotkey, "Ctrl+Shift+Space");
        assert!(config.auto_paste);
        assert!(config.restore_clipboard);
        assert_eq!(config.input_device, None);
    }
}
