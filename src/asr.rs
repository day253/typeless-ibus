mod bailian_realtime;
mod cloud_batch;
mod openai_compatible;
mod provider;
mod shared;
mod volcengine;
mod volcengine_frame;

use anyhow::{Context, Result, anyhow, bail};
use futures_util::{SinkExt, StreamExt};
use http::HeaderMap;
use http::header::{HeaderValue, USER_AGENT as USER_AGENT_HEADER};
use opus::{Application, Channels, Encoder};
use prost::Message as ProstMessage;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use typeless_ibus::config::{AsrConfig, AsrProviderKind};
use typeless_ibus::system_preferences::SystemPreferences;
use uuid::Uuid;

use self::bailian_realtime::BailianRealtimeProvider;
use self::cloud_batch::CloudBatchProvider;
use self::openai_compatible::OpenaiCompatibleProvider;
use self::provider::{AsrProvider, DiagnosticFuture, EventHandler, RecognitionFuture};
use self::volcengine::VolcengineProvider;

const REGISTER_URL: &str = "https://log.snssdk.com/service/2/device_register/";
const SETTINGS_URL: &str = "https://is.snssdk.com/service/settings/v3/";
const WEBSOCKET_URL: &str = "wss://frontier-audio-ime-ws.doubao.com/ocean/api/v1/ws";
const AID: u32 = 401_734;
const USER_AGENT: &str = "com.bytedance.android.doubaoime/100102018 (Linux; U; Android 16; en_US; Pixel 7 Pro; Build/BP2A.250605.031.A2; Cronet/TTNetVersion:94cf429a 2025-11-17 QuicVersion:1f89f732 2025-05-08)";
const SAMPLE_RATE: u32 = 16_000;
const FRAME_DURATION_MS: u64 = 20;
const PCM_FRAME_BYTES: usize = 640;

#[derive(Debug, Clone)]
pub enum AsrEvent {
    SpeechStarted,
    Partial(String),
    Final(String),
}

struct DoubaoProvider {
    credentials_path: std::path::PathBuf,
}

impl AsrProvider for DoubaoProvider {
    fn kind(&self) -> AsrProviderKind {
        AsrProviderKind::Doubao
    }

    fn transcribe<'a>(
        &'a self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        on_event: EventHandler,
    ) -> RecognitionFuture<'a> {
        Box::pin(transcribe_doubao_realtime(
            audio_rx,
            &self.credentials_path,
            on_event,
        ))
    }

    fn diagnose<'a>(&'a self) -> DiagnosticFuture<'a> {
        Box::pin(diagnose_doubao_service(&self.credentials_path))
    }
}

fn configured_provider(
    config: &AsrConfig,
    credentials_path: &Path,
) -> Result<Box<dyn AsrProvider>> {
    config.validate()?;
    match config.provider {
        AsrProviderKind::Doubao => Ok(Box::new(DoubaoProvider {
            credentials_path: credentials_path.to_path_buf(),
        })),
        AsrProviderKind::OpenaiCompatible
        | AsrProviderKind::Whisper
        | AsrProviderKind::Groq
        | AsrProviderKind::Siliconflow
        | AsrProviderKind::Zhipu => Ok(Box::new(OpenaiCompatibleProvider::new(config.clone())?)),
        AsrProviderKind::Elevenlabs
        | AsrProviderKind::Openrouter
        | AsrProviderKind::XiaomiMimoAsr
        | AsrProviderKind::BailianFunAsrFlash => {
            Ok(Box::new(CloudBatchProvider::new(config.clone())?))
        }
        AsrProviderKind::Bailian | AsrProviderKind::BailianQwen3Realtime => {
            Ok(Box::new(BailianRealtimeProvider::new(config.clone())?))
        }
        AsrProviderKind::Volcengine => Ok(Box::new(VolcengineProvider::new(config.clone())?)),
    }
}

pub async fn diagnose(config: &AsrConfig, credentials_path: &Path) -> Result<()> {
    let provider = configured_provider(config, credentials_path)?;
    let preferences = SystemPreferences::current();
    println!(
        "asr.system_locale: {}",
        preferences.locale().unwrap_or("unknown")
    );
    println!(
        "asr.system_time_zone: {}",
        preferences.time_zone().unwrap_or("unknown")
    );
    println!(
        "asr.system_speech_language: {}",
        preferences.speech_language()
    );
    tracing::info!(
        provider = provider.kind().as_str(),
        "diagnosing ASR provider"
    );
    provider.diagnose().await
}

pub async fn transcribe<F>(
    config: &AsrConfig,
    audio_rx: mpsc::Receiver<Vec<u8>>,
    credentials_path: &Path,
    on_event: F,
) -> Result<String>
where
    F: FnMut(AsrEvent) + Send + 'static,
{
    let provider = configured_provider(config, credentials_path)?;
    tracing::info!(provider = provider.kind().as_str(), "starting ASR session");
    provider.transcribe(audio_rx, Box::new(on_event)).await
}

#[derive(Debug, Default)]
struct TranscriptAccumulator {
    confirmed: String,
    current: String,
    current_final: bool,
}

impl TranscriptAccumulator {
    fn apply(&mut self, event: AsrEvent) -> AsrEvent {
        match event {
            AsrEvent::SpeechStarted => {
                self.freeze_current();
                let text = self.text();
                if text.is_empty() {
                    AsrEvent::SpeechStarted
                } else {
                    AsrEvent::Partial(text)
                }
            }
            AsrEvent::Partial(text) => {
                if self.starts_new_segment(&text) {
                    self.freeze_current();
                }
                self.current = text;
                self.current_final = false;
                AsrEvent::Partial(self.text())
            }
            AsrEvent::Final(text) => {
                self.current = text;
                self.current_final = true;
                AsrEvent::Final(self.text())
            }
        }
    }

    fn freeze_current(&mut self) {
        if !self.current.is_empty() {
            self.confirmed = merge_transcript(&self.confirmed, &self.current);
            self.current.clear();
        }
        self.current_final = false;
    }

