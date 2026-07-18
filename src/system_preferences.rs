use std::env;
use std::fs;
use std::path::Path;

const CHINA_TIME_ZONES: &[&str] = &[
    "Asia/Shanghai",
    "Asia/Chongqing",
    "Asia/Harbin",
    "Asia/Urumqi",
    "Asia/Hong_Kong",
    "Asia/Macau",
    "Asia/Taipei",
    "PRC",
    "ROC",
    "Hongkong",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPreferences {
    locale: Option<String>,
    time_zone: Option<String>,
    speech_language: String,
    utc_offset_seconds: i32,
}

impl SystemPreferences {
    pub fn current() -> Self {
        let locale = preferred_locale_from_environment();
        let time_zone = current_time_zone();
        Self::from_parts(
            locale.as_deref(),
            time_zone.as_deref(),
            current_utc_offset_seconds(),
        )
    }

    pub fn from_parts(
        locale: Option<&str>,
        time_zone: Option<&str>,
        utc_offset_seconds: i32,
    ) -> Self {
        let locale = locale.map(str::trim).filter(|value| !value.is_empty());
        let time_zone = time_zone.map(str::trim).filter(|value| !value.is_empty());
        Self {
            locale: locale.map(str::to_owned),
            time_zone: time_zone.map(str::to_owned),
            speech_language: infer_speech_language(locale, time_zone),
            utc_offset_seconds,
        }
    }

    pub fn locale(&self) -> Option<&str> {
        self.locale.as_deref()
    }

    pub fn time_zone(&self) -> Option<&str> {
        self.time_zone.as_deref()
    }

    pub fn speech_language(&self) -> &str {
        &self.speech_language
    }

    pub fn utc_offset_seconds(&self) -> i32 {
        self.utc_offset_seconds
    }
}

fn preferred_locale_from_environment() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES"]
        .into_iter()
        .find_map(non_empty_environment)
        .or_else(|| {
            non_empty_environment("LANGUAGE").and_then(|languages| {
                languages
                    .split(':')
                    .find(|value| !value.trim().is_empty())
                    .map(str::to_owned)
            })
        })
        .or_else(|| non_empty_environment("LANG"))
}

fn non_empty_environment(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn infer_speech_language(locale: Option<&str>, time_zone: Option<&str>) -> String {
    let language = locale.and_then(normalize_language_code);
    if matches!(language.as_deref(), None | Some("en")) && time_zone.is_some_and(is_china_time_zone)
    {
        "zh".to_string()
    } else {
        language.unwrap_or_else(|| "en".to_string())
    }
}

fn normalize_language_code(locale: &str) -> Option<String> {
    let locale = locale.trim();
    if locale.eq_ignore_ascii_case("c")
        || locale.to_ascii_lowercase().starts_with("c.")
        || locale.eq_ignore_ascii_case("posix")
    {
        return Some("en".to_string());
    }
    let language = locale
        .split(['_', '-', '.', '@'])
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    ((2..=3).contains(&language.len()) && language.bytes().all(|value| value.is_ascii_alphabetic()))
        .then_some(language)
}

fn is_china_time_zone(time_zone: &str) -> bool {
    let Some(time_zone) = normalize_time_zone(time_zone) else {
        return false;
    };
    CHINA_TIME_ZONES
        .iter()
        .any(|candidate| time_zone.eq_ignore_ascii_case(candidate))
}

fn current_time_zone() -> Option<String> {
    non_empty_environment("TZ")
        .and_then(|value| normalize_time_zone(&value))
        .or_else(|| {
            fs::read_to_string("/etc/timezone")
                .ok()
                .and_then(|value| normalize_time_zone(&value))
        })
        .or_else(|| time_zone_from_localtime(Path::new("/etc/localtime")))
}

fn time_zone_from_localtime(path: &Path) -> Option<String> {
    let canonical = fs::canonicalize(path).ok()?;
    normalize_time_zone(canonical.to_str()?)
}

fn normalize_time_zone(value: &str) -> Option<String> {
    let value = value.trim().trim_start_matches(':');
    let value = value
        .split_once("/zoneinfo/")
        .map_or(value, |(_, time_zone)| time_zone);
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(unix)]
fn current_utc_offset_seconds() -> i32 {
    let timestamp = unsafe { libc::time(std::ptr::null_mut()) };
    let mut local = unsafe { std::mem::zeroed::<libc::tm>() };
    if timestamp == -1 || unsafe { libc::localtime_r(&timestamp, &mut local) }.is_null() {
        return 0;
    }
    local.tm_gmtoff.clamp(i32::MIN as _, i32::MAX as _) as i32
}

#[cfg(not(unix))]
fn current_utc_offset_seconds() -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_default_in_china_uses_chinese_speech() {
        assert_eq!(
            infer_speech_language(Some("en_US.UTF-8"), Some("Asia/Shanghai")),
            "zh"
        );
        assert_eq!(
            infer_speech_language(Some("C.UTF-8"), Some("Asia/Chongqing")),
            "zh"
        );
    }

    #[test]
    fn explicit_non_english_locale_wins_over_time_zone() {
        assert_eq!(
            infer_speech_language(Some("ja_JP.UTF-8"), Some("Asia/Shanghai")),
            "ja"
        );
        assert_eq!(
            infer_speech_language(Some("zh_TW.UTF-8"), Some("Europe/London")),
            "zh"
        );
    }

    #[test]
    fn english_outside_china_stays_english() {
        assert_eq!(
            infer_speech_language(Some("en_GB.UTF-8"), Some("Europe/London")),
            "en"
        );
    }

    #[test]
    fn normalizes_locale_and_zoneinfo_paths() {
        assert_eq!(normalize_language_code("fil_PH.UTF-8"), Some("fil".into()));
        assert_eq!(normalize_language_code("POSIX"), Some("en".into()));
        assert_eq!(
            normalize_time_zone("/usr/share/zoneinfo/Asia/Shanghai"),
            Some("Asia/Shanghai".into())
        );
    }
}
