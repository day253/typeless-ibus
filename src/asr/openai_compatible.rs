use super::AsrEvent;
use super::provider::{AsrProvider, DiagnosticFuture, EventHandler, RecognitionFuture};
use anyhow::{Context, Result, bail};
use reqwest::header::HeaderMap;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc;
use typeless_ibus::config::{AsrConfig, AsrProviderKind};

const SAMPLE_RATE: u32 = 16_000;

pub(crate) struct OpenaiCompatibleProvider {
    config: AsrConfig,
    client: reqwest::Client,
}

impl OpenaiCompatibleProvider {
    pub(crate) fn new(config: AsrConfig) -> Result<Self> {
        config.validate()?;
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(600))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("typeless-ibus/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("创建 OpenAI-compatible ASR 网络客户端失败")?;
        Ok(Self { config, client })
    }

    async fn transcribe_stream(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        mut on_event: EventHandler,
    ) -> Result<String> {
        let mut pcm = Vec::new();
        while let Some(chunk) = audio_rx.recv().await {
            pcm.extend_from_slice(&chunk);
        }
        let text = self.transcribe_pcm(&pcm).await?;
        on_event(AsrEvent::Final(text.clone()));
        Ok(text)
    }

    async fn transcribe_pcm(&self, pcm: &[u8]) -> Result<String> {
        let wav = encode_wav(pcm)?;
        let file = Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .context("创建 ASR 音频表单失败")?;
        let mut form = Form::new()
            .part("file", file)
            .text("model", self.config.model().to_string());
        if let Some(language) = non_empty(self.config.language.as_deref()) {
            form = form.text("language", language.to_string());
        }
        if let Some(prompt) = non_empty(self.config.prompt.as_deref()) {
            form = form.text("prompt", prompt.to_string());
        }

        let mut request = self.client.post(self.config.endpoint()).multipart(form);
        if let Some(api_key) = non_empty(self.config.api_key.as_deref()) {
            request = request.bearer_auth(api_key);
        }
        let response = request
            .send()
            .await
            .context("请求 OpenAI-compatible ASR 失败")?;
        let status = response.status();
        let request_id = extract_request_id(response.headers());
        if !status.is_success() {
            tracing::error!(
                provider = "openai-compatible",
                request_id = request_id.as_deref().unwrap_or("missing"),
                http_status = %status,
                "ASR request failed"
            );
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "response body unavailable".to_string());
            let message = parse_error_message(&body)
                .unwrap_or_else(|| "upstream response omitted details".to_string());
            bail!("OpenAI-compatible ASR 返回 HTTP {status}：{message}");
        }

        let payload: TranscriptionResponse = match response.json().await {
            Ok(payload) => payload,
            Err(error) => {
                tracing::error!(
                    provider = "openai-compatible",
                    request_id = request_id.as_deref().unwrap_or("missing"),
                    error = %error,
                    "failed to parse ASR response"
                );
                return Err(error).context("解析 OpenAI-compatible ASR 响应失败");
            }
        };
        let text = payload.text.trim().to_string();
        if text.is_empty() {
            tracing::error!(
                provider = "openai-compatible",
                request_id = request_id.as_deref().unwrap_or("missing"),
                "ASR response did not contain text"
            );
            bail!("OpenAI-compatible ASR 完成请求但没有返回识别文字");
        }
        tracing::info!(
            provider = "openai-compatible",
            request_id = request_id.as_deref().unwrap_or("missing"),
            "ASR request completed"
        );
        Ok(text)
    }
}

impl AsrProvider for OpenaiCompatibleProvider {
    fn kind(&self) -> AsrProviderKind {
        AsrProviderKind::OpenaiCompatible
    }

