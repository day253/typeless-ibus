use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}

pub fn current_language() -> Language {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
        .map_or(Language::English, |locale| language_for_locale(&locale))
}

pub fn text(english: &'static str, chinese: &'static str) -> &'static str {
    match current_language() {
        Language::English => english,
        Language::Chinese => chinese,
    }
}

fn language_for_locale(locale: &str) -> Language {
    let locale = locale.trim().to_ascii_lowercase();
    if locale == "zh" || locale.starts_with("zh_") || locale.starts_with("zh-") {
        Language::Chinese
    } else {
        Language::English
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_selects_english_and_chinese() {
        assert_eq!(language_for_locale("en_US.UTF-8"), Language::English);
        assert_eq!(language_for_locale("C.UTF-8"), Language::English);
        assert_eq!(language_for_locale("zh_CN.UTF-8"), Language::Chinese);
        assert_eq!(language_for_locale("zh-TW"), Language::Chinese);
    }
}
