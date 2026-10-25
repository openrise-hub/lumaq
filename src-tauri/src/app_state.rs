use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use lumaq_audio::{AudioChunk, AudioEngine};
use lumaq_capture::{CaptureEngine, RawVideoFrame};
use lumaq_core::controller::CoreController;
use lumaq_core::state::RecordingState;
use lumaq_encoder::EncoderSession;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tracing::info;

use crate::commands::RecordingResult;

pub struct LumaqAppState {
    pub core: CoreController,
    pub capture: CaptureEngine,
    pub audio: AudioEngine,
    pub encoder: Mutex<Option<EncoderSession>>,
    pub recording_start_instant: Mutex<Option<Instant>>,
    pub current_output_path: Mutex<Option<PathBuf>>,
    pub is_recording: Arc<AtomicBool>,
}

impl LumaqAppState {
    pub fn new() -> Self {
        Self {
            core: CoreController::new(None),
            capture: CaptureEngine::new(),
            audio: AudioEngine::new(),
            encoder: Mutex::new(None),
            recording_start_instant: Mutex::new(None),
            current_output_path: Mutex::new(None),
            is_recording: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start_recording(&self, app_handle: AppHandle) -> Result<()> {
        let config = self.core.get_config();
        let output_path = self.core.prepare_output_path()?;

        if config.output.countdown_seconds > 0 {
            for rem in (1..=config.output.countdown_seconds).rev() {
                self.core.set_state(RecordingState::Countdown {
                    remaining_seconds: rem,
                });
                let _ = app_handle.emit("recording-state-changed", self.core.get_state());
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }

        self.core.transition_to_recording(output_path.clone())?;
        let _ = app_handle.emit("recording-state-changed", self.core.get_state());

        *self.recording_start_instant.lock() = Some(Instant::now());
        *self.current_output_path.lock() = Some(output_path.clone());
        self.is_recording.store(true, Ordering::SeqCst);

        let (video_tx, video_rx): (Sender<RawVideoFrame>, Receiver<RawVideoFrame>) = bounded(120);
        let (audio_tx, audio_rx): (Sender<AudioChunk>, Receiver<AudioChunk>) = bounded(240);

        self.capture
            .start_capture(config.target.clone(), config.video.fps, video_tx)?;

        let _ = self.audio.start_audio_capture(config.audio.clone(), audio_tx);

        let encoder = EncoderSession::new();
        encoder.start_encoding(config, output_path, video_rx, audio_rx)?;
        *self.encoder.lock() = Some(encoder);

        info!("All subsystems started successfully");
        Ok(())
    }

    pub fn pause_recording(&self) -> Result<()> {
        self.core.transition_to_paused()?;
        Ok(())
    }

    pub fn resume_recording(&self) -> Result<()> {
        self.core.transition_to_resumed()?;
        Ok(())
    }

    pub async fn stop_recording(&self) -> Result<RecordingResult> {
        info!("Stopping recording session...");

        self.capture.stop();
        self.audio.stop();

        if let Some(encoder) = self.encoder.lock().take() {
            encoder.stop();
        }

        self.is_recording.store(false, Ordering::SeqCst);

        let duration_secs = self
            .recording_start_instant
            .lock()
            .take()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        let output_path = self
            .current_output_path
            .lock()
            .take()
            .unwrap_or_else(|| PathBuf::from("recording.mp4"));

        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let file_size_bytes = std::fs::metadata(&output_path)
            .map(|m| m.len())
            .unwrap_or(0);

        self.core.transition_to_completed(
            output_path.clone(),
            duration_secs,
            file_size_bytes,
        );

        let result = RecordingResult {
            file_path: output_path.to_string_lossy().to_string(),
            duration_seconds: duration_secs,
            file_size_bytes,
        };

        info!("Recording completed: {:?}", result);
        Ok(result)
    }
}
