use crate::system_preferences::SystemPreferences;
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
    Whisper,
    Groq,
    Openrouter,
    Siliconflow,
    Zhipu,
    Elevenlabs,
    XiaomiMimoAsr,
    Bailian,
    BailianQwen3Realtime,
    BailianFunAsrFlash,
    Volcengine,
}

impl AsrProviderKind {
    pub const ALL: [Self; 13] = [
        Self::Doubao,
        Self::OpenaiCompatible,
        Self::Whisper,
        Self::Groq,
        Self::Openrouter,
        Self::Siliconflow,
        Self::Zhipu,
        Self::Elevenlabs,
        Self::XiaomiMimoAsr,
        Self::Bailian,
        Self::BailianQwen3Realtime,
        Self::BailianFunAsrFlash,
        Self::Volcengine,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Doubao => "doubao",
            Self::OpenaiCompatible => "openai-compatible",
            Self::Whisper => "whisper",
            Self::Groq => "groq",
            Self::Openrouter => "openrouter",
            Self::Siliconflow => "siliconflow",
            Self::Zhipu => "zhipu",
            Self::Elevenlabs => "elevenlabs",
            Self::XiaomiMimoAsr => "xiaomi-mimo-asr",
            Self::Bailian => "bailian",
            Self::BailianQwen3Realtime => "bailian-qwen3-realtime",
            Self::BailianFunAsrFlash => "bailian-fun-asr-flash",
            Self::Volcengine => "volcengine",
        }
    }

    pub const fn default_endpoint(self) -> &'static str {
        match self {
            Self::Doubao => "",
            Self::OpenaiCompatible => "https://api.openai.com/v1/audio/transcriptions",
            Self::Whisper => "https://api.openai.com/v1/audio/transcriptions",
            Self::Groq => "https://api.groq.com/openai/v1/audio/transcriptions",
            Self::Openrouter => "https://openrouter.ai/api/v1/audio/transcriptions",
            Self::Siliconflow => "https://api.siliconflow.cn/v1/audio/transcriptions",
            Self::Zhipu => "https://open.bigmodel.cn/api/paas/v4/audio/transcriptions",
            Self::Elevenlabs => "https://api.elevenlabs.io/v1/speech-to-text",
            Self::XiaomiMimoAsr => "https://api.xiaomimimo.com/v1/chat/completions",
            Self::Bailian => "wss://dashscope.aliyuncs.com/api-ws/v1/inference/",
            Self::BailianQwen3Realtime => "wss://dashscope.aliyuncs.com/api-ws/v1/realtime",
            Self::BailianFunAsrFlash => {
                "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation"
            }
            Self::Volcengine => "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async",
        }
    }

    pub const fn default_model(self) -> &'static str {
        match self {
            Self::Doubao | Self::Volcengine => "",
            Self::OpenaiCompatible | Self::Whisper => "whisper-1",
            Self::Groq => "whisper-large-v3-turbo",
            Self::Openrouter => "openai/whisper-large-v3-turbo",
            Self::Siliconflow => "FunAudioLLM/SenseVoiceSmall",
            Self::Zhipu => "glm-asr-2512",
            Self::Elevenlabs => "scribe_v2",
            Self::XiaomiMimoAsr => "mimo-v2.5-asr",
            Self::Bailian => "fun-asr-realtime",
            Self::BailianQwen3Realtime => "qwen3-asr-flash-realtime",
            Self::BailianFunAsrFlash => "fun-asr-flash-2026-06-15",
        }
    }

    pub const fn accepts_language_configuration(self) -> bool {
        matches!(
            self,
            Self::OpenaiCompatible
                | Self::Whisper
                | Self::Groq
                | Self::Siliconflow
                | Self::Zhipu
                | Self::Elevenlabs
                | Self::BailianQwen3Realtime
        )
    }

    pub const fn uses_system_language_hint(self) -> bool {
        matches!(
            self,
            Self::Whisper | Self::Groq | Self::Elevenlabs | Self::BailianQwen3Realtime
        )
    }

    pub const fn default_prompt(self) -> Option<&'static str> {
        None
    }

    pub const fn default_resource_id(self) -> Option<&'static str> {
        match self {
            Self::Volcengine => Some("volc.seedasr.sauc.duration"),
            _ => None,
        }
    }

    pub const fn is_websocket(self) -> bool {
        matches!(
            self,
            Self::Bailian | Self::BailianQwen3Realtime | Self::Volcengine
        )
    }

    pub const fn requires_api_key(self) -> bool {
        matches!(
            self,
            Self::Whisper
                | Self::Groq
                | Self::Openrouter
                | Self::Siliconflow
                | Self::Zhipu
                | Self::Elevenlabs
                | Self::XiaomiMimoAsr
                | Self::Bailian
                | Self::BailianQwen3Realtime
                | Self::BailianFunAsrFlash
                | Self::Volcengine
        )
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|provider| provider.as_str() == value)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
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
            resource_id: None,
        }
    }
}

