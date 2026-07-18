use super::AsrEvent;
use super::provider::{AsrProvider, DiagnosticFuture, EventHandler, RecognitionFuture};
use super::shared::{
    collect_pcm, encode_wav, extract_request_id, http_client, join_transcripts, print_diagnosis,
    split_pcm,
};
use anyhow::{Context, Result, bail};
use base64::Engine;
use reqwest::multipart::{Form, Part};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;
use typeless_ibus::config::{AsrConfig, AsrProviderKind};

const JSON_AUDIO_CHUNK_MS: u64 = 180_000;
const OPENROUTER_AUDIO_CHUNK_MS: u64 = 30_000;

pub(crate) struct CloudBatchProvider {
    config: AsrConfig,
    client: reqwest::Client,
}

impl CloudBatchProvider {
    pub(crate) fn new(config: AsrConfig) -> Result<Self> {
        config.validate()?;
        if !matches!(
            config.provider,
            AsrProviderKind::Openrouter
                | AsrProviderKind::Elevenlabs
                | AsrProviderKind::XiaomiMimoAsr
                | AsrProviderKind::BailianFunAsrFlash
        ) {
            bail!("{} 不是批量云端 ASR provider", config.provider.as_str());
        }
        Ok(Self {
            config,
            client: http_client(Duration::from_secs(600))?,
        })
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
        let text = match self.config.provider {
            AsrProviderKind::Elevenlabs => self.transcribe_elevenlabs(pcm).await?,
            AsrProviderKind::Openrouter => {
                let mut texts = Vec::new();
                for chunk in split_pcm(pcm, OPENROUTER_AUDIO_CHUNK_MS) {
                    texts.push(self.transcribe_openrouter(chunk).await?);
                }
                join_transcripts(&texts)
            }
            AsrProviderKind::XiaomiMimoAsr | AsrProviderKind::BailianFunAsrFlash => {
                let mut texts = Vec::new();
                for chunk in split_pcm(pcm, JSON_AUDIO_CHUNK_MS) {
                    let text = match self.config.provider {
                        AsrProviderKind::XiaomiMimoAsr => self.transcribe_mimo(chunk).await?,
                        AsrProviderKind::BailianFunAsrFlash => {
                            self.transcribe_dashscope(chunk).await?
                        }
                        _ => unreachable!(),
                    };
                    texts.push(text);
                }
                join_transcripts(&texts)
            }
            _ => unreachable!("provider kind was validated"),
        };
        let text = text.trim().to_string();
        if text.is_empty() {
            bail!(
                "{} 完成请求但没有返回识别文字",
                self.config.provider.as_str()
            );
        }
        Ok(text)
    }

    async fn transcribe_elevenlabs(&self, pcm: &[u8]) -> Result<String> {
        let wav = encode_wav(pcm)?;
        let file = Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .context("创建 ElevenLabs 音频表单失败")?;
        let mut form = Form::new()
            .part("file", file)
            .text("model_id", self.config.model().to_string())
            .text("tag_audio_events", "false")
            .text("timestamps_granularity", "none");
        if let Some(language) = self.config.language() {
            form = form.text("language_code", language);
        }
        let response = self
            .client
            .post(elevenlabs_url(self.config.endpoint())?)
            .header(
                "xi-api-key",
                self.config.api_key().context("ElevenLabs 缺少 API Key")?,
            )
            .multipart(form)
            .send()
            .await
            .context("请求 ElevenLabs ASR 失败")?;
        parse_json_response(response, self.config.provider, extract_elevenlabs_text).await
    }

    async fn transcribe_mimo(&self, pcm: &[u8]) -> Result<String> {
        let wav = encode_wav(pcm)?;
        let body = json!({
            "model": self.config.model(),
            "stream": false,
            "messages": [{
                "role": "user",
                "content": [{
                    "type": "input_audio",
                    "input_audio": {
                        "data": format!(
                            "data:audio/wav;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(wav)
                        ),
                        "format": "wav"
                    }
                }]
            }]
        });
        let response = self
            .client
            .post(mimo_url(self.config.endpoint())?)
            .bearer_auth(self.config.api_key().context("MiMo 缺少 API Key")?)
            .json(&body)
            .send()
            .await
            .context("请求小米 MiMo ASR 失败")?;
        parse_json_response(response, self.config.provider, extract_mimo_text).await
    }

