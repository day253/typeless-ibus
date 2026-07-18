use super::AsrEvent;
use super::provider::{AsrProvider, DiagnosticFuture, EventHandler, RecognitionFuture};
use super::shared::{extract_request_id, join_transcripts, print_diagnosis};
use super::volcengine_frame::{self as frame, Flags, MessageType, Serialization};
use anyhow::{Context, Result, anyhow, bail};
use futures_util::{SinkExt, StreamExt};
use http::HeaderValue;
use serde_json::{Value, json};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use typeless_ibus::config::{AsrConfig, AsrProviderKind};
use uuid::Uuid;

const AUDIO_CHUNK_BYTES: usize = 6_400;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const FINAL_TIMEOUT: Duration = Duration::from_secs(30);
const SESSION_TIMEOUT: Duration = Duration::from_secs(660);

pub(crate) struct VolcengineProvider {
    config: AsrConfig,
}

impl VolcengineProvider {
    pub(crate) fn new(config: AsrConfig) -> Result<Self> {
        config.validate()?;
        if config.provider != AsrProviderKind::Volcengine {
            bail!("{} 不是火山引擎 provider", config.provider.as_str());
        }
        Ok(Self { config })
    }

    async fn transcribe_stream(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        mut on_event: EventHandler,
    ) -> Result<String> {
        let connect_id = Uuid::new_v4().to_string();
        let request = build_websocket_request(&self.config, &connect_id)?;
        let (mut socket, response) = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(request))
            .await
            .context("连接火山引擎 ASR 超时")?
            .context("连接火山引擎 ASR 失败")?;
        let request_id = extract_request_id(response.headers());
        tracing::info!(
            provider = self.config.provider.as_str(),
            request_id = request_id.as_deref().unwrap_or("missing"),
            "connected to ASR provider"
        );

        let first_payload = json!({
            "user": { "uid": connect_id },
            "audio": { "format": "pcm", "rate": 16000, "bits": 16, "channel": 1, "codec": "raw" },
            "request": {
                "model_name": "bigmodel",
                "enable_itn": true,
                "enable_punc": true,
                "show_utterances": true,
                "enable_speaker_info": true
            }
        });
        socket
            .send(Message::Binary(frame::build(
                MessageType::FullClientRequest,
                Flags::PositiveSequence,
                Serialization::Json,
                &serde_json::to_vec(&first_payload)?,
                Some(1),
            )))
            .await
            .context("发送火山引擎初始化帧失败")?;

        let mut sequence = 2_i32;
        let mut pending_audio = Vec::new();
        let mut audio_finished = false;
        let mut final_sent = false;
        let mut latest_text = String::new();
        let mut deadline = Instant::now() + SESSION_TIMEOUT;

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => bail!("火山引擎 ASR 会话超时"),
                audio = audio_rx.recv(), if !audio_finished => {
                    match audio {
                        Some(chunk) => pending_audio.extend_from_slice(&chunk),
                        None => audio_finished = true,
                    }
                }
                incoming = socket.next() => {
                    let incoming = match incoming {
                        Some(message) => message.context("读取火山引擎 ASR 响应失败")?,
                        None if !latest_text.is_empty() => return Ok(latest_text),
                        None => return Err(anyhow!("火山引擎 ASR 连接已关闭")),
                    };
                    let Message::Binary(data) = incoming else { continue };
                    let parsed = frame::parse(&data).context("解析火山引擎 ASR 二进制帧失败")?;
                    if parsed.message_type == Some(MessageType::ErrorMessage) {
                        bail!(
                            "火山引擎 ASR 返回错误码 {}",
                            parsed.error_code.unwrap_or_default()
                        );
                    }
                    if parsed.message_type != Some(MessageType::FullServerResponse) {
                        continue;
                    }
                    let payload: Value = serde_json::from_slice(&parsed.payload)
                        .context("解析火山引擎 ASR JSON 失败")?;
                    if let Some(text) = extract_volcengine_text(&payload)
                        && !text.is_empty()
                    {
                        latest_text.clone_from(&text);
                        if parsed.is_final() {
                            on_event(AsrEvent::Final(text));
                            break;
                        }
                        on_event(AsrEvent::Partial(text));
                    } else if parsed.is_final() {
                        break;
                    }
                }
            }

            send_audio_chunks(&mut socket, &mut pending_audio, &mut sequence, false).await?;
            if audio_finished && !final_sent {
                send_audio_chunks(&mut socket, &mut pending_audio, &mut sequence, true).await?;
                socket
                    .send(Message::Binary(frame::build(
                        MessageType::AudioOnlyRequest,
                        Flags::NegativeSequence,
                        Serialization::None,
                        &[],
                        Some(-sequence),
                    )))
                    .await
                    .context("发送火山引擎结束帧失败")?;
                final_sent = true;
                deadline = Instant::now() + FINAL_TIMEOUT;
            }
        }

        if latest_text.trim().is_empty() {
            bail!("火山引擎 ASR 没有返回识别文字");
        }
        Ok(latest_text)
    }
}

impl AsrProvider for VolcengineProvider {
    fn kind(&self) -> AsrProviderKind {
        AsrProviderKind::Volcengine
    }

    fn transcribe<'a>(
        &'a self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        on_event: EventHandler,
    ) -> RecognitionFuture<'a> {
        Box::pin(self.transcribe_stream(audio_rx, on_event))
    }

    fn diagnose<'a>(&'a self) -> DiagnosticFuture<'a> {
        Box::pin(
            async move { print_diagnosis(&self.config, "apiKey configured (secret redacted)") },
        )
    }
}