    fn starts_new_segment(&self, incoming: &str) -> bool {
        if self.current.is_empty()
            || incoming.starts_with(&self.current)
            || self.current.starts_with(incoming)
        {
            return false;
        }
        if self.current_final {
            return true;
        }
        let current_len = self.current.chars().count();
        let incoming_len = incoming.chars().count();
        current_len >= 8 && incoming_len.saturating_mul(2) < current_len
    }

    fn text(&self) -> String {
        merge_transcript(&self.confirmed, &self.current)
    }
}

fn merge_transcript(confirmed: &str, incoming: &str) -> String {
    if confirmed.is_empty() {
        return incoming.to_string();
    }
    if incoming.is_empty() {
        return confirmed.to_string();
    }
    if incoming.starts_with(confirmed) {
        return incoming.to_string();
    }
    if confirmed.starts_with(incoming) || confirmed.ends_with(incoming) {
        return confirmed.to_string();
    }

    let confirmed_chars = confirmed.chars().collect::<Vec<_>>();
    let incoming_chars = incoming.chars().collect::<Vec<_>>();
    let overlap = (1..=confirmed_chars.len().min(incoming_chars.len()))
        .rev()
        .find(|length| {
            confirmed_chars[confirmed_chars.len() - length..] == incoming_chars[..*length]
        })
        .unwrap_or(0);
    let mut merged = confirmed.to_string();
    if overlap == 0
        && confirmed_chars
            .last()
            .is_some_and(|value| value.is_ascii_alphanumeric())
        && incoming_chars
            .first()
            .is_some_and(|value| value.is_ascii_alphanumeric())
    {
        merged.push(' ');
    }
    merged.extend(incoming_chars.into_iter().skip(overlap));
    merged
}

#[derive(Clone, PartialEq, prost::Message)]
struct AsrRequest {
    #[prost(string, tag = "2")]
    token: String,
    #[prost(string, tag = "3")]
    service_name: String,
    #[prost(string, tag = "5")]
    method_name: String,
    #[prost(string, tag = "6")]
    payload: String,
    #[prost(bytes = "vec", tag = "7")]
    audio_data: Vec<u8>,
    #[prost(string, tag = "8")]
    request_id: String,
    #[prost(enumeration = "FrameState", tag = "9")]
    frame_state: i32,
}

#[derive(Clone, PartialEq, prost::Message)]
struct AsrResponse {
    #[prost(string, tag = "1")]
    request_id: String,
    #[prost(string, tag = "2")]
    task_id: String,
    #[prost(string, tag = "3")]
    service_name: String,
    #[prost(string, tag = "4")]
    message_type: String,
    #[prost(int32, tag = "5")]
    status_code: i32,
    #[prost(string, tag = "6")]
    status_message: String,
    #[prost(string, tag = "7")]
    result_json: String,
    #[prost(int32, tag = "9")]
    unknown_field_9: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
enum FrameState {
    Unspecified = 0,
    First = 1,
    Middle = 3,
    Last = 9,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceCredentials {
    device_id: String,
    install_id: String,
    cdid: String,
    openudid: String,
    clientudid: String,
    token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HandshakeOutcome {
    Ready,
    Rejected {
        stage: &'static str,
        log_id: Option<String>,
        message_type: String,
        status_code: i32,
        service_name: String,
        status_message: String,
    },
}

impl HandshakeOutcome {
    fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    fn is_service_discovery_failure(&self) -> bool {
        match self {
            Self::Rejected {
                status_code,
                status_message,
                ..
            } => is_service_discovery_status(*status_code, status_message),
            Self::Ready => false,
        }
    }
}

#[derive(Debug)]
struct AsrServiceError {
    message_type: String,
    status_code: i32,
    service_name: String,
    status_message: String,
}

impl fmt::Display for AsrServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "豆包 ASR 返回错误：type={} code={} service={} message={}",
            self.message_type, self.status_code, self.service_name, self.status_message
        )
    }
}

impl std::error::Error for AsrServiceError {}

async fn diagnose_doubao_service(credentials_path: &Path) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("创建网络客户端失败")?;
    let had_credentials = tokio::fs::try_exists(credentials_path)
        .await
        .with_context(|| format!("检查 ASR 凭据失败：{}", credentials_path.display()))?;
    let credentials = ensure_credentials(&client, credentials_path).await?;
    println!("asr.provider: doubao (zero-configuration default)");
    println!(
        "asr.credentials: {} (secrets redacted)",
        if had_credentials {
            "loaded"
        } else {
            "acquired"
        }
    );
    let refreshed_token = get_asr_token(&client, &credentials.device_id, &credentials.cdid).await?;
    println!(
        "asr.settings: ok (cached_token_current={})",
        refreshed_token == credentials.token
    );

    let cached = probe_handshake(&credentials).await?;
    print_handshake("cached", &cached);
    if cached.is_ready() {
        println!("asr.diagnosis: cached credentials and ASR protocol are working");
        return Ok(());
    }

    let mut refreshed_credentials = credentials.clone();
    refreshed_credentials.token = refreshed_token;
    let token_changed = refreshed_credentials.token != credentials.token;
    let refreshed = probe_handshake(&refreshed_credentials).await?;
    if token_changed {
        print_handshake("refreshed", &refreshed);
    } else {
        print_handshake("retry", &refreshed);
    }
    if refreshed.is_ready() {
        if token_changed {
            println!(
                "asr.diagnosis: cached token is stale; the current device identity still works"
            );
        } else {
            println!(
                "asr.diagnosis: the first ASR handshake failed transiently; unchanged credentials work on retry"
            );
        }
        return Ok(());
    }

    println!("asr.fresh_device: registering an in-memory diagnostic identity");
    let mut fresh_credentials = register_device(&client).await?;
    fresh_credentials.token = get_asr_token(
        &client,
        &fresh_credentials.device_id,
        &fresh_credentials.cdid,
    )
    .await?;
    let fresh = probe_handshake(&fresh_credentials).await?;
    print_handshake("fresh_device", &fresh);
    if fresh.is_ready() {
        if cached.is_service_discovery_failure() || refreshed.is_service_discovery_failure() {
            println!(
                "asr.diagnosis: cached device identity is rejected by service discovery; a newly registered identity works"
            );
        } else {
            println!(
                "asr.diagnosis: cached credentials are rejected; a newly registered identity works"
            );
        }
        println!("asr.credentials: unchanged (fresh diagnostic identity was not saved)");
        return Ok(());
    }

    bail!(
        "新设备身份仍无法完成 ASR 握手；问题位于上游服务、接口协议或当前网络，而非本地 IBus/音频集成"
    )
}

