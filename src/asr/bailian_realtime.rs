use super::AsrEvent;
use super::provider::{AsrProvider, DiagnosticFuture, EventHandler, RecognitionFuture};
use super::shared::{extract_request_id, join_transcripts, non_empty, print_diagnosis};
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use http::HeaderValue;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use typeless_ibus::config::{AsrConfig, AsrProviderKind};
use uuid::Uuid;

const AUDIO_CHUNK_BYTES: usize = 3_200;
const SESSION_START_TIMEOUT: Duration = Duration::from_secs(10);
const FINAL_TIMEOUT: Duration = Duration::from_secs(30);
const SESSION_TIMEOUT: Duration = Duration::from_secs(660);

pub(crate) struct BailianRealtimeProvider {
    config: AsrConfig,
}

impl BailianRealtimeProvider {
    pub(crate) fn new(config: AsrConfig) -> Result<Self> {
        config.validate()?;
        if !matches!(
            config.provider,
            AsrProviderKind::Bailian | AsrProviderKind::BailianQwen3Realtime
        ) {
            bail!("{} 不是百炼实时 provider", config.provider.as_str());
        }
        Ok(Self { config })
    }

    async fn transcribe_stream(
        &self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        on_event: EventHandler,
    ) -> Result<String> {
        match self.config.provider {
            AsrProviderKind::Bailian => self.transcribe_classic(audio_rx, on_event).await,
            AsrProviderKind::BailianQwen3Realtime => {
                self.transcribe_qwen3(audio_rx, on_event).await
            }
            _ => unreachable!(),
        }
    }