impl AsrConfig {
    pub fn endpoint(&self) -> &str {
        self.endpoint
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.provider.default_endpoint())
    }

    pub fn model(&self) -> &str {
        self.model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.provider.default_model())
    }

    pub fn api_key(&self) -> Option<&str> {
        non_empty(self.api_key.as_deref())
    }

    pub fn configured_language(&self) -> Option<&str> {
        non_empty(self.language.as_deref())
    }

    pub fn language(&self) -> Option<String> {
        self.language_with_preferences(&SystemPreferences::current())
    }

    pub fn prompt(&self) -> Option<&str> {
        non_empty(self.prompt.as_deref()).or_else(|| self.provider.default_prompt())
    }

    pub fn resource_id(&self) -> &str {
        non_empty(self.resource_id.as_deref())
            .or_else(|| self.provider.default_resource_id())
            .unwrap_or("")
    }

    pub fn validate(&self) -> Result<()> {
        if self.configured_language().is_some() && !self.provider.accepts_language_configuration() {
            bail!(
                "{} 不支持 asr.language；请删除该字段并使用服务端自动识别",
                self.provider.as_str()
            );
        }
        if self.provider != AsrProviderKind::Doubao {
            let endpoint =
                reqwest::Url::parse(self.endpoint()).context("asr.endpoint 必须是有效的 URL")?;
            if self.provider.is_websocket() {
                if !matches!(endpoint.scheme(), "ws" | "wss") {
                    bail!(
                        "{} 的 asr.endpoint 必须使用 ws 或 wss",
                        self.provider.as_str()
                    );
                }
            } else if !matches!(endpoint.scheme(), "http" | "https") {
                bail!(
                    "{} 的 asr.endpoint 必须使用 http 或 https",
                    self.provider.as_str()
                );
            }
            if self.provider != AsrProviderKind::Volcengine && self.model().trim().is_empty() {
                bail!("asr.model 不能为空");
            }
        }
        if self.provider.requires_api_key() && self.api_key().is_none() {
            bail!("{} 需要 asr.apiKey", self.provider.as_str());
        }
        Ok(())
    }

    fn language_with_preferences(&self, preferences: &SystemPreferences) -> Option<String> {
        if let Some(language) = self.configured_language() {
            return Some(language.to_string());
        }
        if !self.provider.uses_system_language_hint() {
            return None;
        }
        let language = preferences.speech_language();
        match self.provider {
            AsrProviderKind::Whisper | AsrProviderKind::Groq => {
                (language.len() == 2).then(|| language.to_string())
            }
            AsrProviderKind::Elevenlabs => {
                ((2..=3).contains(&language.len())).then(|| language.to_string())
            }
            AsrProviderKind::BailianQwen3Realtime => QWEN_ASR_LANGUAGES
                .contains(&language)
                .then(|| language.to_string()),
            _ => None,
        }
    }
}

