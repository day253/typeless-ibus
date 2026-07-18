use crate::asr::{self, AsrEvent};
use crate::audio::AudioCaptureHandle;
use crate::config::{AsrConfig, ConfigStore, TriggerMode};
use crate::i18n;
use crate::properties::{self, ConfigAction};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::Instrument;
use uuid::Uuid;
use xkeysym::{Keysym, key};
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{Structure, Value};
use zbus::{fdo, interface};

const RELEASE_MASK: u32 = 1 << 30;
const WAITING_PREEDIT_ENGLISH: &str = "Listening…";
const WAITING_PREEDIT_CHINESE: &str = "聆听中…";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Recording,
    Processing,
}

struct SessionState {
    phase: Phase,
    generation: u64,
    capture: Option<AudioCaptureHandle>,
    log_context: Option<RecognitionLogContext>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            generation: 0,
            capture: None,
            log_context: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CursorLocation {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct IbusContext {
    focused: bool,
    client: Option<String>,
    object_path: Option<String>,
    cursor: CursorLocation,
    capabilities: u32,
}

#[derive(Debug, Clone)]
struct RecognitionLogContext {
    session_id: String,
    started_at: Instant,
    engine_path: String,
    provider: String,
    ibus: IbusContext,
    content_type: (u32, u32),
}

impl RecognitionLogContext {
    fn new(
        engine_path: &str,
        provider: &str,
        ibus: &IbusContext,
        content_type: (u32, u32),
    ) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            started_at: Instant::now(),
            engine_path: engine_path.to_string(),
            provider: provider.to_string(),
            ibus: ibus.clone(),
            content_type,
        }
    }

    fn log_started(&self, trigger: &str, mode: TriggerMode) {
        tracing::info!(
            schema_version = 1,
            event = "voice_session.started",
            session_id = %self.session_id,
            engine_path = %self.engine_path,
            provider = %self.provider,
            trigger,
            trigger_mode = ?mode,
            ibus_client = self.ibus.client.as_deref().unwrap_or("unknown"),
            input_context = self.ibus.object_path.as_deref().unwrap_or("unknown"),
            cursor_x = self.ibus.cursor.x,
            cursor_y = self.ibus.cursor.y,
            cursor_width = self.ibus.cursor.width,
            cursor_height = self.ibus.cursor.height,
            input_purpose = input_purpose_name(self.content_type.0),
            input_purpose_code = self.content_type.0,
            input_hints = self.content_type.1,
            client_capabilities = self.ibus.capabilities,
            "voice session started"
        );
    }

    fn log_finished(&self, status: &str, transcript: Option<&str>, error: Option<&str>) {
        let transcript = transcript.unwrap_or_default();
        tracing::info!(
            schema_version = 1,
            event = "voice_session.finished",
            session_id = %self.session_id,
            engine_path = %self.engine_path,
            provider = %self.provider,
            status,
            duration_ms = elapsed_millis(self.started_at),
            ibus_client = self.ibus.client.as_deref().unwrap_or("unknown"),
            input_context = self.ibus.object_path.as_deref().unwrap_or("unknown"),
            cursor_x = self.ibus.cursor.x,
            cursor_y = self.ibus.cursor.y,
            cursor_width = self.ibus.cursor.width,
            cursor_height = self.ibus.cursor.height,
            input_purpose = input_purpose_name(self.content_type.0),
            input_purpose_code = self.content_type.0,
            input_hints = self.content_type.1,
            client_capabilities = self.ibus.capabilities,
            transcript,
            transcript_characters = transcript.chars().count(),
            error = error.unwrap_or_default(),
            "voice session finished"
        );
    }
}

pub struct VoiceEngine {
    engine_path: String,
    config: ConfigStore,
    credentials_path: PathBuf,
    session: Arc<Mutex<SessionState>>,
    ibus_context: IbusContext,
    content_type: (u32, u32),
}

impl VoiceEngine {
    pub fn new(engine_path: String, config: ConfigStore, credentials_path: PathBuf) -> Self {
        Self {
            engine_path,
            config,
            credentials_path,
            session: Arc::new(Mutex::new(SessionState::default())),
            ibus_context: IbusContext::default(),
            content_type: (0, 0),
        }
    }