async fn transcribe_doubao_realtime<F>(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    credentials_path: &Path,
    mut on_event: F,
) -> Result<String>
where
    F: FnMut(AsrEvent) + Send,
{
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("创建网络客户端失败")?;
    let credentials = ensure_credentials(&client, credentials_path).await?;
    let mut buffered_audio = Vec::new();
    let mut source_finished = false;
    let mut transcript = TranscriptAccumulator::default();
    let first_attempt = transcribe_attempt(
        &mut audio_rx,
        &credentials,
        &mut buffered_audio,
        &mut source_finished,
        false,
        &mut transcript,
        &mut on_event,
    )
    .await;

    match first_attempt {
        Ok(text) => Ok(text),
        Err(error) if is_service_discovery_error(&error) => {
            tracing::warn!(
                error = %format_args!("{error:#}"),
                buffered_frames = buffered_audio.len(),
                "ASR service discovery failed; registering replacement credentials"
            );
            let replacement = acquire_fresh_credentials(&client)
                .await
                .context("重新获取 ASR 凭据失败")?;
            let text = transcribe_attempt(
                &mut audio_rx,
                &replacement,
                &mut buffered_audio,
                &mut source_finished,
                true,
                &mut transcript,
                &mut on_event,
            )
            .await
            .context("使用新 ASR 凭据重试失败")?;

            match save_credentials(credentials_path, &replacement).await {
                Ok(()) => tracing::info!("replaced ASR credentials after successful retry"),
                Err(save_error) => tracing::error!(
                    error = %format_args!("{save_error:#}"),
                    "ASR retry succeeded but replacement credentials could not be saved"
                ),
            }
            Ok(text)
        }
        Err(error) => Err(error),
    }
}

async fn transcribe_attempt<F>(
    audio_rx: &mut mpsc::Receiver<Vec<u8>>,
    credentials: &DeviceCredentials,
    buffered_audio: &mut Vec<Vec<u8>>,
    source_finished: &mut bool,
    replay_buffer: bool,
    transcript: &mut TranscriptAccumulator,
    on_event: &mut F,
) -> Result<String>
where
    F: FnMut(AsrEvent),
{
    let url = format!(
        "{WEBSOCKET_URL}?aid={AID}&device_id={}",
        credentials.device_id
    );
    let mut request = url
        .into_client_request()
        .context("创建 WebSocket 请求失败")?;
    request
        .headers_mut()
        .insert(USER_AGENT_HEADER, HeaderValue::from_static(USER_AGENT));
    request
        .headers_mut()
        .insert("proto-version", HeaderValue::from_static("v2"));
    request
        .headers_mut()
        .insert("x-custom-keepalive", HeaderValue::from_static("true"));

    let (mut socket, response) = connect_async(request).await.context("连接豆包 ASR 失败")?;
    let log_id = extract_log_id(response.headers());
    tracing::info!(
        x_tt_logid = log_id.as_deref().unwrap_or("missing"),
        "connected to Doubao ASR"
    );

    let result = async {
        let request_id = Uuid::new_v4().to_string();

        send_request(
            &mut socket,
            request_message(&request_id, &credentials.token, "StartTask"),
        )
        .await?;
        expect_message(&mut socket, "TaskStarted").await?;

        let session_payload = session_payload(&credentials.device_id);
        let mut start_session = request_message(&request_id, &credentials.token, "StartSession");
        start_session.payload = session_payload;
        send_request(&mut socket, start_session).await?;
        expect_message(&mut socket, "SessionStarted").await?;

        let mut encoder = Encoder::new(SAMPLE_RATE, Channels::Mono, Application::Audio)
            .context("初始化 Opus 编码器失败")?;
        let started_at = unix_time_ms();
        let mut frame_index = 0_u64;
        let mut finishing = false;

        if replay_buffer {
            for pcm in buffered_audio.iter() {
                send_pcm_frame(
                    &mut socket,
                    &mut encoder,
                    &request_id,
                    pcm,
                    frame_index,
                    started_at,
                )
                .await?;
                frame_index += 1;
            }
            tracing::info!(
                replayed_frames = frame_index,
                "replayed buffered audio with replacement credentials"
            );
        }

        if *source_finished {
            finish_audio_session(
                &mut socket,
                &mut encoder,
                &request_id,
                &credentials.token,
                frame_index,
                started_at,
            )
            .await?;
            finishing = true;
        }

        loop {
            tokio::select! {
                audio = audio_rx.recv(), if !finishing => {
                    match audio {
                        Some(pcm) => {
                            buffered_audio.push(pcm.clone());
                            send_pcm_frame(
                                &mut socket,
                                &mut encoder,
                                &request_id,
                                &pcm,
                                frame_index,
                                started_at,
                            ).await?;
                            frame_index += 1;
                        }
                        None => {
                            *source_finished = true;
                            finish_audio_session(
                                &mut socket,
                                &mut encoder,
                                &request_id,
                                &credentials.token,
                                frame_index,
                                started_at,
                            ).await?;
                            finishing = true;
                        }
                    }
                }
                incoming = socket.next() => {
                    let response = decode_socket_message(incoming).await?;
                    if matches!(response.message_type.as_str(), "TaskFailed" | "SessionFailed") {
                        return Err(asr_response_error(&response));
                    }
                    if response.message_type == "SessionFinished" {
                        break;
                    }
                    if let Some(event) = parse_transcript(&response)? {
                        on_event(transcript.apply(event));
                    }
                }
            }
        }

        Ok(transcript.text())
    }
    .await;

    if let Err(error) = &result {
        tracing::error!(
            x_tt_logid = log_id.as_deref().unwrap_or("missing"),
            error = %format_args!("{error:#}"),
            "Doubao ASR request failed"
        );
    }
    result
}

