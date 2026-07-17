use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use xkeysym::{Keysym, key};

pub const TRIGGER_KEY_CHOICES: &[&str] = &[
    "XF86_Fn",
    "Control_R",
    "Control_L",
    "F8",
    "F9",
    "F10",
    "Space",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TriggerMode {
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AsrProviderKind {
    Doubao,
    OpenaiCompatible,
}

impl AsrProviderKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Doubao => "doubao",
            Self::OpenaiCompatible => "openai-compatible",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct AsrConfig {
    pub provider: AsrProviderKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

impl Default for AsrConfig {
    fn default() -> Self {
        Self {
            provider: AsrProviderKind::Doubao,
            endpoint: None,
            api_key: None,
            model: None,
            language: None,
            prompt: None,
        }
    }
}

impl AsrConfig {
    pub const DEFAULT_OPENAI_ENDPOINT: &'static str =
        "https://api.openai.com/v1/audio/transcriptions";
    pub const DEFAULT_OPENAI_MODEL: &'static str = "whisper-1";

    pub fn endpoint(&self) -> &str {
        self.endpoint
            .as_deref()
            .unwrap_or(Self::DEFAULT_OPENAI_ENDPOINT)
    }

    pub fn model(&self) -> &str {
        self.model.as_deref().unwrap_or(Self::DEFAULT_OPENAI_MODEL)
    }

    pub fn validate(&self) -> Result<()> {
        if self.provider == AsrProviderKind::OpenaiCompatible {
            let endpoint =
                reqwest::Url::parse(self.endpoint()).context("asr.endpoint 必须是有效的 URL")?;
            if !matches!(endpoint.scheme(), "http" | "https") {
                bail!("asr.endpoint 只支持 http 或 https URL");
            }
            if self.model().trim().is_empty() {
                bail!("asr.model 不能为空");
            }
        }
        if self
            .api_key
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            bail!("asr.apiKey 不能是空字符串；无密钥服务请删除该字段");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    pub trigger_key: String,
    pub trigger_mode: TriggerMode,
    pub input_device: Option<String>,
    pub max_recording_seconds: u64,
    pub asr: AsrConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trigger_key: "XF86_Fn".to_string(),
            trigger_mode: TriggerMode::Hold,
            input_device: None,
            max_recording_seconds: 600,
            asr: AsrConfig::default(),
        }
    }
}

#[derive(Clone)]
pub struct ConfigStore {
    path: Arc<PathBuf>,
    value: Arc<RwLock<Config>>,
}

impl ConfigStore {
    pub fn load(path: PathBuf) -> Result<Self> {
        let value = Config::load_or_create(&path)?;
        Ok(Self {
            path: Arc::new(path),
            value: Arc::new(RwLock::new(value)),
        })
    }

    pub fn snapshot(&self) -> Config {
        self.read().clone()
    }

    pub fn update(&self, change: impl FnOnce(&mut Config)) -> Result<Config> {
        let mut current = self.write();
        let mut next = current.clone();
        change(&mut next);
        next.save(self.path.as_ref())?;
        *current = next.clone();
        Ok(next)
    }

    pub fn reload(&self) -> Result<Config> {
        let next = Config::load_or_create(self.path.as_ref())?;
        *self.write() = next.clone();
        Ok(next)
    }

    fn read(&self) -> RwLockReadGuard<'_, Config> {
        self.value.read().unwrap_or_else(|error| error.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, Config> {
        self.value
            .write()
            .unwrap_or_else(|error| error.into_inner())
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
        self.asr.validate()?;
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
        assert_eq!(config.max_recording_seconds, 600);
        assert_eq!(config.asr.provider, AsrProviderKind::Doubao);
        assert_eq!(config.asr.endpoint, None);
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

    #[test]
    fn trigger_choices_match_supported_keys() {
        for value in TRIGGER_KEY_CHOICES {
            assert!(parse_trigger_key(value).is_ok());
        }
    }

    #[test]
    fn old_config_defaults_to_zero_configuration_doubao() {
        let config: Config = serde_json::from_str(
            r#"{
                "triggerKey": "XF86_Fn",
                "triggerMode": "hold",
                "inputDevice": null,
                "maxRecordingSeconds": 600
            }"#,
        )
        .unwrap();

        assert_eq!(config.asr, AsrConfig::default());
        assert_eq!(config.asr.provider.as_str(), "doubao");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validates_openai_compatible_configuration() {
        let mut config = Config::default();
        config.asr.provider = AsrProviderKind::OpenaiCompatible;
        assert!(config.validate().is_ok());
        assert_eq!(config.asr.endpoint(), AsrConfig::DEFAULT_OPENAI_ENDPOINT);
        assert_eq!(config.asr.model(), AsrConfig::DEFAULT_OPENAI_MODEL);

        config.asr.endpoint = Some("file:///tmp/asr".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn serializes_default_asr_without_vendor_secrets() {
        let value = serde_json::to_value(Config::default()).unwrap();
        assert_eq!(value["asr"]["provider"], "doubao");
        assert!(value["asr"].get("apiKey").is_none());
        assert!(value["asr"].get("endpoint").is_none());
    }
}