    async fn start_recording(&self, emitter: &SignalEmitter<'_>) {
        let phase = lock_session(&self.session).phase;
        if phase != Phase::Idle {
            return;
        }

        let config = self.config.snapshot();
        let (capture, audio_rx) = match AudioCaptureHandle::start(config.input_device.as_deref()) {
            Ok(value) => value,
            Err(error) => {
                let message = format!(
                    "{}{error:#}",
                    i18n::text("Unable to start the microphone: ", "无法启动麦克风：")
                );
                tracing::error!(%message);
                show_error(emitter, message).await;
                return;
            }
        };

        let log_context = RecognitionLogContext::new(
            &self.engine_path,
            config.asr.provider.as_str(),
            &self.ibus_context,
            self.content_type,
        );
        let generation = {
            let mut session = lock_session(&self.session);
            session.generation = session.generation.wrapping_add(1);
            session.phase = Phase::Recording;
            session.capture = Some(capture);
            session.log_context = Some(log_context.clone());
            session.generation
        };
        log_context.log_started(&config.trigger_key, config.trigger_mode);

        let _ = Self::update_auxiliary_text(emitter, ibus_text(String::new()), false).await;
        update_preedit(
            emitter,
            i18n::text(WAITING_PREEDIT_ENGLISH, WAITING_PREEDIT_CHINESE),
        )
        .await;

        let owned_emitter = emitter.to_owned();
        let session = self.session.clone();
        let credentials_path = self.credentials_path.clone();
        let asr_config = config.asr.clone();
        let recognition_span = tracing::info_span!(
            "voice_session",
            session_id = %log_context.session_id,
            provider = %log_context.provider
        );
        tokio::spawn(
            run_recognition(
                session.clone(),
                generation,
                audio_rx,
                asr_config,
                credentials_path,
                owned_emitter.clone(),
                log_context,
            )
            .instrument(recognition_span),
        );

        let max_duration = Duration::from_secs(config.max_recording_seconds);
        tokio::spawn(async move {
            tokio::time::sleep(max_duration).await;
            if request_stop(&session, generation) {
                let session_id = active_session_id(&session).unwrap_or_else(|| "unknown".into());
                tracing::warn!(generation, %session_id, "maximum recording duration reached");
                let _ = Self::update_auxiliary_text(
                    &owned_emitter,
                    ibus_text(i18n::text("Finishing recognition…", "正在完成识别…").to_string()),
                    true,
                )
                .await;
            }
        });
    }

    async fn stop_recording(&self, emitter: &SignalEmitter<'_>) {
        let should_stop = {
            let mut session = lock_session(&self.session);
            if session.phase != Phase::Recording {
                false
            } else {
                session.phase = Phase::Processing;
                if let Some(mut capture) = session.capture.take() {
                    capture.stop();
                }
                true
            }
        };
        if should_stop {
            let _ = Self::update_auxiliary_text(
                emitter,
                ibus_text(i18n::text("Finishing recognition…", "正在完成识别…").to_string()),
                true,
            )
            .await;
        }
    }

    async fn toggle_recording(&self, emitter: &SignalEmitter<'_>) {
        let phase = { lock_session(&self.session).phase };
        match phase {
            Phase::Idle => self.start_recording(emitter).await,
            Phase::Recording => self.stop_recording(emitter).await,
            Phase::Processing => {
                let _ = Self::update_auxiliary_text(
                    emitter,
                    ibus_text(
                        i18n::text("Finishing the previous recording…", "正在完成上一段识别…")
                            .to_string(),
                    ),
                    true,
                )
                .await;
            }
        }
    }

    async fn cancel(&self, emitter: &SignalEmitter<'_>, show_message: bool) {
        let canceled = cancel_session(&self.session);
        if let Some(log_context) = canceled {
            log_context.log_finished("canceled", None, None);
            hide_preedit(emitter).await;
            if show_message {
                let _ = Self::update_auxiliary_text(
                    emitter,
                    ibus_text(i18n::text("Voice input canceled", "已取消语音输入").to_string()),
                    true,
                )
                .await;
            }
        }
    }

    async fn register_config_properties(&self, emitter: &SignalEmitter<'_>) {
        let properties = properties::config_properties(&self.config.snapshot());
        if let Err(error) = Self::register_properties(emitter, properties).await {
            tracing::warn!(%error, "failed to register IBus configuration properties");
        }
    }

    fn has_active_session(&self) -> bool {
        lock_session(&self.session).phase != Phase::Idle
    }

    fn clear_focus_context(&mut self) {
        self.ibus_context.focused = false;
        self.ibus_context.client = None;
        self.ibus_context.object_path = None;
        self.ibus_context.cursor = CursorLocation::default();
        self.content_type = (0, 0);
    }
}