    async fn transcribe_classic(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        mut on_event: EventHandler,
    ) -> Result<String> {
        let mut request = self
            .config
            .endpoint()
            .into_client_request()
            .context("创建百炼 WebSocket 请求失败")?;
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&format!(
                "bearer {}",
                self.config.api_key().context("百炼缺少 API Key")?
            ))
            .context("百炼 API Key 不能写入请求头")?,
        );
        let (mut socket, response) =
            tokio::time::timeout(SESSION_START_TIMEOUT, connect_async(request))
                .await
                .context("连接百炼实时 ASR 超时")?
                .context("连接百炼实时 ASR 失败")?;
        let request_id = extract_request_id(response.headers());
        tracing::info!(
            provider = self.config.provider.as_str(),
            request_id = request_id.as_deref().unwrap_or("missing"),
            "connected to ASR provider"
        );

        let task_id = Uuid::new_v4().simple().to_string();
        socket
            .send(Message::Text(classic_run_task(
                &task_id,
                self.config.model(),
            )))
            .await
            .context("发送百炼 run-task 失败")?;

        let mut pending_audio = Vec::new();
        let mut audio_finished = false;
        let mut task_started = false;
        let mut finish_sent = false;
        let mut final_segments = BTreeMap::<i64, String>::new();
        let mut partial_segments = BTreeMap::<i64, String>::new();
        let mut last_text = String::new();
        let started_at = Instant::now();
        let mut deadline = started_at + SESSION_START_TIMEOUT;

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    bail!("百炼实时 ASR 会话超时");
                }
                audio = audio_rx.recv(), if !audio_finished => {
                    match audio {
                        Some(chunk) => pending_audio.extend_from_slice(&chunk),
                        None => audio_finished = true,
                    }
                }
                incoming = socket.next() => {
                    let message = incoming
                        .ok_or_else(|| anyhow!("百炼实时 ASR 连接已关闭"))?
                        .context("读取百炼实时 ASR 响应失败")?;
                    let Message::Text(text) = message else { continue };
                    let payload: Value = serde_json::from_str(&text)
                        .context("解析百炼实时 ASR 事件失败")?;
                    let event = payload.pointer("/header/event").and_then(Value::as_str).unwrap_or("");
                    match event {
                        "task-started" => {
                            task_started = true;
                            deadline = started_at + SESSION_TIMEOUT;
                        }
                        "result-generated" => {
                            if let Some((sentence_id, text, is_final)) = classic_result(&payload) {
                                last_text.clone_from(&text);
                                if is_final {
                                    final_segments.insert(sentence_id, text);
                                    partial_segments.remove(&sentence_id);
                                } else {
                                    partial_segments.insert(sentence_id, text);
                                }
                                let visible = classic_visible_text(&final_segments, &partial_segments);
                                if is_final {
                                    on_event(AsrEvent::Final(visible));
                                } else {
                                    on_event(AsrEvent::Partial(visible));
                                }
                            }
                        }
                        "task-finished" => break,
                        "task-failed" => {
                            let message = payload.pointer("/header/error_message")
                                .and_then(Value::as_str)
                                .unwrap_or("task failed");
                            bail!("百炼实时 ASR 任务失败：{message}");
                        }
                        _ => {}
                    }
                }
            }

            if task_started {
                send_classic_audio_chunks(&mut socket, &mut pending_audio, false).await?;
                if audio_finished && !finish_sent {
                    send_classic_audio_chunks(&mut socket, &mut pending_audio, true).await?;
                    socket
                        .send(Message::Text(classic_finish_task(&task_id)))
                        .await
                        .context("发送百炼 finish-task 失败")?;
                    finish_sent = true;
                    deadline = Instant::now() + FINAL_TIMEOUT;
                }
            }
        }

        let text = if final_segments.is_empty() {
            if last_text.is_empty() {
                classic_visible_text(&final_segments, &partial_segments)
            } else {
                last_text
            }
        } else {
            join_transcripts(&final_segments.into_values().collect::<Vec<_>>())
        };
        if text.trim().is_empty() {
            bail!("百炼实时 ASR 没有返回识别文字");
        }
        Ok(text)
    }

    async fn transcribe_qwen3(
        &self,
        mut audio_rx: mpsc::Receiver<Vec<u8>>,
        mut on_event: EventHandler,
    ) -> Result<String> {
        let url = qwen_connect_url(self.config.endpoint(), self.config.model())?;
        let mut request = url
            .into_client_request()
            .context("创建百炼 Qwen3 WebSocket 请求失败")?;
        request.headers_mut().insert(
            "Authorization",
            HeaderValue::from_str(&format!(
                "Bearer {}",
                self.config.api_key().context("百炼缺少 API Key")?
            ))
            .context("百炼 API Key 不能写入请求头")?,
        );
        request
            .headers_mut()
            .insert("OpenAI-Beta", HeaderValue::from_static("realtime=v1"));
        let (mut socket, response) =
            tokio::time::timeout(SESSION_START_TIMEOUT, connect_async(request))
                .await
                .context("连接百炼 Qwen3 Realtime 超时")?
                .context("连接百炼 Qwen3 Realtime 失败")?;
        let request_id = extract_request_id(response.headers());
        tracing::info!(
            provider = self.config.provider.as_str(),
            request_id = request_id.as_deref().unwrap_or("missing"),
            "connected to ASR provider"
        );
        socket
            .send(Message::Text(qwen_session_update(self.config.language())))
            .await
            .context("发送百炼 Qwen3 session.update 失败")?;

        let mut pending_audio = Vec::new();
        let mut audio_finished = false;
        let mut session_started = false;
        let mut finish_sent = false;
        let mut completed = Vec::<String>::new();
        let mut partial = String::new();
        let started_at = Instant::now();
        let mut deadline = started_at + SESSION_START_TIMEOUT;

        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => bail!("百炼 Qwen3 Realtime 会话超时"),
                audio = audio_rx.recv(), if !audio_finished => {
                    match audio {
                        Some(chunk) => pending_audio.extend_from_slice(&chunk),
                        None => audio_finished = true,
                    }
                }
                incoming = socket.next() => {
                    let message = incoming
                        .ok_or_else(|| anyhow!("百炼 Qwen3 Realtime 连接已关闭"))?
                        .context("读取百炼 Qwen3 Realtime 响应失败")?;
                    let Message::Text(text) = message else { continue };
                    let payload: Value = serde_json::from_str(&text)
                        .context("解析百炼 Qwen3 Realtime 事件失败")?;
                    match payload.get("type").and_then(Value::as_str).unwrap_or("") {
                        "session.updated" => {
                            session_started = true;
                            deadline = started_at + SESSION_TIMEOUT;
                        }
                        "conversation.item.input_audio_transcription.text" => {
                            if let Some(text) = qwen_partial(&payload) {
                                partial = text;
                                on_event(AsrEvent::Partial(qwen_visible_text(&completed, &partial)));
                            }
                        }
                        "conversation.item.input_audio_transcription.completed" => {
                            if let Some(text) = payload.get("transcript").and_then(Value::as_str)
                                && !text.trim().is_empty()
                            {
                                completed.push(text.trim().to_string());
                                partial.clear();
                                on_event(AsrEvent::Final(qwen_visible_text(&completed, &partial)));
                            }
                        }
                        "session.finished" => break,
                        "conversation.item.input_audio_transcription.failed" | "error" => {
                            let message = payload.pointer("/error/message")
                                .and_then(Value::as_str)
                                .or_else(|| payload.get("message").and_then(Value::as_str))
                                .unwrap_or("realtime task failed");
                            bail!("百炼 Qwen3 Realtime 任务失败：{message}");
                        }
                        _ => {}
                    }
                }
            }

            if session_started {
                send_qwen_audio_chunks(&mut socket, &mut pending_audio, false).await?;
                if audio_finished && !finish_sent {
                    send_qwen_audio_chunks(&mut socket, &mut pending_audio, true).await?;
                    socket
                        .send(Message::Text(
                            json!({
                                "type": "session.finish",
                                "event_id": event_id()
                            })
                            .to_string(),
                        ))
                        .await
                        .context("发送百炼 Qwen3 session.finish 失败")?;
                    finish_sent = true;
                    deadline = Instant::now() + FINAL_TIMEOUT;
                }
            }
        }

        if !partial.is_empty() {
            completed.push(partial);
        }
        let text = join_transcripts(&completed);
        if text.trim().is_empty() {
            bail!("百炼 Qwen3 Realtime 没有返回识别文字");
        }
        Ok(text)
    }
}

