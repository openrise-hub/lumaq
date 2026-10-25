use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub channels: u16,
    pub default_sample_rate: u32,
}

pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let default_device_name = host
        .default_input_device()
        .and_then(|d| d.name().ok());

    match host.input_devices() {
        Ok(devices) => devices
            .filter_map(|d| {
                let name = d.name().ok()?;
                let is_default = default_device_name.as_deref() == Some(&name);
                let config = d.default_input_config().ok();
                let channels = config.as_ref().map(|c| c.channels()).unwrap_or(2);
                let default_sample_rate = config.as_ref().map(|c| c.sample_rate().0).unwrap_or(48000);

                Some(AudioDeviceInfo {
                    id: name.clone(),
                    name,
                    is_default,
                    channels,
                    default_sample_rate,
                })
            })
            .collect(),
        Err(err) => {
            error!("Failed to enumerate input devices: {}", err);
            Vec::new()
        }
    }
}

pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let default_device_name = host
        .default_output_device()
        .and_then(|d| d.name().ok());

    match host.output_devices() {
        Ok(devices) => devices
            .filter_map(|d| {
                let name = d.name().ok()?;
                let is_default = default_device_name.as_deref() == Some(&name);
                let config = d.default_output_config().ok();
                let channels = config.as_ref().map(|c| c.channels()).unwrap_or(2);
                let default_sample_rate = config.as_ref().map(|c| c.sample_rate().0).unwrap_or(48000);

                Some(AudioDeviceInfo {
                    id: name.clone(),
                    name,
                    is_default,
                    channels,
                    default_sample_rate,
                })
            })
            .collect(),
        Err(err) => {
            error!("Failed to enumerate output devices: {}", err);
            Vec::new()
        }
    }
}
