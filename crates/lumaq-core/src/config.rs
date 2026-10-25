use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureMode {
    FullScreen,
    Window,
    Area,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AreaRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for AreaRegion {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1280,
            height: 720,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSelection {
    pub mode: CaptureMode,
    pub monitor_index: usize,
    pub window_id: Option<String>,
    pub window_title: Option<String>,
    pub region: Option<AreaRegion>,
}

impl Default for TargetSelection {
    fn default() -> Self {
        Self {
            mode: CaptureMode::FullScreen,
            monitor_index: 0,
            window_id: None,
            window_title: None,
            region: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoFormat {
    Mp4,
    Mkv,
    Webm,
    Gif,
}

impl Default for VideoFormat {
    fn default() -> Self {
        Self::Mp4
    }
}

impl VideoFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
            Self::Gif => "gif",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QualityPreset {
    Ultra,
    High,
    Medium,
    Low,
}

impl Default for QualityPreset {
    fn default() -> Self {
        Self::High
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HardwareEncoderMode {
    Auto,
    Nvenc,
    QuickSync,
    Amf,
    Vaapi,
    SoftwareCpu,
}

impl Default for HardwareEncoderMode {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSettings {
    pub fps: u32,
    pub show_cursor: bool,
    pub quality: QualityPreset,
    pub encoder: HardwareEncoderMode,
    pub scale_factor: f32,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            fps: 60,
            show_cursor: true,
            quality: QualityPreset::High,
            encoder: HardwareEncoderMode::Auto,
            scale_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    pub mic_enabled: bool,
    pub mic_device_id: Option<String>,
    pub mic_volume: f32,
    pub system_audio_enabled: bool,
    pub system_device_id: Option<String>,
    pub system_volume: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mic_enabled: true,
            mic_device_id: None,
            mic_volume: 1.0,
            system_audio_enabled: true,
            system_device_id: None,
            system_volume: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebcamShape {
    Circle,
    RoundedRectangle,
    Square,
}

impl Default for WebcamShape {
    fn default() -> Self {
        Self::Circle
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebcamPipSettings {
    pub enabled: bool,
    pub device_id: Option<String>,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub shape: WebcamShape,
    pub mirror: bool,
    pub border_color: String,
    pub border_width: u32,
}

impl Default for WebcamPipSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            device_id: None,
            width: 240,
            height: 240,
            x: 40,
            y: 40,
            shape: WebcamShape::Circle,
            mirror: true,
            border_color: "#6366F1".to_string(),
            border_width: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayEffectsSettings {
    pub show_mouse_clicks: bool,
    pub click_ripple_color: String,
    pub show_cursor_halo: bool,
    pub show_keystrokes: bool,
    pub keycast_position_bottom: bool,
    pub magnifier_enabled: bool,
    pub magnifier_scale: f32,
}

impl Default for OverlayEffectsSettings {
    fn default() -> Self {
        Self {
            show_mouse_clicks: true,
            click_ripple_color: "#38BDF8".to_string(),
            show_cursor_halo: false,
            show_keystrokes: false,
            keycast_position_bottom: true,
            magnifier_enabled: false,
            magnifier_scale: 2.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSettings {
    pub output_dir: PathBuf,
    pub format: VideoFormat,
    pub filename_prefix: String,
    pub countdown_seconds: u32,
    pub auto_stop_minutes: Option<u32>,
    pub min_disk_space_mb: u64,
}

impl Default for OutputSettings {
    fn default() -> Self {
        let default_dir = directories::UserDirs::new()
            .and_then(|dirs| dirs.video_dir().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("./recordings"));

        Self {
            output_dir: default_dir,
            format: VideoFormat::Mp4,
            filename_prefix: "Lumaq_Recording".to_string(),
            countdown_seconds: 3,
            auto_stop_minutes: None,
            min_disk_space_mb: 500,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub target: TargetSelection,
    pub video: VideoSettings,
    pub audio: AudioSettings,
    pub webcam: WebcamPipSettings,
    pub effects: OverlayEffectsSettings,
    pub output: OutputSettings,
}