    async fn transcribe_openrouter(&self, pcm: &[u8]) -> Result<String> {
        let wav = encode_wav(pcm)?;
        let body = json!({
            "model": self.config.model(),
            "input_audio": {
                "data": base64::engine::general_purpose::STANDARD.encode(wav),
                "format": "wav"
            }
        });
        let response = self
            .client
            .post(openrouter_url(self.config.endpoint())?)
            .bearer_auth(self.config.api_key().context("OpenRouter 缺少 API Key")?)
            .json(&body)
            .send()
            .await
            .context("请求 OpenRouter ASR 失败")?;
        parse_json_response(response, self.config.provider, extract_openrouter_text).await
    }

    async fn transcribe_dashscope(&self, pcm: &[u8]) -> Result<String> {
        let wav = encode_wav(pcm)?;
        let body = json!({
            "model": self.config.model(),
            "input": {
                "messages": [{
                    "role": "user",
                    "content": [{
                        "type": "input_audio",
                        "input_audio": {
                            "data": format!(
                                "data:audio/wav;base64,{}",
                                base64::engine::general_purpose::STANDARD.encode(wav)
                            )
                        }
                    }]
                }]
            },
            "parameters": { "format": "wav", "sample_rate": "16000" }
        });
        let response = self
            .client
            .post(dashscope_url(self.config.endpoint())?)
            .bearer_auth(self.config.api_key().context("百炼缺少 API Key")?)
            .header("X-DashScope-SSE", "disable")
            .json(&body)
            .send()
            .await
            .context("请求百炼 Fun-ASR-Flash 失败")?;
        parse_json_response(response, self.config.provider, extract_dashscope_text).await
    }
}

impl AsrProvider for CloudBatchProvider {
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
        Box::pin(async move { print_diagnosis(&self.config, "configured (secret redacted)") })
    }
}

