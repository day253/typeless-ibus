use anyhow::{Context, Result, bail};
use reqwest::header::HeaderMap;
use std::time::Duration;
use tokio::sync::mpsc;
use typeless_ibus::config::AsrConfig;

pub(crate) const SAMPLE_RATE: u32 = 16_000;
pub(crate) const PCM_BYTES_PER_SECOND: usize = 32_000;

pub(crate) async fn collect_pcm(mut audio_rx: mpsc::Receiver<Vec<u8>>) -> Result<Vec<u8>> {
    let mut pcm = Vec::new();
    while let Some(chunk) = audio_rx.recv().await {
        pcm.extend_from_slice(&chunk);
    }
    validate_pcm(&pcm)?;
    Ok(pcm)
}

pub(crate) fn validate_pcm(pcm: &[u8]) -> Result<()> {
    if pcm.is_empty() || !pcm.len().is_multiple_of(2) {
        bail!("ASR 音频必须是非空的 16 kHz 单声道 16-bit little-endian PCM");
    }
    Ok(())
}

pub(crate) fn encode_wav(pcm: &[u8]) -> Result<Vec<u8>> {
    validate_pcm(pcm)?;
    let data_len = u32::try_from(pcm.len()).context("ASR 音频太长，无法编码为 WAV")?;
    let riff_len = data_len
        .checked_add(36)
        .context("ASR 音频太长，无法编码为 WAV")?;
    let mut wav = Vec::with_capacity(pcm.len() + 44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    Ok(wav)
}

pub(crate) fn split_pcm(pcm: &[u8], max_duration_ms: u64) -> Vec<&[u8]> {
    let max_bytes = ((PCM_BYTES_PER_SECOND as u64 * max_duration_ms) / 1_000) as usize;
    let aligned = max_bytes.max(2) & !1;
    pcm.chunks(aligned).collect()
}

pub(crate) fn join_transcripts(texts: &[String]) -> String {
    let mut result = String::new();
    for text in texts.iter().map(|text| text.trim()) {
        if text.is_empty() {
            continue;
        }
        if needs_separator(&result, text) {
            result.push(' ');
        }
        result.push_str(text);
    }
    result
}

fn needs_separator(current: &str, next: &str) -> bool {
    let (Some(previous), Some(next)) = (current.chars().last(), next.chars().next()) else {
        return false;
    };
    if is_closing_punctuation(next) || is_opening_punctuation(previous) {
        return false;
    }
    if is_cjk(previous) && (is_cjk(next) || is_opening_punctuation(next)) {
        return false;
    }
    if is_cjk(next) && (is_closing_punctuation(previous) || is_cjk_punctuation(previous)) {
        return false;
    }
    true
}

fn is_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0x3040..=0x30ff | 0xac00..=0xd7af
    )
}

fn is_cjk_punctuation(character: char) -> bool {
    matches!(
        character,
        '。' | '，' | '、' | '！' | '？' | '；' | '：' | '」' | '』' | '）' | '》'
    )
}

fn is_closing_punctuation(character: char) -> bool {
    (character.is_ascii_punctuation() && !matches!(character, '(' | '[' | '{' | '"' | '\''))
        || is_cjk_punctuation(character)
}

fn is_opening_punctuation(character: char) -> bool {
    matches!(
        character,
        '(' | '[' | '{' | '"' | '\'' | '「' | '『' | '（' | '《'
    )
}

pub(crate) fn http_client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("typeless-ibus/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("创建 ASR 网络客户端失败")
}

pub(crate) fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    [
        "x-request-id",
        "request-id",
        "x-trace-id",
        "x-tt-logid",
        "x-api-request-id",
    ]
    .into_iter()
    .find_map(|name| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| non_empty(Some(value)))
            .map(str::to_owned)
    })
}

pub(crate) fn redacted_endpoint(endpoint: &str) -> String {
    match reqwest::Url::parse(endpoint) {
        Ok(url) => format!("{}{}", url.origin().ascii_serialization(), url.path()),
        Err(_) => "invalid endpoint".to_string(),
    }
}

pub(crate) fn print_diagnosis(config: &AsrConfig, authentication: &str) -> Result<()> {
    config.validate()?;
    println!("asr.provider: {}", config.provider.as_str());
    println!("asr.endpoint: {}", redacted_endpoint(config.endpoint()));
    if !config.model().is_empty() {
        println!("asr.model: {}", config.model());
    }
    println!("asr.authentication: {authentication}");
    println!("asr.diagnosis: configuration is valid; use --check-asr-audio to test recognition");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_pcm_as_standard_wav() {
        let pcm = [1_u8, 2, 3, 4];
        let wav = encode_wav(&pcm).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 16_000);
        assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 4);
        assert_eq!(&wav[44..], pcm.as_slice());
    }

    #[test]
    fn splits_audio_on_even_sample_boundaries() {
        let pcm = vec![0_u8; PCM_BYTES_PER_SECOND * 2 + 2];
        let chunks = split_pcm(&pcm, 1_000);
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|chunk| chunk.len().is_multiple_of(2)));
    }

    #[test]
    fn joins_english_with_spaces_and_cjk_without_them() {
        assert_eq!(
            join_transcripts(&["hello".into(), "world".into()]),
            "hello world"
        );
        assert_eq!(
            join_transcripts(&["你好".into(), "世界".into()]),
            "你好世界"
        );
        assert_eq!(
            join_transcripts(&["hello,".into(), "world".into()]),
            "hello, world"
        );
        assert_eq!(
            join_transcripts(&["你好，".into(), "世界".into()]),
            "你好，世界"
        );
    }
}
