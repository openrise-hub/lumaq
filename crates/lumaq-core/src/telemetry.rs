use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStats {
    pub elapsed_seconds: f64,
    pub frames_captured: u64,
    pub frames_dropped: u64,
    pub current_fps: f32,
    pub file_size_bytes: u64,
    pub mic_db: f32,
    pub system_db: f32,
    pub disk_free_mb: u64,
}

impl RecordingStats {
    pub fn new() -> Self {
        Self {
            elapsed_seconds: 0.0,
            frames_captured: 0,
            frames_dropped: 0,
            current_fps: 0.0,
            file_size_bytes: 0,
            mic_db: -60.0,
            system_db: -60.0,
            disk_free_mb: 0,
        }
    }
}
