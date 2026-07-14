use anyhow::{Context, Result, anyhow, bail};
use futures_util::{SinkExt, StreamExt};
use http::header::{HeaderValue, USER_AGENT as USER_AGENT_HEADER};
use opus::{Application, Channels, Encoder};
use prost::Message as ProstMessage;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use uuid::Uuid;

const REGISTER_URL: &str = "https://log.snssdk.com/service/2/device_register/";
const SETTINGS_URL: &str = "https://is.snssdk.com/service/settings/v3/";
const WEBSOCKET_URL: &str = "wss://frontier-audio-ime-ws.doubao.com/ocean/api/v1/ws";
const AID: u32 = 401_734;
const USER_AGENT: &str = "com.bytedance.android.doubaoime/100102018 (Linux; U; Android 16; en_US; Pixel 7 Pro; Build/BP2A.250605.031.A2; Cronet/TTNetVersion:94cf429a 2025-11-17 QuicVersion:1f89f732 2025-05-08)";
const SAMPLE_RATE: u32 = 16_000;
const FRAME_DURATION_MS: u64 = 20;

#[derive(Debug, Clone)]
pub enum AsrEvent {
    SpeechStarted,
    Partial(String),
    Final(String),
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

pub async fn transcribe_realtime<F>(
    mut audio_rx: mpsc::Receiver<Vec<u8>>,
    credentials_path: &Path,
    mut on_event: F,
) -> Result<String>
where
    F: FnMut(AsrEvent),
{
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .context("创建网络客户端失败")?;
    let credentials = ensure_credentials(&client, credentials_path).await?;
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

    let (mut socket, _) = connect_async(request).await.context("连接豆包 ASR 失败")?;
    let request_id = Uuid::new_v4().to_string();

    send_request(
        &mut socket,
        request_message(&request_id, &credentials.token, "StartTask"),
    )
    .await?;
    expect_message(&mut socket, "TaskStarted").await?;

    let session_payload = json!({
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
            "did": credentials.device_id,
            "enable_asr_threepass": true,
            "enable_asr_twopass": true,
            "input_mode": "tool"
        }
    })
    .to_string();
    let mut start_session = request_message(&request_id, &credentials.token, "StartSession");
    start_session.payload = session_payload;
    send_request(&mut socket, start_session).await?;
    expect_message(&mut socket, "SessionStarted").await?;

    let mut encoder = Encoder::new(SAMPLE_RATE, Channels::Mono, Application::Audio)
        .context("初始化 Opus 编码器失败")?;
    let started_at = unix_time_ms();
    let mut frame_index = 0_u64;
    let mut final_text = String::new();
    let mut finishing = false;

    loop {
        tokio::select! {
            audio = audio_rx.recv(), if !finishing => {
                match audio {
                    Some(pcm) => {
                        let frame_state = if frame_index == 0 { FrameState::First } else { FrameState::Middle };
                        let encoded = encode_pcm(&mut encoder, &pcm)?;
                        let message = audio_message(
                            &request_id,
                            encoded,
                            frame_state,
                            started_at + frame_index * FRAME_DURATION_MS,
                        );
                        send_request(&mut socket, message).await?;
                        frame_index += 1;
                    }
                    None => {
                        if frame_index > 0 {
                            let silence = vec![0_u8; 640];
                            let encoded = encode_pcm(&mut encoder, &silence)?;
                            let message = audio_message(
                                &request_id,
                                encoded,
                                FrameState::Last,
                                started_at + frame_index * FRAME_DURATION_MS,
                            );
                            send_request(&mut socket, message).await?;
                        }
                        send_request(
                            &mut socket,
                            request_message(&request_id, &credentials.token, "FinishSession"),
                        ).await?;
                        finishing = true;
                    }
                }
            }
            incoming = socket.next() => {
                let response = decode_socket_message(incoming).await?;
                if matches!(response.message_type.as_str(), "TaskFailed" | "SessionFailed") {
                    bail!("豆包 ASR 返回错误：{}", response.status_message);
                }
                if response.message_type == "SessionFinished" {
                    break;
                }
                if let Some(event) = parse_transcript(&response)? {
                    if let AsrEvent::Final(text) = &event {
                        final_text.clone_from(text);
                    }
                    on_event(event);
                }
            }
        }
    }

    Ok(final_text)
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
        bail!("豆包 ASR 初始化失败：{}", response.status_message);
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
        if let Some(value) = result.get("text").and_then(Value::as_str) {
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
        && !credentials.token.is_empty()
    {
        return Ok(credentials);
    }

    let mut credentials = register_device(client).await?;
    credentials.token = get_asr_token(client, &credentials.device_id, &credentials.cdid).await?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .context("创建凭据目录失败")?;
    }
    tokio::fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&credentials)?),
    )
    .await
    .context("保存 ASR 凭据失败")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .await
            .context("设置 ASR 凭据权限失败")?;
    }
    Ok(credentials)
}

async fn register_device(client: &reqwest::Client) -> Result<DeviceCredentials> {
    let cdid = Uuid::new_v4().to_string();
    let clientudid = Uuid::new_v4().to_string();
    let openudid = Uuid::new_v4().simple().to_string()[..16].to_string();
    let now = unix_time_ms();
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
        "language": "zh",
        "timezone": 8,
        "access": "wifi",
        "rom": "UP1A.231005.007",
        "rom_version": "UP1A.231005.007",
        "openudid": openudid,
        "clientudid": clientudid,
        "cdid": cdid,
        "region": "CN",
        "tz_name": "Asia/Shanghai",
        "tz_offset": 28800,
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
        ("language", "zh".to_string()),
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
    fn protobuf_field_numbers_match_protocol() {
        let message = request_message("request", "token", "StartTask");
        let encoded = message.encode_to_vec();
        let decoded = AsrRequest::decode(encoded.as_slice()).unwrap();
        assert_eq!(decoded.token, "token");
        assert_eq!(decoded.method_name, "StartTask");
        assert_eq!(decoded.request_id, "request");
    }
}
