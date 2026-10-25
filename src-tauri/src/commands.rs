use lumaq_audio::{list_input_devices, list_output_devices, AudioDeviceInfo};
use lumaq_capture::{list_monitors, list_windows, MonitorInfo, WindowInfo};
use lumaq_core::config::AppConfig;
use lumaq_core::state::RecordingState;
use lumaq_core::telemetry::RecordingStats;
use lumaq_encoder::{detect_capabilities, HardwareEncoderCapabilities};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, State};

use crate::app_state::LumaqAppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingResult {
    pub file_path: String,
    pub duration_seconds: f64,
    pub file_size_bytes: u64,
}

#[tauri::command]
pub fn get_config(state: State<'_, LumaqAppState>) -> AppConfig {
    state.core.get_config()
}

#[tauri::command]
pub fn save_config(
    state: State<'_, LumaqAppState>,
    config: AppConfig,
) -> Result<(), String> {
    state.core.update_config(|c| *c = config);
    Ok(())
}

#[tauri::command]
pub fn get_state(state: State<'_, LumaqAppState>) -> RecordingState {
    state.core.get_state()
}

#[tauri::command]
pub fn get_stats(state: State<'_, LumaqAppState>) -> RecordingStats {
    let mut stats = state.core.get_stats();
    let (mic_db, sys_db) = state.audio.get_meter_levels();
    stats.mic_db = mic_db;
    stats.system_db = sys_db;
    stats
}

#[tauri::command]
pub fn get_monitors() -> Vec<MonitorInfo> {
    list_monitors()
}

#[tauri::command]
pub fn get_windows() -> Vec<WindowInfo> {
    list_windows()
}

#[tauri::command]
pub fn get_audio_inputs() -> Vec<AudioDeviceInfo> {
    list_input_devices()
}

#[tauri::command]
pub fn get_audio_outputs() -> Vec<AudioDeviceInfo> {
    list_output_devices()
}

#[tauri::command]
pub fn get_hardware_capabilities() -> HardwareEncoderCapabilities {
    detect_capabilities()
}

#[tauri::command]
pub async fn start_recording(
    state: State<'_, LumaqAppState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    state.start_recording(app_handle).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pause_recording(state: State<'_, LumaqAppState>) -> Result<(), String> {
    state.pause_recording().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resume_recording(state: State<'_, LumaqAppState>) -> Result<(), String> {
    state.resume_recording().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, LumaqAppState>,
) -> Result<RecordingResult, String> {
    state.stop_recording().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_recording_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let folder = if p.is_file() {
        p.parent().unwrap_or(&p).to_path_buf()
    } else {
        p
    };

    open::that(&folder).map_err(|e| format!("Failed to open folder: {}", e))
}

#[tauri::command]
pub fn open_recording_file(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| format!("Failed to open file: {}", e))
}
