use lumaq_core::config::HardwareEncoderMode;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareEncoderCapabilities {
    pub has_nvenc: bool,
    pub has_quicksync: bool,
    pub has_amf: bool,
    pub has_vaapi: bool,
    pub ffmpeg_installed: bool,
    pub ffmpeg_path: Option<String>,
}

pub fn detect_capabilities() -> HardwareEncoderCapabilities {
    let ffmpeg_path = which::which("ffmpeg").ok().map(|p| p.to_string_lossy().to_string());
    let ffmpeg_installed = ffmpeg_path.is_some();

    #[cfg(target_os = "windows")]
    let (has_nvenc, has_quicksync, has_amf, has_vaapi) = {
        (true, true, true, false)
    };

    #[cfg(target_os = "linux")]
    let (has_nvenc, has_quicksync, has_amf, has_vaapi) = {
        (true, false, false, true)
    };

    #[cfg(target_os = "macos")]
    let (has_nvenc, has_quicksync, has_amf, has_vaapi) = {
        (false, false, false, false)
    };

    let caps = HardwareEncoderCapabilities {
        has_nvenc,
        has_quicksync,
        has_amf,
        has_vaapi,
        ffmpeg_installed,
        ffmpeg_path,
    };

    info!("Hardware encoder capabilities detected: {:?}", caps);
    caps
}

pub fn resolve_codec_flag(mode: HardwareEncoderMode, caps: &HardwareEncoderCapabilities) -> &'static str {
    match mode {
        HardwareEncoderMode::Nvenc if caps.has_nvenc => "h264_nvenc",
        HardwareEncoderMode::QuickSync if caps.has_quicksync => "h264_qsv",
        HardwareEncoderMode::Amf if caps.has_amf => "h264_amf",
        HardwareEncoderMode::Vaapi if caps.has_vaapi => "h264_vaapi",
        HardwareEncoderMode::Auto => {
            if caps.has_nvenc {
                "h264_nvenc"
            } else if caps.has_quicksync {
                "h264_qsv"
            } else if caps.has_amf {
                "h264_amf"
            } else {
                "libx264"
            }
        }
        _ => "libx264",
    }
}