    fn transcribe<'a>(
        &'a self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        on_event: EventHandler,
    ) -> RecognitionFuture<'a> {
        Box::pin(self.transcribe_stream(audio_rx, on_event))
    }

    fn diagnose<'a>(&'a self) -> DiagnosticFuture<'a> {
        Box::pin(async move {
            self.config.validate()?;
            println!("asr.provider: openai-compatible");
            println!(
                "asr.endpoint: {}",
                redacted_endpoint(self.config.endpoint())
            );
            println!("asr.model: {}", self.config.model());
            println!(
                "asr.authentication: {}",
                if non_empty(self.config.api_key.as_deref()).is_some() {
                    "configured (secret redacted)"
                } else {
                    "not configured"
                }
            );
            println!(
                "asr.diagnosis: configuration is valid; use --check-asr-audio to test recognition"
            );
            Ok(())
        })
    }
}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: Option<ErrorBody>,
    detail: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: Option<String>,
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn parse_error_message(body: &str) -> Option<String> {
    let envelope = serde_json::from_str::<ErrorEnvelope>(body).ok()?;
    envelope
        .error
        .and_then(|error| error.message)
        .or(envelope.detail)
        .or(envelope.message)
        .map(|message| message.trim().to_string())
        .filter(|message| !message.is_empty())
}

fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    ["x-request-id", "request-id", "x-trace-id"]
        .into_iter()
        .find_map(|name| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| non_empty(Some(value)))
                .map(str::to_owned)
        })
}

fn redacted_endpoint(endpoint: &str) -> String {
    match reqwest::Url::parse(endpoint) {
        Ok(url) => format!("{}{}", url.origin().ascii_serialization(), url.path()),
        Err(_) => "invalid endpoint".to_string(),
    }
}

fn encode_wav(pcm: &[u8]) -> Result<Vec<u8>> {
    if pcm.is_empty() || !pcm.len().is_multiple_of(2) {
        bail!("ASR 音频必须是非空的 16 kHz 单声道 16-bit little-endian PCM");
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

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
    fn extracts_common_request_identifiers() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Request-Id", "request-123".parse().unwrap());
        assert_eq!(extract_request_id(&headers).as_deref(), Some("request-123"));
    }

    #[test]
    fn reads_structured_upstream_errors() {
        assert_eq!(
            parse_error_message(r#"{"error":{"message":"invalid model"}}"#),
            Some("invalid model".to_string())
        );
        assert_eq!(parse_error_message("not json"), None);
    }

    #[test]
    fn redacts_endpoint_credentials_and_query_parameters() {
        assert_eq!(
            redacted_endpoint("https://user:pass@example.com/v1/audio/transcriptions?key=secret"),
            "https://example.com/v1/audio/transcriptions"
        );
    }

    #[tokio::test]
    async fn sends_openai_compatible_multipart_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let request = read_http_request(&mut stream);
            let request_text = String::from_utf8_lossy(&request);
            assert!(request_text.starts_with("POST /transcribe HTTP/1.1"));
            assert!(request_text.contains("authorization: Bearer test-key"));
            assert!(request_text.contains("name=\"model\""));
            assert!(request_text.contains("whisper-test"));
            assert!(request.windows(4).any(|window| window == b"RIFF"));

            let body = r#"{"text":"hello from mock ASR"}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Request-Id: mock-request-42\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let provider = OpenaiCompatibleProvider::new(AsrConfig {
            provider: AsrProviderKind::OpenaiCompatible,
            endpoint: Some(format!("http://{address}/transcribe")),
            api_key: Some("test-key".to_string()),
            model: Some("whisper-test".to_string()),
            language: Some("en".to_string()),
            prompt: None,
        })
        .unwrap();
        let text = provider.transcribe_pcm(&[0_u8; 640]).await.unwrap();
        assert_eq!(text, "hello from mock ASR");
        server.join().unwrap();
    }

    fn read_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected_len = None;
        loop {
            let count = stream.read(&mut buffer).unwrap();
            assert!(
                count > 0,
                "client closed before sending the complete request"
            );
            request.extend_from_slice(&buffer[..count]);
            if expected_len.is_none()
                && let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
            {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|value| value.trim().parse::<usize>().ok())
                    })
                    .expect("multipart request should have a content length");
                expected_len = Some(header_end + 4 + content_length);
            }
            if expected_len.is_some_and(|length| request.len() >= length) {
                return request;
            }
        }
    }
}