#[interface(name = "org.freedesktop.IBus.Engine")]
impl VoiceEngine {
    async fn process_key_event(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        keyval: u32,
        _keycode: u32,
        state: u32,
    ) -> fdo::Result<bool> {
        let keyval = Keysym::new(keyval);
        let pressed = state & RELEASE_MASK == 0;
        tracing::debug!(keyval = keyval.raw(), state, pressed, "IBus key event");

        let config = self.config.snapshot();
        let is_trigger = config
            .trigger_keysym()
            .map(|configured| configured == keyval)
            .unwrap_or(false);
        if is_trigger {
            tracing::info!(
                trigger = %config.trigger_key,
                pressed,
                mode = ?config.trigger_mode,
                "voice trigger event"
            );
            match config.trigger_mode {
                TriggerMode::Hold => {
                    if pressed {
                        self.start_recording(&emitter).await;
                    } else {
                        self.stop_recording(&emitter).await;
                    }
                }
                TriggerMode::Toggle if pressed => self.toggle_recording(&emitter).await,
                TriggerMode::Toggle => {}
            }
            return Ok(true);
        }

        if keyval == Keysym::new(key::Escape) && pressed && self.has_active_session() {
            self.cancel(&emitter, true).await;
            return Ok(true);
        }
        Ok(false)
    }

    fn set_cursor_location(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.ibus_context.cursor = CursorLocation {
            x,
            y,
            width,
            height,
        };
    }

    fn process_hand_writing_event(&mut self, _coordinates: Vec<f64>) {}

    fn cancel_hand_writing(&mut self, _n_strokes: u32) {}

    fn set_capabilities(&mut self, capabilities: u32) {
        self.ibus_context.capabilities = capabilities;
    }

    async fn property_activate(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        name: String,
        state: u32,
    ) -> fdo::Result<()> {
        let Some(action) = properties::action_for_activation(&name, state) else {
            return Ok(());
        };
        let next = self
            .config
            .update(|config| match action {
                ConfigAction::SetMode(mode) => config.trigger_mode = mode,
                ConfigAction::SetTrigger(trigger) => config.trigger_key = trigger,
            })
            .map_err(|error| {
                fdo::Error::Failed(format!(
                    "{}{error:#}",
                    i18n::text(
                        "Failed to save typeless-ibus configuration: ",
                        "保存 typeless-ibus 配置失败："
                    )
                ))
            })?;
        tracing::info!(
            trigger = %next.trigger_key,
            mode = ?next.trigger_mode,
            "updated configuration from IBus property"
        );
        self.register_config_properties(&emitter).await;
        Ok(())
    }

    fn property_show(&mut self, _name: String) {}

    fn property_hide(&mut self, _name: String) {}

    fn candidate_clicked(&mut self, _index: u32, _button: u32, _state: u32) {}

    async fn focus_in(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.ibus_context.focused = true;
        self.register_config_properties(&emitter).await;
    }

    async fn focus_in_id(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        object_path: String,
        client: String,
    ) {
        self.ibus_context.focused = true;
        self.ibus_context.object_path = non_empty_context_value(object_path);
        self.ibus_context.client = non_empty_context_value(client);
        self.register_config_properties(&emitter).await;
    }

    async fn focus_out(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.cancel(&emitter, false).await;
        self.clear_focus_context();
    }

    async fn focus_out_id(
        &mut self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        _object_path: String,
    ) {
        self.cancel(&emitter, false).await;
        self.clear_focus_context();
    }

    async fn reset(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.cancel(&emitter, false).await;
    }

    async fn enable(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.register_config_properties(&emitter).await;
    }

    async fn disable(&mut self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) {
        self.cancel(&emitter, false).await;
    }

    fn page_up(&mut self) {}

    fn page_down(&mut self) {}

    fn cursor_up(&mut self) {}

    fn cursor_down(&mut self) {}