impl AsrProvider for BailianRealtimeProvider {
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

async fn send_classic_audio_chunks<S>(
    socket: &mut S,
    pending: &mut Vec<u8>,
    flush_tail: bool,
) -> Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    while pending.len() >= AUDIO_CHUNK_BYTES {
        let chunk = pending.drain(..AUDIO_CHUNK_BYTES).collect::<Vec<_>>();
        socket
            .send(Message::Binary(chunk))
            .await
            .context("发送百炼实时音频失败")?;
    }
    if flush_tail && !pending.is_empty() {
        let tail = std::mem::take(pending);
        socket
            .send(Message::Binary(tail))
            .await
            .context("发送百炼实时尾部音频失败")?;
    }
    Ok(())
}

async fn send_qwen_audio_chunks<S>(
    socket: &mut S,
    pending: &mut Vec<u8>,
    flush_tail: bool,
) -> Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    while pending.len() >= AUDIO_CHUNK_BYTES {
        let chunk = pending.drain(..AUDIO_CHUNK_BYTES).collect::<Vec<_>>();
        send_qwen_audio(socket, &chunk).await?;
    }
    if flush_tail && !pending.is_empty() {
        let tail = std::mem::take(pending);
        send_qwen_audio(socket, &tail).await?;
    }
    Ok(())
}