async fn send_pcm_frame<S>(
    socket: &mut S,
    encoder: &mut Encoder,
    request_id: &str,
    pcm: &[u8],
    frame_index: u64,
    started_at: u64,
) -> Result<()>
where
    S: SinkExt<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let frame_state = if frame_index == 0 {
        FrameState::First
    } else {
        FrameState::Middle
    };
    let encoded = encode_pcm(encoder, pcm)?;
    send_request(
        socket,
        audio_message(
            request_id,
            encoded,
            frame_state,
            started_at + frame_index * FRAME_DURATION_MS,
        ),
    )
    .await
}

async fn finish_audio_session<S>(
    socket: &mut S,
    encoder: &mut Encoder,
    request_id: &str,
    token: &str,
    frame_index: u64,
    started_at: u64,
) -> Result<()>
where
    S: SinkExt<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    if frame_index > 0 {
        let silence = vec![0_u8; PCM_FRAME_BYTES];
        let encoded = encode_pcm(encoder, &silence)?;
        send_request(
            socket,
            audio_message(
                request_id,
                encoded,
                FrameState::Last,
                started_at + frame_index * FRAME_DURATION_MS,
            ),
        )
        .await?;
    }
    send_request(socket, request_message(request_id, token, "FinishSession")).await
}

pub async fn check_audio_fixture(
    config: &AsrConfig,
    audio_path: &Path,
    credentials_path: &Path,
) -> Result<String> {
    let pcm = tokio::fs::read(audio_path)
        .await
        .with_context(|| format!("读取 ASR 测试音频失败：{}", audio_path.display()))?;
    if pcm.len() < PCM_FRAME_BYTES || pcm.len() % 2 != 0 {
        bail!("ASR 测试音频必须是非空的 16 kHz 单声道 16-bit little-endian PCM");
    }

    let (audio_tx, audio_rx) = mpsc::channel(8);
    let feeder = tokio::spawn(async move {
        for chunk in pcm.chunks(PCM_FRAME_BYTES) {
            let mut frame = chunk.to_vec();
            frame.resize(PCM_FRAME_BYTES, 0);
            if audio_tx.send(frame).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(FRAME_DURATION_MS)).await;
        }
    });

    let recognition = transcribe(config, audio_rx, credentials_path, |_| {});
    let result = timeout(Duration::from_secs(60), recognition)
        .await
        .context("ASR 测试音频识别超时")?;
    let _ = feeder.await;
    let text = result?;
    if text.trim().is_empty() {
        bail!("ASR 接口完成请求但没有返回识别文字");
    }
    Ok(text)
}

fn session_payload(device_id: &str) -> String {
    json!({
        "audio_info": {
            "channel": 1,
            "format": "speech_opus",
            "sample_rate": SAMPLE_RATE
        },
        "enable_punctuation": true,
        "enable_speech_rejection": false,
        "extra": {
            "app_name": "com.android.chrome",
            "cell_compress_rate": 8,
            "did": device_id,
            "enable_asr_threepass": true,
            "enable_asr_twopass": true,
            "input_mode": "tool"
        }
    })
    .to_string()
}

fn extract_log_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-tt-logid")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

async fn probe_handshake(credentials: &DeviceCredentials) -> Result<HandshakeOutcome> {
    let url = format!(
        "{WEBSOCKET_URL}?aid={AID}&device_id={}",
        credentials.device_id
    );
    let mut request = url
        .into_client_request()
        .context("创建 ASR 诊断 WebSocket 请求失败")?;
    request
        .headers_mut()
        .insert(USER_AGENT_HEADER, HeaderValue::from_static(USER_AGENT));
    request
        .headers_mut()
        .insert("proto-version", HeaderValue::from_static("v2"));
    request
        .headers_mut()
        .insert("x-custom-keepalive", HeaderValue::from_static("true"));

    let (mut socket, response) = timeout(Duration::from_secs(15), connect_async(request))
        .await
        .context("连接豆包 ASR 诊断超时")?
        .context("连接豆包 ASR 诊断失败")?;
    let log_id = extract_log_id(response.headers());
    tracing::info!(
        x_tt_logid = log_id.as_deref().unwrap_or("missing"),
        "connected to Doubao ASR diagnostic"
    );

    let result = async {
        let request_id = Uuid::new_v4().to_string();
        send_request(
            &mut socket,
            request_message(&request_id, &credentials.token, "StartTask"),
        )
        .await?;
        let task_response = receive_probe_response(&mut socket).await?;
        if task_response.message_type != "TaskStarted" {
            let _ = socket.send(WsMessage::Close(None)).await;
            return Ok(rejected_outcome("StartTask", task_response, log_id.clone()));
        }

        let mut start_session = request_message(&request_id, &credentials.token, "StartSession");
        start_session.payload = session_payload(&credentials.device_id);
        send_request(&mut socket, start_session).await?;
        let session_response = receive_probe_response(&mut socket).await?;
        let outcome = if session_response.message_type == "SessionStarted" {
            let _ = send_request(
                &mut socket,
                request_message(&request_id, &credentials.token, "FinishSession"),
            )
            .await;
            let _ = timeout(Duration::from_secs(5), async {
                loop {
                    let response = receive_probe_response(&mut socket).await?;
                    if response.message_type == "SessionFinished" {
                        return Ok::<(), anyhow::Error>(());
                    }
                }
            })
            .await;
            HandshakeOutcome::Ready
        } else {
            rejected_outcome("StartSession", session_response, log_id.clone())
        };
        let _ = socket.send(WsMessage::Close(None)).await;
        Ok(outcome)
    }
    .await;

    match &result {
        Ok(HandshakeOutcome::Rejected {
            stage,
            log_id,
            status_code,
            status_message,
            ..
        }) => tracing::warn!(
            x_tt_logid = log_id.as_deref().unwrap_or("missing"),
            stage,
            status_code,
            status_message,
            "Doubao ASR diagnostic rejected"
        ),
        Err(error) => tracing::error!(
            x_tt_logid = log_id.as_deref().unwrap_or("missing"),
            error = %format_args!("{error:#}"),
            "Doubao ASR diagnostic failed"
        ),
        Ok(HandshakeOutcome::Ready) => {}
    }
    result
}