const QWEN_ASR_LANGUAGES: &[&str] = &[
    "zh", "yue", "en", "ja", "de", "ko", "ru", "fr", "pt", "ar", "it", "es", "hi", "id", "th",
    "tr", "uk", "vi", "cs", "da", "fil", "fi", "is", "ms", "no", "pl", "sv",
];

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
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
        assert_eq!(
            config.asr.endpoint(),
            AsrProviderKind::OpenaiCompatible.default_endpoint()
        );
        assert_eq!(
            config.asr.model(),
            AsrProviderKind::OpenaiCompatible.default_model()
        );

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

    #[test]
    fn packaged_example_is_a_valid_default_config() {
        let config: Config =
            serde_json::from_str(include_str!("../data/config.example.json")).unwrap();

        assert_eq!(config.asr.provider, AsrProviderKind::Doubao);
        assert_eq!(config.asr.endpoint, None);
        assert_eq!(config.asr.api_key, None);
        assert_eq!(config.asr.resource_id, None);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rejects_removed_volcengine_app_and_access_keys() {
        let error = serde_json::from_str::<Config>(
            r#"{
                "asr": {
                    "provider": "volcengine",
                    "appKey": "removed",
                    "accessKey": "removed"
                }
            }"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown field `appKey`"));
    }

    #[test]
    fn rejects_removed_vocabulary_id() {
        let error = serde_json::from_str::<Config>(
            r#"{
                "asr": {
                    "provider": "bailian",
                    "apiKey": "key",
                    "vocabularyId": "removed"
                }
            }"#,
        )
        .unwrap_err();

        assert!(error.to_string().contains("unknown field `vocabularyId`"));
    }

    #[test]
    fn every_configured_cloud_provider_has_protocol_defaults() {
        let providers = [
            AsrProviderKind::OpenaiCompatible,
            AsrProviderKind::Whisper,
            AsrProviderKind::Groq,
            AsrProviderKind::Openrouter,
            AsrProviderKind::Siliconflow,
            AsrProviderKind::Zhipu,
            AsrProviderKind::Elevenlabs,
            AsrProviderKind::XiaomiMimoAsr,
            AsrProviderKind::Bailian,
            AsrProviderKind::BailianQwen3Realtime,
            AsrProviderKind::BailianFunAsrFlash,
            AsrProviderKind::Volcengine,
        ];
        for provider in providers {
            assert!(!provider.default_endpoint().is_empty());
            if provider != AsrProviderKind::Volcengine {
                assert!(!provider.default_model().is_empty());
            }
        }
    }

    #[test]
    fn every_provider_accepts_minimal_configuration_and_resolves_defaults() {
        let preferences =
            SystemPreferences::from_parts(Some("en_US.UTF-8"), Some("America/New_York"), -18_000);
        for provider in AsrProviderKind::ALL {
            let config = AsrConfig {
                provider,
                api_key: (provider != AsrProviderKind::Doubao).then(|| "api-key".to_string()),
                ..AsrConfig::default()
            };

            assert!(config.validate().is_ok(), "{}", provider.as_str());
            if provider == AsrProviderKind::Doubao {
                assert_eq!(config.endpoint(), "");
                assert_eq!(config.model(), "");
            } else {
                assert!(!config.endpoint().is_empty(), "{}", provider.as_str());
                if provider != AsrProviderKind::Volcengine {
                    assert!(!config.model().is_empty(), "{}", provider.as_str());
                }
            }
            let expected_language = provider
                .uses_system_language_hint()
                .then(|| "en".to_string());
            assert_eq!(
                config.language_with_preferences(&preferences),
                expected_language,
                "{}",
                provider.as_str()
            );
            assert_eq!(config.prompt(), None, "{}", provider.as_str());
            if provider == AsrProviderKind::Volcengine {
                assert_eq!(config.resource_id(), "volc.seedasr.sauc.duration");
            } else {
                assert_eq!(config.resource_id(), "");
            }
        }
    }

    #[test]
    fn speech_language_prefers_config_then_locale_and_china_time_zone() {
        let china =
            SystemPreferences::from_parts(Some("en_US.UTF-8"), Some("Asia/Shanghai"), 28_800);
        let mut config = AsrConfig {
            provider: AsrProviderKind::Whisper,
            api_key: Some("key".to_string()),
            ..AsrConfig::default()
        };
        assert_eq!(config.language_with_preferences(&china), Some("zh".into()));

        config.language = Some("fr".to_string());
        assert_eq!(config.language_with_preferences(&china), Some("fr".into()));
    }

    #[test]
    fn unsupported_language_hints_fall_back_to_provider_detection() {
        let preferences =
            SystemPreferences::from_parts(Some("nl_NL.UTF-8"), Some("Europe/Amsterdam"), 7_200);
        let qwen = AsrConfig {
            provider: AsrProviderKind::BailianQwen3Realtime,
            api_key: Some("key".to_string()),
            ..AsrConfig::default()
        };
        assert_eq!(qwen.language_with_preferences(&preferences), None);

        let openrouter = AsrConfig {
            provider: AsrProviderKind::Openrouter,
            api_key: Some("key".to_string()),
            language: Some("en".to_string()),
            ..AsrConfig::default()
        };
        assert!(openrouter.validate().is_err());
    }

    #[test]
    fn every_provider_id_round_trips_through_json_and_cli_parsing() {
        for provider in AsrProviderKind::ALL {
            assert_eq!(AsrProviderKind::parse(provider.as_str()), Some(provider));
            assert_eq!(
                serde_json::to_value(provider).unwrap(),
                serde_json::Value::String(provider.as_str().to_string())
            );
            assert_eq!(
                serde_json::from_value::<AsrProviderKind>(serde_json::Value::String(
                    provider.as_str().to_string()
                ))
                .unwrap(),
                provider
            );
        }
    }

    #[test]
    fn credentialed_cloud_providers_require_their_selected_keys() {
        for provider in [
            AsrProviderKind::Whisper,
            AsrProviderKind::Groq,
            AsrProviderKind::Openrouter,
            AsrProviderKind::Siliconflow,
            AsrProviderKind::Zhipu,
            AsrProviderKind::Elevenlabs,
            AsrProviderKind::XiaomiMimoAsr,
            AsrProviderKind::Bailian,
            AsrProviderKind::BailianQwen3Realtime,
            AsrProviderKind::BailianFunAsrFlash,
            AsrProviderKind::Volcengine,
        ] {
            let mut config = AsrConfig {
                provider,
                ..AsrConfig::default()
            };
            assert!(config.validate().is_err(), "{}", provider.as_str());
            config.api_key = Some("secret".to_string());
            assert!(config.validate().is_ok(), "{}", provider.as_str());
        }
    }
}
