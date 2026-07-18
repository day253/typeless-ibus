use super::AsrEvent;
use super::provider::{AsrProvider, DiagnosticFuture, EventHandler, RecognitionFuture};
use super::shared::{
    collect_pcm, encode_wav, extract_request_id, http_client, join_transcripts, print_diagnosis,
    split_pcm,
};
use anyhow::{Context, Result, bail};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc;
use typeless_ibus::config::{AsrConfig, AsrProviderKind};

const SHORT_BATCH_CHUNK_MS: u64 = 30_000;

pub(crate) struct OpenaiCompatibleProvider {
    config: AsrConfig,
    client: reqwest::Client,
}

impl OpenaiCompatibleProvider {
    pub(crate) fn new(config: AsrConfig) -> Result<Self> {
        config.validate()?;
        if !matches!(
            config.provider,
            AsrProviderKind::OpenaiCompatible
                | AsrProviderKind::Whisper
                | AsrProviderKind::Groq
                | AsrProviderKind::Siliconflow
                | AsrProviderKind::Zhipu
        ) {
            bail!("{} 不是 multipart ASR provider", config.provider.as_str());
        }
        let client = http_client(Duration::from_secs(600))?;
        Ok(Self { config, client })
    }

    async fn transcribe_stream(
        &self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        mut on_event: EventHandler,
    ) -> Result<String> {
        let pcm = collect_pcm(audio_rx).await?;
        let text = self.transcribe_pcm(&pcm).await?;
        on_event(AsrEvent::Final(text.clone()));
        Ok(text)
    }

    async fn transcribe_pcm(&self, pcm: &[u8]) -> Result<String> {
        let chunks = match batch_chunk_limit_ms(self.config.provider) {
            Some(limit) => split_pcm(pcm, limit),
            None => vec![pcm],
        };
        let mut texts = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            texts.push(self.transcribe_chunk(chunk).await?);
        }
        Ok(join_transcripts(&texts))
    }

    async fn transcribe_chunk(&self, pcm: &[u8]) -> Result<String> {
        let wav = encode_wav(pcm)?;
        let file = Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .context("创建 ASR 音频表单失败")?;
        let mut form = Form::new()
            .part("file", file)
            .text("model", self.config.model().to_string());
        if let Some(language) = self.config.language() {
            form = form.text("language", language.to_string());
        }
        if let Some(prompt) = self.config.prompt() {
            form = form.text("prompt", prompt.to_string());
        }

        let mut request = self
            .client
            .post(provider_url(&self.config)?)
            .multipart(form);
        if let Some(api_key) = self.config.api_key() {
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
                provider = self.config.provider.as_str(),
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
            bail!(
                "{} ASR 返回 HTTP {status}：{message}",
                self.config.provider.as_str()
            );
        }

        let payload: TranscriptionResponse = match response.json().await {
            Ok(payload) => payload,
            Err(error) => {
                tracing::error!(
                    provider = self.config.provider.as_str(),
                    request_id = request_id.as_deref().unwrap_or("missing"),
                    error = %error,
                    "failed to parse ASR response"
                );
                return Err(error).with_context(|| {
                    format!("解析 {} ASR 响应失败", self.config.provider.as_str())
                });
            }
        };
        let text = payload.text.trim().to_string();
        if text.is_empty() {
            tracing::error!(
                provider = self.config.provider.as_str(),
                request_id = request_id.as_deref().unwrap_or("missing"),
                "ASR response did not contain text"
            );
            bail!(
                "{} ASR 完成请求但没有返回识别文字",
                self.config.provider.as_str()
            );
        }
        tracing::info!(
            provider = self.config.provider.as_str(),
            request_id = request_id.as_deref().unwrap_or("missing"),
            "ASR request completed"
        );
        Ok(text)
    }
}

impl AsrProvider for OpenaiCompatibleProvider {
    fn kind(&self) -> AsrProviderKind {
        self.config.provider
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
            let authentication = if self.config.api_key().is_some() {
                "configured (secret redacted)"
            } else {
                "not configured"
            };
            print_diagnosis(&self.config, authentication)
        })
    }
}

fn batch_chunk_limit_ms(provider: AsrProviderKind) -> Option<u64> {
    (provider == AsrProviderKind::Zhipu).then_some(SHORT_BATCH_CHUNK_MS)
}

fn provider_url(config: &AsrConfig) -> Result<String> {
    if config.provider == AsrProviderKind::OpenaiCompatible {
        return Ok(config.endpoint().to_string());
    }
    transcription_url(config.endpoint())
}

fn transcription_url(endpoint: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(endpoint).context("解析转写 endpoint 失败")?;
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with("/audio/transcriptions") {
        path.to_string()
    } else if path.ends_with("/audio") {
        format!("{path}/transcriptions")
    } else if let Some(prefix) = path.strip_suffix("/chat/completions") {
        format!("{prefix}/audio/transcriptions")
    } else {
        format!("{path}/audio/transcriptions")
    };
    url.set_path(&path);
    Ok(url.to_string())
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

#[cfg(test)]
mod tests {
    use super::super::shared::redacted_endpoint;
    use super::*;
    use reqwest::header::HeaderMap;
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

    #[test]
    fn normalizes_transcription_endpoints_and_provider_limits() {
        assert_eq!(
            transcription_url("https://api.openai.com/v1").unwrap(),
            "https://api.openai.com/v1/audio/transcriptions"
        );
        assert_eq!(
            transcription_url("https://example.com/v1/chat/completions").unwrap(),
            "https://example.com/v1/audio/transcriptions"
        );
        assert_eq!(batch_chunk_limit_ms(AsrProviderKind::Zhipu), Some(30_000));
        assert_eq!(batch_chunk_limit_ms(AsrProviderKind::Groq), None);
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
            ..AsrConfig::default()
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