async fn parse_json_response(
    response: reqwest::Response,
    provider: AsrProviderKind,
    extract: fn(&Value) -> Result<String>,
) -> Result<String> {
    let status = response.status();
    let header_request_id = extract_request_id(response.headers());
    if !status.is_success() {
        let body_request_id = response.json::<Value>().await.ok().and_then(|payload| {
            payload
                .get("request_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
        let request_id = header_request_id.or(body_request_id);
        tracing::error!(
            provider = provider.as_str(),
            request_id = request_id.as_deref().unwrap_or("missing"),
            http_status = %status,
            "ASR request failed"
        );
        bail!("{} 返回 HTTP {status}", provider.as_str());
    }
    let payload: Value = response
        .json()
        .await
        .with_context(|| format!("解析 {} ASR 响应失败", provider.as_str()))?;
    let request_id = header_request_id.or_else(|| {
        payload
            .get("request_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let text = extract(&payload)?;
    tracing::info!(
        provider = provider.as_str(),
        request_id = request_id.as_deref().unwrap_or("missing"),
        "ASR request completed"
    );
    Ok(text)
}

fn elevenlabs_url(endpoint: &str) -> Result<String> {
    append_endpoint_path(endpoint, "/speech-to-text")
}

fn mimo_url(endpoint: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(endpoint).context("解析 ASR endpoint 失败")?;
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with("/chat/completions") {
        path.to_string()
    } else if path.ends_with("/chat") {
        format!("{path}/completions")
    } else {
        format!("{path}/chat/completions")
    };
    url.set_path(&path);
    Ok(url.to_string())
}

fn openrouter_url(endpoint: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(endpoint).context("解析 ASR endpoint 失败")?;
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with("/audio/transcriptions") {
        path.to_string()
    } else if path.ends_with("/audio") {
        format!("{path}/transcriptions")
    } else {
        format!("{path}/audio/transcriptions")
    };
    url.set_path(&path);
    Ok(url.to_string())
}

fn dashscope_url(endpoint: &str) -> Result<String> {
    const SUFFIX: &str = "/multimodal-generation/generation";
    const SERVICE_PATH: &str = "/services/aigc/multimodal-generation/generation";
    const CANONICAL_PATH: &str = "/api/v1/services/aigc/multimodal-generation/generation";
    let mut url = reqwest::Url::parse(endpoint).context("解析 ASR endpoint 失败")?;
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with(SUFFIX) {
        path.to_string()
    } else if path.ends_with("/api/v1") {
        format!("{path}{SERVICE_PATH}")
    } else if path.is_empty() {
        CANONICAL_PATH.to_string()
    } else {
        format!("{path}{CANONICAL_PATH}")
    };
    url.set_path(&path);
    Ok(url.to_string())
}

fn append_endpoint_path(endpoint: &str, suffix: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(endpoint).context("解析 ASR endpoint 失败")?;
    let path = url.path().trim_end_matches('/').to_string();
    if path.ends_with(suffix) {
        url.set_path(&path);
    } else {
        url.set_path(&format!("{path}{suffix}"));
    }
    Ok(url.to_string())
}

fn extract_elevenlabs_text(payload: &Value) -> Result<String> {
    if let Some(text) = payload.get("text").and_then(Value::as_str) {
        return Ok(text.to_string());
    }
    if let Some(transcripts) = payload.get("transcripts").and_then(Value::as_array) {
        return Ok(transcripts
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "));
    }
    bail!("ElevenLabs ASR 响应缺少 text 或 transcripts")
}

fn extract_mimo_text(payload: &Value) -> Result<String> {
    let content = payload
        .pointer("/choices/0/message/content")
        .context("MiMo ASR 响应缺少 choices[0].message.content")?;
    if let Some(text) = content.as_str() {
        return Ok(text.to_string());
    }
    if let Some(items) = content.as_array() {
        return Ok(items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<String>());
    }
    bail!("MiMo ASR 响应 content 格式不受支持")
}

fn extract_openrouter_text(payload: &Value) -> Result<String> {
    payload
        .get("text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .context("OpenRouter ASR 响应缺少 text")
}

fn extract_dashscope_text(payload: &Value) -> Result<String> {
    let paths = [
        "/output/text",
        "/output/output/sentence/text",
        "/output/sentence/text",
    ];
    for path in paths {
        if let Some(text) = payload.pointer(path).and_then(Value::as_str)
            && !text.trim().is_empty()
        {
            return Ok(text.to_string());
        }
    }
    if let Some(content) = payload.pointer("/output/choices/0/message/content") {
        if let Some(text) = content.as_str() {
            return Ok(text.to_string());
        }
        if let Some(items) = content.as_array() {
            return Ok(items
                .iter()
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect());
        }
    }
    bail!("百炼 Fun-ASR-Flash 响应缺少识别文字")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn normalizes_each_batch_endpoint() {
        assert_eq!(
            elevenlabs_url("https://api.elevenlabs.io/v1").unwrap(),
            "https://api.elevenlabs.io/v1/speech-to-text"
        );
        assert_eq!(
            mimo_url("https://api.xiaomimimo.com/v1").unwrap(),
            "https://api.xiaomimimo.com/v1/chat/completions"
        );
        assert_eq!(
            mimo_url("https://api.xiaomimimo.com/v1/chat").unwrap(),
            "https://api.xiaomimimo.com/v1/chat/completions"
        );
        assert_eq!(
            openrouter_url("https://openrouter.ai/api/v1").unwrap(),
            "https://openrouter.ai/api/v1/audio/transcriptions"
        );
        assert_eq!(
            dashscope_url("https://workspace.example.com").unwrap(),
            "https://workspace.example.com/api/v1/services/aigc/multimodal-generation/generation"
        );
        assert_eq!(
            dashscope_url("https://dashscope.aliyuncs.com/api/v1").unwrap(),
            "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation"
        );
    }

    #[tokio::test]
    async fn sends_elevenlabs_multipart_protocol() {
        let (endpoint, server) = mock_http(r#"{"text":"eleven ok"}"#);
        let provider = configured(AsrProviderKind::Elevenlabs, endpoint);
        assert_eq!(
            provider.transcribe_pcm(&[0_u8; 640]).await.unwrap(),
            "eleven ok"
        );
        let request = String::from_utf8_lossy(&server.join().unwrap()).to_string();
        let lower = request.to_ascii_lowercase();
        assert!(request.starts_with("POST /v1/speech-to-text HTTP/1.1"));
        assert!(lower.contains("xi-api-key: test-key"));
        assert!(request.contains("name=\"model_id\""));
        assert!(request.contains("scribe_v2"));
    }

    #[tokio::test]
    async fn sends_mimo_audio_chat_protocol() {
        let (endpoint, server) = mock_http(r#"{"choices":[{"message":{"content":"mimo ok"}}]}"#);
        let provider = configured(AsrProviderKind::XiaomiMimoAsr, endpoint);
        assert_eq!(
            provider.transcribe_pcm(&[0_u8; 640]).await.unwrap(),
            "mimo ok"
        );
        let request = String::from_utf8_lossy(&server.join().unwrap()).to_string();
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(request.contains("\"type\":\"input_audio\""));
        assert!(request.contains("data:audio/wav;base64,"));
    }

    #[tokio::test]
    async fn sends_openrouter_json_audio_protocol() {
        let (endpoint, server) = mock_http(r#"{"text":"openrouter ok"}"#);
        let provider = configured(AsrProviderKind::Openrouter, endpoint);
        assert_eq!(
            provider.transcribe_pcm(&[0_u8; 640]).await.unwrap(),
            "openrouter ok"
        );
        let request = String::from_utf8_lossy(&server.join().unwrap()).to_string();
        assert!(request.starts_with("POST /v1/audio/transcriptions HTTP/1.1"));
        assert!(request.contains("\"input_audio\""));
        assert!(request.contains("\"format\":\"wav\""));
        assert!(!request.contains("multipart/form-data"));
    }

    #[tokio::test]
    async fn sends_dashscope_multimodal_protocol() {
        let (endpoint, server) = mock_http(r#"{"output":{"text":"dashscope ok"}}"#);
        let provider = configured(
            AsrProviderKind::BailianFunAsrFlash,
            endpoint.trim_end_matches("/v1").to_string(),
        );
        assert_eq!(
            provider.transcribe_pcm(&[0_u8; 640]).await.unwrap(),
            "dashscope ok"
        );
        let request = String::from_utf8_lossy(&server.join().unwrap()).to_string();
        assert!(
            request.starts_with(
                "POST /api/v1/services/aigc/multimodal-generation/generation HTTP/1.1"
            )
        );
        assert!(request.contains("x-dashscope-sse: disable"));
        assert!(request.contains("fun-asr-flash-2026-06-15"));
    }

    fn configured(kind: AsrProviderKind, endpoint: String) -> CloudBatchProvider {
        CloudBatchProvider::new(AsrConfig {
            provider: kind,
            endpoint: Some(endpoint),
            api_key: Some("test-key".to_string()),
            ..AsrConfig::default()
        })
        .unwrap()
    }

    fn mock_http(response_body: &'static str) -> (String, std::thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let request = read_http_request(&mut stream);
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Request-Id: mock-cloud-id\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            )
            .unwrap();
            request
        });
        (format!("http://{address}/v1"), server)
    }

    fn read_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected_len = None;
        loop {
            let count = stream.read(&mut buffer).unwrap();
            assert!(count > 0);
            request.extend_from_slice(&buffer[..count]);
            if expected_len.is_none()
                && let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
            {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap();
                expected_len = Some(header_end + 4 + content_length);
            }
            if expected_len.is_some_and(|length| request.len() >= length) {
                return request;
            }
        }
    }
}