async fn send_qwen_audio<S>(socket: &mut S, pcm: &[u8]) -> Result<()>
where
    S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    socket
        .send(Message::Text(
            json!({
                "type": "input_audio_buffer.append",
                "event_id": event_id(),
                "audio": base64::engine::general_purpose::STANDARD.encode(pcm)
            })
            .to_string(),
        ))
        .await
        .context("发送百炼 Qwen3 音频失败")
}

fn classic_run_task(task_id: &str, model: &str) -> String {
    json!({
        "header": { "action": "run-task", "task_id": task_id, "streaming": "duplex" },
        "payload": {
            "task_group": "audio",
            "task": "asr",
            "function": "recognition",
            "model": model,
            "parameters": { "sample_rate": 16000, "format": "pcm" },
            "input": {}
        }
    })
    .to_string()
}

fn classic_finish_task(task_id: &str) -> String {
    json!({
        "header": { "action": "finish-task", "task_id": task_id, "streaming": "duplex" },
        "payload": { "input": {} }
    })
    .to_string()
}

fn classic_result(payload: &Value) -> Option<(i64, String, bool)> {
    let sentence = payload.pointer("/payload/output/sentence")?;
    if sentence.get("heartbeat").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let text = sentence.get("text").and_then(Value::as_str)?.trim();
    if text.is_empty() {
        return None;
    }
    let sentence_id = sentence
        .get("sentence_id")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let is_final = sentence
        .get("sentence_end")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            sentence
                .get("end_time")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                > 0
        });
    Some((sentence_id, text.to_string(), is_final))
}

fn classic_visible_text(
    finals: &BTreeMap<i64, String>,
    partials: &BTreeMap<i64, String>,
) -> String {
    let mut values = finals
        .iter()
        .chain(partials.iter())
        .map(|(id, text)| (*id, text.clone()))
        .collect::<Vec<_>>();
    values.sort_by_key(|(id, _)| *id);
    join_transcripts(&values.into_iter().map(|(_, text)| text).collect::<Vec<_>>())
}

fn qwen_connect_url(endpoint: &str, model: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(endpoint).context("解析百炼 Qwen3 endpoint 失败")?;
    if !url.query_pairs().any(|(name, _)| name == "model") {
        url.query_pairs_mut().append_pair("model", model);
    }
    Ok(url.to_string())
}

fn qwen_session_update(language: Option<&str>) -> String {
    let mut session = json!({
        "modalities": ["text"],
        "input_audio_format": "pcm",
        "sample_rate": 16000,
        "turn_detection": { "type": "server_vad", "silence_duration_ms": 500 }
    });
    if let Some(language) = non_empty(language) {
        session["input_audio_transcription"] = json!({ "language": language });
    }
    json!({ "type": "session.update", "event_id": event_id(), "session": session }).to_string()
}

fn qwen_partial(payload: &Value) -> Option<String> {
    payload
        .get("text")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .or_else(|| {
            payload
                .get("stash")
                .and_then(Value::as_str)
                .filter(|text| !text.trim().is_empty())
        })
        .map(|text| text.trim().to_string())
}

fn qwen_visible_text(completed: &[String], partial: &str) -> String {
    let mut segments = completed.to_vec();
    if !partial.trim().is_empty() {
        segments.push(partial.trim().to_string());
    }
    join_transcripts(&segments)
}

