use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalHotkeyBindings {
    pub toggle_record: String,
    pub toggle_pause: String,
    pub toggle_webcam: String,
    pub take_screenshot: String,
}

impl Default for GlobalHotkeyBindings {
    fn default() -> Self {
        Self {
            toggle_record: "Ctrl+Shift+R".to_string(),
            toggle_pause: "Ctrl+Shift+P".to_string(),
            toggle_webcam: "Ctrl+Shift+C".to_string(),
            take_screenshot: "Ctrl+Shift+S".to_string(),
        }
    }
}
