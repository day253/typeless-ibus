use super::AsrEvent;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;
use typeless_ibus::config::AsrProviderKind;

pub(crate) type EventHandler = Box<dyn FnMut(AsrEvent) + Send>;
pub(crate) type RecognitionFuture<'a> = Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
pub(crate) type DiagnosticFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

/// Provider boundary between the IBus session and a speech-recognition backend.
///
/// Providers receive the same 16 kHz mono s16le PCM stream. A streaming backend
/// may emit partial events while audio arrives; a batch backend emits a final
/// event after the sender closes.
pub(crate) trait AsrProvider: Send + Sync {
    fn kind(&self) -> AsrProviderKind;

    fn transcribe<'a>(
        &'a self,
        audio_rx: mpsc::Receiver<Vec<u8>>,
        on_event: EventHandler,
    ) -> RecognitionFuture<'a>;

    fn diagnose<'a>(&'a self) -> DiagnosticFuture<'a>;
}