async fn receive_probe_response<S>(socket: &mut S) -> Result<AsrResponse>
where
    S: StreamExt<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    timeout(Duration::from_secs(15), async {
        loop {
            let message = socket
                .next()
                .await
                .ok_or_else(|| anyhow!("豆包 ASR 诊断连接已关闭"))?
                .context("读取豆包 ASR 诊断响应失败")?;
            match message {
                WsMessage::Binary(bytes) => {
                    return AsrResponse::decode(bytes.as_slice()).context("解析 ASR 诊断响应失败");
                }
                WsMessage::Close(frame) => bail!("豆包 ASR 诊断关闭连接：{frame:?}"),
                WsMessage::Ping(_)
                | WsMessage::Pong(_)
                | WsMessage::Text(_)
                | WsMessage::Frame(_) => {}
            }
        }
    })
    .await
    .context("等待豆包 ASR 诊断响应超时")?
}

fn rejected_outcome(
    stage: &'static str,
    response: AsrResponse,
    log_id: Option<String>,
) -> HandshakeOutcome {
    HandshakeOutcome::Rejected {
        stage,
        log_id,
        message_type: response.message_type,
        status_code: response.status_code,
        service_name: response.service_name,
        status_message: response.status_message,
    }
}

fn print_handshake(label: &str, outcome: &HandshakeOutcome) {
    match outcome {
        HandshakeOutcome::Ready => println!("asr.{label}: ready (StartTask + StartSession)"),
        HandshakeOutcome::Rejected {
            stage,
            log_id,
            message_type,
            status_code,
            service_name,
            status_message,
        } => println!(
            "asr.{label}: rejected stage={stage} logid={} type={message_type} code={status_code} service={service_name:?} message={status_message:?}",
            log_id.as_deref().unwrap_or("missing")
        ),
    }
}

fn is_service_discovery_status(status_code: i32, status_message: &str) -> bool {
    status_code == 50_700_000
        || status_message
            .to_ascii_lowercase()
            .contains("service discovery failure")
}

fn asr_response_error(response: &AsrResponse) -> anyhow::Error {
    AsrServiceError {
        message_type: response.message_type.clone(),
        status_code: response.status_code,
        service_name: response.service_name.clone(),
        status_message: response.status_message.clone(),
    }
    .into()
}

fn is_service_discovery_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<AsrServiceError>()
            .is_some_and(|service_error| {
                is_service_discovery_status(
                    service_error.status_code,
                    &service_error.status_message,
                )
            })
    })
}

async fn decode_socket_message(
    incoming: Option<Result<WsMessage, tokio_tungstenite::tungstenite::Error>>,
) -> Result<AsrResponse> {
    let message = incoming
        .ok_or_else(|| anyhow!("豆包 ASR 连接已关闭"))?
        .context("读取豆包 ASR 响应失败")?;
    match message {
        WsMessage::Binary(bytes) => {
            AsrResponse::decode(bytes.as_slice()).context("解析 ASR 响应失败")
        }
        WsMessage::Close(frame) => bail!("豆包 ASR 关闭连接：{frame:?}"),
        _ => Ok(AsrResponse::default()),
    }
}

async fn expect_message<S>(socket: &mut S, expected: &str) -> Result<()>
where
    S: StreamExt<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let response = decode_socket_message(socket.next().await).await?;
    if matches!(
        response.message_type.as_str(),
        "TaskFailed" | "SessionFailed"
    ) {
        return Err(asr_response_error(&response));
    }
    if response.message_type != expected {
        bail!("豆包 ASR 返回了意外消息：{}", response.message_type);
    }
    Ok(())
}

async fn send_request<S>(socket: &mut S, request: AsrRequest) -> Result<()>
where
    S: SinkExt<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let bytes = request.encode_to_vec();
    socket
        .send(WsMessage::Binary(bytes))
        .await
        .context("发送 ASR 请求失败")
}

fn request_message(request_id: &str, token: &str, method: &str) -> AsrRequest {
    AsrRequest {
        token: token.to_string(),
        service_name: "ASR".to_string(),
        method_name: method.to_string(),
        payload: String::new(),
        audio_data: Vec::new(),
        request_id: request_id.to_string(),
        frame_state: FrameState::Unspecified as i32,
    }
}

fn audio_message(
    request_id: &str,
    audio_data: Vec<u8>,
    frame_state: FrameState,
    timestamp_ms: u64,
) -> AsrRequest {
    AsrRequest {
        token: String::new(),
        service_name: "ASR".to_string(),
        method_name: "TaskRequest".to_string(),
        payload: json!({ "extra": {}, "timestamp_ms": timestamp_ms }).to_string(),
        audio_data,
        request_id: request_id.to_string(),
        frame_state: frame_state as i32,
    }
}

fn encode_pcm(encoder: &mut Encoder, pcm: &[u8]) -> Result<Vec<u8>> {
    let mut samples: Vec<i16> = pcm
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    samples.resize(320, 0);
    samples.truncate(320);
    let mut output = vec![0_u8; 4_000];
    let encoded = encoder
        .encode(&samples, &mut output)
        .context("Opus 编码失败")?;
    output.truncate(encoded);
    Ok(output)
}