fn build_websocket_request(config: &AsrConfig, connect_id: &str) -> Result<http::Request<()>> {
    let mut request = config
        .endpoint()
        .into_client_request()
        .context("创建火山引擎 WebSocket 请求失败")?;

    insert_header(
        &mut request,
        "X-Api-Key",
        config.api_key().context("火山引擎缺少 apiKey")?,
    )?;
    insert_header(&mut request, "X-Api-Resource-Id", config.resource_id())?;
    insert_header(&mut request, "X-Api-Connect-Id", connect_id)?;
    Ok(request)
}

fn insert_header(request: &mut http::Request<()>, name: &'static str, value: &str) -> Result<()> {
    request.headers_mut().insert(
        name,
        HeaderValue::from_str(value).with_context(|| format!("{name} 不能写入请求头"))?,
    );
    Ok(())
}

async fn send_audio_chunks<S>(
    socket: &mut S,
    pending: &mut Vec<u8>,
    sequence: &mut i32,
    flush_tail: bool,
) -> Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    while pending.len() >= AUDIO_CHUNK_BYTES {
        let chunk = pending.drain(..AUDIO_CHUNK_BYTES).collect::<Vec<_>>();
        send_audio(socket, &chunk, *sequence).await?;
        *sequence += 1;
    }
    if flush_tail && !pending.is_empty() {
        let tail = std::mem::take(pending);
        send_audio(socket, &tail, *sequence).await?;
        *sequence += 1;
    }
    Ok(())
}

async fn send_audio<S>(socket: &mut S, pcm: &[u8], sequence: i32) -> Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    socket
        .send(Message::Binary(frame::build(
            MessageType::AudioOnlyRequest,
            Flags::PositiveSequence,
            Serialization::None,
            pcm,
            Some(sequence),
        )))
        .await
        .context("发送火山引擎音频帧失败")
}

fn extract_volcengine_text(payload: &Value) -> Option<String> {
    let result = match payload.get("result") {
        Some(Value::Object(_)) => payload.get("result")?,
        Some(Value::Array(results)) => results.first()?,
        _ if payload.get("text").is_some() => payload,
        _ => return None,
    };
    if let Some(utterances) = result.get("utterances").and_then(Value::as_array) {
        let segments = utterances
            .iter()
            .filter_map(|utterance| utterance.get("text").and_then(Value::as_str))
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !segments.is_empty() {
            return Some(join_transcripts(&segments));
        }
    }
    result
        .get("text")
        .and_then(Value::as_str)
        .map(|text| text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_hdr_async;

    #[test]
    fn extracts_object_array_and_utterance_results() {
        assert_eq!(
            extract_volcengine_text(&json!({ "result": { "text": "hello" } })),
            Some("hello".to_string())
        );
        assert_eq!(
            extract_volcengine_text(&json!({
                "result": [{ "utterances": [{ "text": "你" }, { "text": "好" }] }]
            })),
            Some("你好".to_string())
        );
    }

    #[test]
    fn uses_single_api_key_auth() {
        let config = AsrConfig {
            provider: AsrProviderKind::Volcengine,
            api_key: Some("api-key".to_string()),
            ..AsrConfig::default()
        };
        let request = build_websocket_request(&config, "connect-id").unwrap();
        assert_eq!(request.headers()["x-api-key"], "api-key");
        assert!(!request.headers().contains_key("x-api-app-key"));
        assert!(!request.headers().contains_key("x-api-access-key"));
        assert_eq!(
            request.headers()["x-api-resource-id"],
            "volc.seedasr.sauc.duration"
        );
        assert_eq!(request.headers()["x-api-connect-id"], "connect-id");
    }

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn volcengine_websocket_protocol_returns_transcript() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_hdr_async(stream, |request: &http::Request<()>, response| {
                assert_eq!(request.headers()["x-api-key"], "api-key");
                assert!(!request.headers().contains_key("x-api-app-key"));
                assert!(!request.headers().contains_key("x-api-access-key"));
                assert_eq!(
                    request.headers()["x-api-resource-id"],
                    "volc.seedasr.sauc.duration"
                );
                Ok(response)
            })
            .await
            .unwrap();
            let first = socket.next().await.unwrap().unwrap().into_data();
            assert_eq!(
                frame::parse(&first).unwrap().message_type,
                Some(MessageType::FullClientRequest)
            );
            let audio = socket.next().await.unwrap().unwrap().into_data();
            assert_eq!(
                frame::parse(&audio).unwrap().message_type,
                Some(MessageType::AudioOnlyRequest)
            );
            let end = socket.next().await.unwrap().unwrap().into_data();
            assert!(frame::parse(&end).unwrap().is_final());
            let payload = serde_json::to_vec(&json!({
                "result": { "text": "火山成功" }
            }))
            .unwrap();
            socket
                .send(Message::Binary(frame::build(
                    MessageType::FullServerResponse,
                    Flags::NegativeSequence,
                    Serialization::Json,
                    &payload,
                    Some(-1),
                )))
                .await
                .unwrap();
        });
        let provider = VolcengineProvider::new(AsrConfig {
            provider: AsrProviderKind::Volcengine,
            endpoint: Some(format!("ws://{address}")),
            api_key: Some("api-key".to_string()),
            ..AsrConfig::default()
        })
        .unwrap();
        let (tx, rx) = mpsc::channel(2);
        tx.send(vec![0_u8; 640]).await.unwrap();
        drop(tx);
        let text = provider
            .transcribe_stream(rx, Box::new(|_| {}))
            .await
            .unwrap();
        assert_eq!(text, "火山成功");
        server.await.unwrap();
    }
}