    fn set_surrounding_text(&mut self, _text: Value<'_>, _cursor_pos: u32, _anchor_pos: u32) {}

    fn panel_extension_received(&mut self, _event: Value<'_>) {}

    fn panel_extension_register_keys(&mut self, _data: Value<'_>) {}

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_lookup_table(
        emitter: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn register_properties(
        emitter: &SignalEmitter<'_>,
        properties: Value<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_property(emitter: &SignalEmitter<'_>, property: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn forward_key_event(
        emitter: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn panel_extension(emitter: &SignalEmitter<'_>, data: Value<'_>) -> zbus::Result<()>;

    #[zbus(property)]
    fn set_content_type(&mut self, value: (u32, u32)) {
        self.content_type = value;
    }

    #[zbus(property)]
    fn focus_id(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn active_surrounding_text(&self) -> bool {
        false
    }
}

async fn run_recognition(
    session: Arc<Mutex<SessionState>>,
    generation: u64,
    audio_rx: mpsc::Receiver<Vec<u8>>,
    asr_config: AsrConfig,
    credentials_path: PathBuf,
    emitter: SignalEmitter<'static>,
    log_context: RecognitionLogContext,
) {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let recognition = asr::transcribe(&asr_config, audio_rx, &credentials_path, move |event| {
        let _ = event_tx.send(event);
    });
    tokio::pin!(recognition);
    let mut latest_text = String::new();

    let result = loop {
        tokio::select! {
            event = event_rx.recv() => {
                if let Some(event) = event {
                    if !is_current(&session, generation) {
                        return;
                    }
                    match event {
                        AsrEvent::SpeechStarted => {
                            update_preedit(
                                &emitter,
                                i18n::text("🎙 Recognizing…", "🎙 正在识别…"),
                            )
                            .await
                        }
                        AsrEvent::Partial(text) | AsrEvent::Final(text) => {
                            latest_text.clone_from(&text);
                            update_preedit(&emitter, &text).await;
                        }
                    }
                }
            }
            result = &mut recognition => break result,
        }
    };

    if !finish_session(&session, generation) {
        return;
    }
    let commit_text = preferred_transcript(&result, &latest_text);
    if let Err(error) = &result
        && let Some(text) = &commit_text
    {
        tracing::warn!(
            error = %format_args!("{error:#}"),
            characters = text.chars().count(),
            "recognition ended with an error; committing the latest visible transcript"
        );
    }
    hide_preedit(&emitter).await;
    if let Some(text) = commit_text {
        if let Err(error) = VoiceEngine::commit_text(&emitter, ibus_text(text.clone())).await {
            tracing::error!(%error, "failed to commit recognized text");
            log_context.log_finished("commit_failed", Some(&text), Some(&error.to_string()));
            show_error(
                &emitter,
                format!(
                    "{}{error}",
                    i18n::text(
                        "Failed to insert the recognition result: ",
                        "提交识别结果失败："
                    )
                ),
            )
            .await;
        } else {
            tracing::info!(
                characters = text.chars().count(),
                "committed recognized text"
            );
            log_context.log_finished("committed", Some(&text), None);
            let _ =
                VoiceEngine::update_auxiliary_text(&emitter, ibus_text(String::new()), false).await;
        }
        return;
    }
    match result {
        Ok(_) => {
            log_context.log_finished("no_speech", None, None);
            show_error(
                &emitter,
                i18n::text("No speech was recognized", "没有识别到语音").to_string(),
            )
            .await
        }
        Err(error) => {
            tracing::error!(error = %format_args!("{error:#}"), "speech recognition failed");
            log_context.log_finished("recognition_failed", None, Some(&format!("{error:#}")));
            show_error(
                &emitter,
                format!(
                    "{}{error:#}",
                    i18n::text("Speech recognition failed: ", "语音识别失败：")
                ),
            )
            .await;
        }
    }
}

fn preferred_transcript(result: &anyhow::Result<String>, latest_text: &str) -> Option<String> {
    let final_text = result.as_deref().unwrap_or_default().trim();
    let text = if final_text.is_empty() {
        latest_text.trim()
    } else {
        final_text
    };
    (!text.is_empty()).then(|| text.to_string())
}

fn request_stop(session: &Arc<Mutex<SessionState>>, generation: u64) -> bool {
    let mut session = lock_session(session);
    if session.generation != generation || session.phase != Phase::Recording {
        return false;
    }
    session.phase = Phase::Processing;
    if let Some(mut capture) = session.capture.take() {
        capture.stop();
    }
    true
}

fn cancel_session(session: &Arc<Mutex<SessionState>>) -> Option<RecognitionLogContext> {
    let mut session = lock_session(session);
    if session.phase == Phase::Idle {
        return None;
    }
    session.generation = session.generation.wrapping_add(1);
    session.phase = Phase::Idle;
    if let Some(mut capture) = session.capture.take() {
        capture.stop();
    }
    session.log_context.take()
}

fn finish_session(session: &Arc<Mutex<SessionState>>, generation: u64) -> bool {
    let mut session = lock_session(session);
    if session.generation != generation {
        return false;
    }
    if let Some(mut capture) = session.capture.take() {
        capture.stop();
    }
    session.phase = Phase::Idle;
    session.log_context = None;
    true
}

fn active_session_id(session: &Arc<Mutex<SessionState>>) -> Option<String> {
    lock_session(session)
        .log_context
        .as_ref()
        .map(|context| context.session_id.clone())
}

fn is_current(session: &Arc<Mutex<SessionState>>, generation: u64) -> bool {
    let session = lock_session(session);
    session.generation == generation && session.phase != Phase::Idle
}

fn lock_session(session: &Arc<Mutex<SessionState>>) -> MutexGuard<'_, SessionState> {
    session.lock().unwrap_or_else(|error| error.into_inner())
}

fn non_empty_context_value(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn input_purpose_name(purpose: u32) -> &'static str {
    match purpose {
        0 => "free-form",
        1 => "alpha",
        2 => "digits",
        3 => "number",
        4 => "phone",
        5 => "url",
        6 => "email",
        7 => "name",
        8 => "password",
        9 => "pin",
        10 => "terminal",
        _ => "unknown",
    }
}

fn elapsed_millis(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

async fn update_preedit(emitter: &SignalEmitter<'_>, text: &str) {
    let cursor = text.chars().count() as u32;
    if let Err(error) =
        VoiceEngine::update_preedit_text(emitter, ibus_text(text.to_string()), cursor, true, 0)
            .await
    {
        tracing::warn!(%error, "failed to update preedit text");
    }
}

async fn hide_preedit(emitter: &SignalEmitter<'_>) {
    let _ = VoiceEngine::update_preedit_text(emitter, ibus_text(String::new()), 0, false, 0).await;
}

async fn show_error(emitter: &SignalEmitter<'_>, message: String) {
    hide_preedit(emitter).await;
    let _ = VoiceEngine::update_auxiliary_text(emitter, ibus_text(message), true).await;
}

fn ibus_text(text: String) -> Value<'static> {
    let attributes = Structure::from((
        "IBusAttrList",
        HashMap::<String, Value<'static>>::new(),
        Vec::<Value<'static>>::new(),
    ));
    let text = Structure::from((
        "IBusText",
        HashMap::<String, Value<'static>>::new(),
        text,
        Value::new(attributes),
    ));
    Value::new(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ibus_text_has_expected_signature() {
        assert_eq!(
            ibus_text("测试".to_string()).value_signature(),
            "(sa{sv}sv)"
        );
    }

    #[test]
    fn release_mask_distinguishes_hold_events() {
        let pressed_state = 0_u32;
        assert_eq!(pressed_state & RELEASE_MASK, 0);
        assert_ne!(RELEASE_MASK & RELEASE_MASK, 0);
    }

    #[test]
    fn waiting_preedit_is_localized_and_uses_a_centered_ellipsis() {
        assert_eq!(WAITING_PREEDIT_ENGLISH, "Listening…");
        assert_eq!(WAITING_PREEDIT_CHINESE, "聆听中…");
    }

    #[test]
    fn stale_session_cannot_commit() {
        let session = Arc::new(Mutex::new(SessionState {
            phase: Phase::Recording,
            generation: 8,
            capture: None,
            log_context: None,
        }));
        assert!(!finish_session(&session, 7));
        assert_eq!(lock_session(&session).phase, Phase::Recording);
    }

    #[test]
    fn final_text_wins_and_latest_partial_prevents_data_loss() {
        let final_result: anyhow::Result<String> = Ok("最终文本".to_string());
        assert_eq!(
            preferred_transcript(&final_result, "中间文本").as_deref(),
            Some("最终文本")
        );

        let empty_result: anyhow::Result<String> = Ok(String::new());
        assert_eq!(
            preferred_transcript(&empty_result, "仍然可用的中间文本").as_deref(),
            Some("仍然可用的中间文本")
        );

        let failed_result: anyhow::Result<String> = Err(anyhow::anyhow!("收尾超时"));
        assert_eq!(
            preferred_transcript(&failed_result, "超时前的完整内容").as_deref(),
            Some("超时前的完整内容")
        );
    }

    #[test]
    fn ibus_context_values_are_normalized_for_logs() {
        assert_eq!(
            non_empty_context_value(" gtk4:org.example.App ".into()).as_deref(),
            Some("gtk4:org.example.App")
        );
        assert_eq!(non_empty_context_value("  ".into()), None);
        assert_eq!(input_purpose_name(0), "free-form");
        assert_eq!(input_purpose_name(8), "password");
        assert_eq!(input_purpose_name(99), "unknown");
    }
}