fn parse_transcript(response: &AsrResponse) -> Result<Option<AsrEvent>> {
    if response.result_json.is_empty() {
        return Ok(None);
    }
    let payload: Value = serde_json::from_str(&response.result_json).context("解析识别结果失败")?;
    let extra = payload.get("extra").unwrap_or(&Value::Null);
    if extra.get("vad_start").and_then(Value::as_bool) == Some(true) {
        return Ok(Some(AsrEvent::SpeechStarted));
    }
    let Some(results) = payload.get("results").and_then(Value::as_array) else {
        return Ok(None);
    };

    let mut text = String::new();
    let mut is_interim = true;
    let mut vad_finished = false;
    let mut nonstream_result = false;
    for result in results {
        if let Some(value) = result.get("text").and_then(Value::as_str)
            && value.chars().count() > text.chars().count()
        {
            text = value.to_string();
        }
        if result.get("is_interim").and_then(Value::as_bool) == Some(false) {
            is_interim = false;
        }
        if result.get("is_vad_finished").and_then(Value::as_bool) == Some(true) {
            vad_finished = true;
        }
        if result
            .get("extra")
            .and_then(|value| value.get("nonstream_result"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            nonstream_result = true;
        }
    }
    if text.is_empty() {
        return Ok(None);
    }
    if nonstream_result || (!is_interim && vad_finished) {
        Ok(Some(AsrEvent::Final(text)))
    } else {
        Ok(Some(AsrEvent::Partial(text)))
    }
}

async fn ensure_credentials(client: &reqwest::Client, path: &Path) -> Result<DeviceCredentials> {
    if let Ok(content) = tokio::fs::read_to_string(path).await
        && let Ok(credentials) = serde_json::from_str::<DeviceCredentials>(&content)
        && !credentials.device_id.is_empty()
        && !credentials.cdid.is_empty()
        && !credentials.token.is_empty()
    {
        return Ok(credentials);
    }

    let credentials = acquire_fresh_credentials(client).await?;
    save_credentials(path, &credentials).await?;
    Ok(credentials)
}

async fn acquire_fresh_credentials(client: &reqwest::Client) -> Result<DeviceCredentials> {
    let mut credentials = register_device(client).await?;
    credentials.token = get_asr_token(client, &credentials.device_id, &credentials.cdid).await?;
    Ok(credentials)
}

async fn save_credentials(path: &Path, credentials: &DeviceCredentials) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    tokio::fs::create_dir_all(parent)
        .await
        .context("创建凭据目录失败")?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("credentials.json");
    let temporary_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));
    let content = format!("{}\n", serde_json::to_string_pretty(credentials)?);

    let result: Result<()> = async {
        tokio::fs::write(&temporary_path, content)
            .await
            .context("写入临时 ASR 凭据失败")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&temporary_path, std::fs::Permissions::from_mode(0o600))
                .await
                .context("设置临时 ASR 凭据权限失败")?;
        }

        tokio::fs::rename(&temporary_path, path)
            .await
            .context("原子替换 ASR 凭据失败")?;
        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
    }
    result
}