fn event_id() -> String {
    format!("event_{}", Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_hdr_async;

    #[test]
    fn classic_messages_match_dashscope_protocol() {
        let run: Value = serde_json::from_str(&classic_run_task("task", "model")).unwrap();
        assert_eq!(run["header"]["action"], "run-task");
        assert_eq!(run["payload"]["parameters"]["sample_rate"], 16000);
        assert!(run["payload"]["parameters"].get("vocabulary_id").is_none());
        let finish: Value = serde_json::from_str(&classic_finish_task("task")).unwrap();
        assert_eq!(finish["header"]["action"], "finish-task");
    }

    #[test]
    fn qwen_messages_use_realtime_audio_events() {
        let update: Value = serde_json::from_str(&qwen_session_update(None)).unwrap();
        assert_eq!(update["type"], "session.update");
        assert_eq!(update["session"]["input_audio_format"], "pcm");
        assert_eq!(
            qwen_connect_url("wss://example.com/realtime", "qwen-model").unwrap(),
            "wss://example.com/realtime?model=qwen-model"
        );
    }

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn classic_websocket_protocol_returns_transcript() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_hdr_async(stream, |request: &http::Request<()>, response| {
                assert_eq!(request.headers()["authorization"], "bearer test-key");
                Ok(response)
            })
            .await
            .unwrap();
            let run = socket.next().await.unwrap().unwrap().into_text().unwrap();
            let run: Value = serde_json::from_str(&run).unwrap();
            let task_id = run["header"]["task_id"].as_str().unwrap();
            socket
                .send(Message::Text(
                    json!({
                        "header": { "event": "task-started" }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            assert!(socket.next().await.unwrap().unwrap().is_binary());
            let finish = socket.next().await.unwrap().unwrap().into_text().unwrap();
            assert!(finish.contains("finish-task"));
            socket
                .send(Message::Text(
                    json!({
                        "header": { "event": "result-generated", "task_id": task_id },
                        "payload": { "output": { "sentence": {
                            "sentence_id": 0, "text": "百炼成功", "sentence_end": true
                        }}}
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    json!({
                        "header": { "event": "task-finished", "task_id": task_id }
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
        });
        let provider = BailianRealtimeProvider::new(AsrConfig {
            provider: AsrProviderKind::Bailian,
            endpoint: Some(format!("ws://{address}")),
            api_key: Some("test-key".to_string()),
            ..AsrConfig::default()
        })
        .unwrap();
        let (tx, rx) = mpsc::channel(2);
        tx.send(vec![0_u8; 640]).await.unwrap();
        drop(tx);
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let text = provider
            .transcribe_stream(
                rx,
                Box::new(move |event| {
                    captured.lock().unwrap().push(event);
                }),
            )
            .await
            .unwrap();
        assert_eq!(text, "百炼成功");
        assert!(
            events
                .lock()
                .unwrap()
                .iter()
                .any(|event| { matches!(event, AsrEvent::Final(text) if text == "百炼成功") })
        );
        server.await.unwrap();
    }

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn qwen_websocket_protocol_returns_transcript() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_hdr_async(stream, |request: &http::Request<()>, response| {
                assert_eq!(request.headers()["authorization"], "Bearer test-key");
                assert_eq!(request.headers()["openai-beta"], "realtime=v1");
                assert!(
                    request
                        .uri()
                        .query()
                        .unwrap()
                        .contains("model=qwen3-asr-flash-realtime")
                );
                Ok(response)
            })
            .await
            .unwrap();
            let update = socket.next().await.unwrap().unwrap().into_text().unwrap();
            assert!(update.contains("session.update"));
            socket
                .send(Message::Text(
                    json!({ "type": "session.updated" }).to_string(),
                ))
                .await
                .unwrap();
            let append = socket.next().await.unwrap().unwrap().into_text().unwrap();
            assert!(append.contains("input_audio_buffer.append"));
            let finish = socket.next().await.unwrap().unwrap().into_text().unwrap();
            assert!(finish.contains("session.finish"));
            socket
                .send(Message::Text(
                    json!({
                        "type": "conversation.item.input_audio_transcription.completed",
                        "transcript": "Qwen 成功"
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    json!({ "type": "session.finished" }).to_string(),
                ))
                .await
                .unwrap();
        });
        let provider = BailianRealtimeProvider::new(AsrConfig {
            provider: AsrProviderKind::BailianQwen3Realtime,
            endpoint: Some(format!("ws://{address}/realtime")),
            api_key: Some("test-key".to_string()),
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
        assert_eq!(text, "Qwen 成功");
        server.await.unwrap();
    }
}
