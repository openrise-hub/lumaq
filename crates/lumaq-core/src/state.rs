use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum RecordingState {
    Idle,
    Armed,
    Countdown {
        remaining_seconds: u32,
    },
    Recording {
        started_at: DateTime<Utc>,
        output_path: PathBuf,
    },
    Paused {
        started_at: DateTime<Utc>,
        paused_at: DateTime<Utc>,
        output_path: PathBuf,
    },
    Finalizing {
        output_path: PathBuf,
    },
    Completed {
        output_path: PathBuf,
        duration_seconds: f64,
        file_size_bytes: u64,
    },
    Failed {
        error: String,
    },
}

impl Default for RecordingState {
    fn default() -> Self {
        Self::Idle
    }
}

impl RecordingState {
    pub fn is_recording(&self) -> bool {
        matches!(self, Self::Recording { .. } | Self::Paused { .. })
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused { .. })
    }

    pub fn is_counting_down(&self) -> bool {
        matches!(self, Self::Countdown { .. })
    }

    pub fn can_start(&self) -> bool {
        matches!(self, Self::Idle | Self::Completed { .. } | Self::Failed { .. })
    }

    pub fn can_pause(&self) -> bool {
        matches!(self, Self::Recording { .. })
    }

    pub fn can_resume(&self) -> bool {
        matches!(self, Self::Paused { .. })
    }

    pub fn can_stop(&self) -> bool {
        matches!(self, Self::Recording { .. } | Self::Paused { .. } | Self::Countdown { .. })
    }
}