async fn register_device(client: &reqwest::Client) -> Result<DeviceCredentials> {
    let cdid = Uuid::new_v4().to_string();
    let clientudid = Uuid::new_v4().to_string();
    let openudid = Uuid::new_v4().simple().to_string()[..16].to_string();
    let now = unix_time_ms();
    let device_locale = DeviceLocale::from_preferences(&SystemPreferences::current());
    let header = json!({
        "device_id": 0,
        "install_id": 0,
        "aid": AID,
        "app_name": "oime",
        "version_code": 100102018,
        "version_name": "1.1.2",
        "manifest_version_code": 100102018,
        "update_version_code": 100102018,
        "channel": "official",
        "package": "com.bytedance.android.doubaoime",
        "device_platform": "android",
        "os": "android",
        "os_api": "34",
        "os_version": "16",
        "device_type": "Pixel 7 Pro",
        "device_brand": "google",
        "device_model": "Pixel 7 Pro",
        "resolution": "1080*2400",
        "dpi": "420",
        "language": device_locale.language.clone(),
        "timezone": device_locale.timezone_hours,
        "access": "wifi",
        "rom": "UP1A.231005.007",
        "rom_version": "UP1A.231005.007",
        "openudid": openudid,
        "clientudid": clientudid,
        "cdid": cdid,
        "region": "CN",
        "tz_name": device_locale.time_zone.clone(),
        "tz_offset": device_locale.utc_offset_seconds,
        "sim_region": "cn",
        "carrier_region": "cn",
        "cpu_abi": "arm64-v8a",
        "build_serial": "unknown",
        "not_request_sender": 0,
        "sig_hash": "",
        "google_aid": "",
        "mc": "",
        "serial_number": ""
    });
    let body = json!({ "magic_tag": "ss_app_log", "header": header, "_gen_time": now });
    let query = vec![
        ("device_platform", "android".to_string()),
        ("os", "android".to_string()),
        ("ssmix", "a".to_string()),
        ("_rticket", now.to_string()),
        ("cdid", cdid.clone()),
        ("channel", "official".to_string()),
        ("aid", AID.to_string()),
        ("app_name", "oime".to_string()),
        ("version_code", "100102018".to_string()),
        ("version_name", "1.1.2".to_string()),
        ("manifest_version_code", "100102018".to_string()),
        ("update_version_code", "100102018".to_string()),
        ("resolution", "1080*2400".to_string()),
        ("dpi", "420".to_string()),
        ("device_type", "Pixel 7 Pro".to_string()),
        ("device_brand", "google".to_string()),
        ("language", device_locale.language.clone()),
        ("os_api", "34".to_string()),
        ("os_version", "16".to_string()),
        ("ac", "wifi".to_string()),
    ];
    let response: Value = client
        .post(REGISTER_URL)
        .query(&query)
        .json(&body)
        .send()
        .await
        .context("注册豆包虚拟设备失败")?
        .error_for_status()
        .context("豆包设备注册返回错误")?
        .json()
        .await
        .context("解析豆包设备凭据失败")?;
    let device_id = json_identifier(&response, "device_id", "device_id_str")?;
    let install_id = json_identifier(&response, "install_id", "install_id_str")?;
    Ok(DeviceCredentials {
        device_id,
        install_id,
        cdid,
        openudid,
        clientudid,
        token: String::new(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeviceLocale {
    language: String,
    time_zone: String,
    timezone_hours: i32,
    utc_offset_seconds: i32,
}

impl DeviceLocale {
    fn from_preferences(preferences: &SystemPreferences) -> Self {
        let utc_offset_seconds = preferences.utc_offset_seconds();
        Self {
            language: preferences.speech_language().to_string(),
            time_zone: preferences.time_zone().unwrap_or("UTC").to_string(),
            timezone_hours: utc_offset_seconds / 3_600,
            utc_offset_seconds,
        }
    }
}

async fn get_asr_token(client: &reqwest::Client, device_id: &str, cdid: &str) -> Result<String> {
    let query = vec![
        ("device_platform", "android".to_string()),
        ("os", "android".to_string()),
        ("ssmix", "a".to_string()),
        ("_rticket", unix_time_ms().to_string()),
        ("cdid", cdid.to_string()),
        ("channel", "official".to_string()),
        ("aid", AID.to_string()),
        ("app_name", "oime".to_string()),
        ("version_code", "100102018".to_string()),
        ("version_name", "1.1.2".to_string()),
        ("device_id", device_id.to_string()),
    ];
    let stub = format!("{:X}", md5::compute("body=null"));
    let response: Value = client
        .post(SETTINGS_URL)
        .query(&query)
        .header("x-ss-stub", stub)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body("body=null")
        .send()
        .await
        .context("获取豆包 ASR Token 失败")?
        .error_for_status()
        .context("豆包设置接口返回错误")?
        .json()
        .await
        .context("解析豆包 ASR Token 失败")?;
    response
        .pointer("/data/settings/asr_config/app_key")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("豆包设置响应中没有 ASR Token"))
}

fn json_identifier(value: &Value, number_key: &str, string_key: &str) -> Result<String> {
    if let Some(value) = value.get(string_key).and_then(Value::as_str) {
        return Ok(value.to_string());
    }
    if let Some(value) = value.get(number_key).and_then(Value::as_i64) {
        return Ok(value.to_string());
    }
    bail!("豆包设备注册响应缺少 {number_key}")
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubao_registration_uses_system_language_and_time_zone() {
        let preferences =
            SystemPreferences::from_parts(Some("en_US.UTF-8"), Some("Asia/Shanghai"), 28_800);
        let locale = DeviceLocale::from_preferences(&preferences);
        assert_eq!(locale.language, "zh");
        assert_eq!(locale.time_zone, "Asia/Shanghai");
        assert_eq!(locale.timezone_hours, 8);
        assert_eq!(locale.utc_offset_seconds, 28_800);
    }

    #[test]
    fn parses_partial_and_final_results() {
        let mut response = AsrResponse {
            result_json: json!({
                "results": [{ "text": "你好", "is_interim": true }],
                "extra": {}
            })
            .to_string(),
            ..Default::default()
        };
        assert!(matches!(
            parse_transcript(&response).unwrap(),
            Some(AsrEvent::Partial(text)) if text == "你好"
        ));

        response.result_json = json!({
            "results": [{
                "text": "你好。",
                "is_interim": false,
                "is_vad_finished": true
            }],
            "extra": {}
        })
        .to_string();
        assert!(matches!(
            parse_transcript(&response).unwrap(),
            Some(AsrEvent::Final(text)) if text == "你好。"
        ));
    }

    #[test]
    fn parser_prefers_the_full_snapshot_from_multiple_results() {
        let response = AsrResponse {
            result_json: json!({
                "results": [
                    {
                        "text": "你好呀。我觉得今天的天气不错。",
                        "is_interim": true
                    },
                    {
                        "text": "我觉得今天的天气不错。",
                        "is_interim": true
                    }
                ],
                "extra": {}
            })
            .to_string(),
            ..Default::default()
        };
        assert!(matches!(
            parse_transcript(&response).unwrap(),
            Some(AsrEvent::Partial(text)) if text == "你好呀。我觉得今天的天气不错。"
        ));
    }

    #[test]
    fn parser_keeps_the_full_snapshot_for_a_final_result() {
        let response = AsrResponse {
            result_json: json!({
                "results": [
                    {
                        "text": "现在是独立 demo 验证。",
                        "is_interim": false,
                        "is_vad_finished": true
                    },
                    {
                        "text": "现在是独立demo验证",
                        "is_interim": false,
                        "extra": { "nonstream_result": true }
                    }
                ],
                "extra": {}
            })
            .to_string(),
            ..Default::default()
        };
        assert!(matches!(
            parse_transcript(&response).unwrap(),
            Some(AsrEvent::Final(text)) if text == "现在是独立 demo 验证。"
        ));
    }

    #[test]
    fn preserves_confirmed_text_across_vad_segments() {
        let mut transcript = TranscriptAccumulator::default();
        assert!(matches!(
            transcript.apply(AsrEvent::SpeechStarted),
            AsrEvent::SpeechStarted
        ));
        assert!(matches!(
            transcript.apply(AsrEvent::Partial("这是第一段".to_string())),
            AsrEvent::Partial(text) if text == "这是第一段"
        ));
        assert!(matches!(
            transcript.apply(AsrEvent::Final("这是第一段。".to_string())),
            AsrEvent::Final(text) if text == "这是第一段。"
        ));
        assert!(matches!(
            transcript.apply(AsrEvent::SpeechStarted),
            AsrEvent::Partial(text) if text == "这是第一段。"
        ));
        assert!(matches!(
            transcript.apply(AsrEvent::Partial("这是第二段".to_string())),
            AsrEvent::Partial(text) if text == "这是第一段。这是第二段"
        ));
        assert!(matches!(
            transcript.apply(AsrEvent::Final("这是第二段。".to_string())),
            AsrEvent::Final(text) if text == "这是第一段。这是第二段。"
        ));
        assert_eq!(transcript.text(), "这是第一段。这是第二段。");
    }

    #[test]
    fn accepts_cumulative_snapshots_without_duplicating_text() {
        let mut transcript = TranscriptAccumulator::default();
        transcript.apply(AsrEvent::Final("第一句。".to_string()));
        assert!(matches!(
            transcript.apply(AsrEvent::Partial("第一句。第二句".to_string())),
            AsrEvent::Partial(text) if text == "第一句。第二句"
        ));
        assert!(matches!(
            transcript.apply(AsrEvent::Final("第一句。第二句。".to_string())),
            AsrEvent::Final(text) if text == "第一句。第二句。"
        ));
    }

    #[test]
    fn replaces_revised_final_snapshots_instead_of_appending_them() {
        let mut transcript = TranscriptAccumulator::default();
        transcript.apply(AsrEvent::Final("我觉得今天不错。".to_string()));
        assert!(matches!(
            transcript.apply(AsrEvent::Final("我觉得今天天气不错。".to_string())),
            AsrEvent::Final(text) if text == "我觉得今天天气不错。"
        ));
        assert_eq!(transcript.text(), "我觉得今天天气不错。");
    }

    #[test]
    fn detects_a_large_segment_reset_without_a_vad_event() {
        let mut transcript = TranscriptAccumulator::default();
        transcript.apply(AsrEvent::Partial(
            "这是没有显式分段事件的第一段长句。".to_string(),
        ));
        assert!(matches!(
            transcript.apply(AsrEvent::Partial("第二段".to_string())),
            AsrEvent::Partial(text) if text == "这是没有显式分段事件的第一段长句。第二段"
        ));
    }

    #[test]
    fn merges_overlapping_segments_and_separates_english_words() {
        assert_eq!(
            merge_transcript("今天很好", "很好，我们继续"),
            "今天很好，我们继续"
        );
        assert_eq!(merge_transcript("hello", "world"), "hello world");
    }

    #[test]
    fn protobuf_field_numbers_match_protocol() {
        let message = request_message("request", "token", "StartTask");
        let encoded = message.encode_to_vec();
        let decoded = AsrRequest::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded.token, "token");
        assert_eq!(decoded.method_name, "StartTask");
        assert_eq!(decoded.request_id, "request");
    }

    #[test]
    fn classifies_service_discovery_failure_without_credentials() {
        let outcome = rejected_outcome(
            "StartSession",
            AsrResponse {
                service_name: "ASR".to_string(),
                message_type: "SessionFailed".to_string(),
                status_code: 50_700_000,
                status_message: "service discovery failure".to_string(),
                ..Default::default()
            },
            Some("test-logid".to_string()),
        );

        assert!(!outcome.is_ready());
        assert!(outcome.is_service_discovery_failure());
        let debug = format!("{outcome:?}");
        assert!(!debug.contains("token"));
        assert!(!debug.contains("device_id"));
        assert!(debug.contains("test-logid"));
    }

    #[test]
    fn classifies_backend_service_discovery_errors_for_recovery() {
        let by_code = asr_response_error(&AsrResponse {
            message_type: "SessionFailed".to_string(),
            status_code: 50_700_000,
            service_name: "ws".to_string(),
            status_message: "backend unavailable".to_string(),
            ..Default::default()
        });
        assert!(is_service_discovery_error(&by_code));
        assert!(by_code.to_string().contains("code=50700000"));

        let by_message = asr_response_error(&AsrResponse {
            message_type: "SessionFailed".to_string(),
            status_code: 2,
            service_name: "ws".to_string(),
            status_message: "read backend response: service discovery failure".to_string(),
            ..Default::default()
        });
        assert!(is_service_discovery_error(&by_message));

        let unrelated = asr_response_error(&AsrResponse {
            message_type: "SessionFailed".to_string(),
            status_code: 40_200_011,
            service_name: "ws".to_string(),
            status_message: "concurrent request limit".to_string(),
            ..Default::default()
        });
        assert!(!is_service_discovery_error(&unrelated));
    }

    #[tokio::test]
    async fn saves_replacement_credentials_atomically() {
        let directory = std::env::temp_dir().join(format!("typeless-asr-{}", Uuid::new_v4()));
        let path = directory.join("credentials.json");
        let credentials = DeviceCredentials {
            device_id: "device".to_string(),
            install_id: "install".to_string(),
            cdid: "cdid".to_string(),
            openudid: "open".to_string(),
            clientudid: "client".to_string(),
            token: "token".to_string(),
        };

        save_credentials(&path, &credentials).await.unwrap();
        let saved: DeviceCredentials =
            serde_json::from_str(&tokio::fs::read_to_string(&path).await.unwrap()).unwrap();
        assert_eq!(saved.device_id, credentials.device_id);
        assert_eq!(saved.token, credentials.token);

        let mut entries = tokio::fs::read_dir(&directory).await.unwrap();
        let mut names = Vec::new();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            names.push(entry.file_name());
        }
        assert_eq!(names, vec![std::ffi::OsString::from("credentials.json")]);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = tokio::fs::metadata(&path)
                .await
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }

        tokio::fs::remove_dir_all(directory).await.unwrap();
    }

    #[test]
    fn extracts_case_insensitive_log_id_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Tt-Logid", HeaderValue::from_static("trace-123"));
        assert_eq!(extract_log_id(&headers).as_deref(), Some("trace-123"));
    }

    #[test]
    fn session_payload_uses_the_probed_device_identity() {
        let payload: Value = serde_json::from_str(&session_payload("device-under-test")).unwrap();
        assert_eq!(payload["extra"]["did"], "device-under-test");
        assert_eq!(payload["audio_info"]["format"], "speech_opus");
        assert_eq!(payload["audio_info"]["sample_rate"], SAMPLE_RATE);
    }

    #[test]
    fn provider_factory_keeps_doubao_as_the_default() {
        let provider =
            configured_provider(&AsrConfig::default(), Path::new("credentials.json")).unwrap();
        assert_eq!(provider.kind(), AsrProviderKind::Doubao);
    }

    #[test]
    fn provider_factory_routes_every_cloud_protocol() {
        for kind in AsrProviderKind::ALL {
            let mut config = AsrConfig {
                provider: kind,
                api_key: Some("api-key".to_string()),
                ..AsrConfig::default()
            };
            if kind == AsrProviderKind::Doubao {
                config.api_key = None;
            }
            let provider = configured_provider(&config, Path::new("credentials.json")).unwrap();
            assert_eq!(provider.kind(), kind);
        }
    }
}
