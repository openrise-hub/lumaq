use crate::state::RecordingState;
use crate::telemetry::RecordingStats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum CoreEvent {
    StateChanged(RecordingState),
    StatsUpdated(RecordingStats),
    Warning(String),
    Error(String),
}
