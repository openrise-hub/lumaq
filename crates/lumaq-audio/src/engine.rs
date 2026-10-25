use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use crossbeam_channel::{bounded, Sender};
use lumaq_core::config::AudioSettings;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
    pub timestamp_ns: u64,
}

enum AudioWorkerCommand {
    Stop,
}

pub struct AudioEngine {
    is_running: Arc<AtomicBool>,
    cmd_tx: Arc<Mutex<Option<Sender<AudioWorkerCommand>>>>,
    mic_db: Arc<Mutex<f32>>,
    system_db: Arc<Mutex<f32>>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            cmd_tx: Arc::new(Mutex::new(None)),
            mic_db: Arc::new(Mutex::new(-60.0)),
            system_db: Arc::new(Mutex::new(-60.0)),
        }
    }

    pub fn get_meter_levels(&self) -> (f32, f32) {
        (*self.mic_db.lock(), *self.system_db.lock())
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(tx) = self.cmd_tx.lock().take() {
            let _ = tx.send(AudioWorkerCommand::Stop);
        }
        *self.mic_db.lock() = -60.0;
        *self.system_db.lock() = -60.0;
    }

    pub fn start_audio_capture(
        &self,
        config: AudioSettings,
        chunk_tx: Sender<AudioChunk>,
    ) -> Result<()> {
        self.stop();
        self.is_running.store(true, Ordering::SeqCst);

        let (cmd_tx, cmd_rx) = bounded::<AudioWorkerCommand>(4);
        *self.cmd_tx.lock() = Some(cmd_tx);

        let is_running_flag = self.is_running.clone();
        let mic_db_store = self.mic_db.clone();
        let _system_db_store = self.system_db.clone();

        std::thread::Builder::new()
            .name("lumaq-audio-worker".into())
            .spawn(move || {
                let host = cpal::default_host();
                let start_time = Instant::now();
                let mut active_streams: Vec<Stream> = Vec::new();

                if config.mic_enabled {
                    let input_device = if let Some(ref dev_name) = config.mic_device_id {
                        host.input_devices()
                            .ok()
                            .and_then(|mut devs| devs.find(|d| d.name().map(|n| n == *dev_name).unwrap_or(false)))
                    } else {
                        host.default_input_device()
                    };

                    if let Some(device) = input_device {
                        if let Ok(default_cfg) = device.default_input_config() {
                            let sample_format = default_cfg.sample_format();
                            let stream_config: cpal::StreamConfig = default_cfg.into();
                            let channels = stream_config.channels;
                            let sample_rate = stream_config.sample_rate.0;

                            let tx = chunk_tx.clone();
                            let db_store = mic_db_store.clone();
                            let volume = config.mic_volume;

                            let err_fn = |err| error!("An error occurred on audio stream: {}", err);

                            let stream_res = match sample_format {
                                SampleFormat::F32 => device.build_input_stream(
                                    &stream_config,
                                    move |data: &[f32], _| {
                                        let mut samples = Vec::with_capacity(data.len());
                                        let mut sum_sq = 0.0f32;

                                        for &s in data {
                                            let scaled = s * volume;
                                            sum_sq += scaled * scaled;
                                            samples.push(scaled);
                                        }

                                        let rms = (sum_sq / (data.len().max(1) as f32)).sqrt();
                                        let db = if rms > 1e-4 {
                                            (20.0 * rms.log10()).clamp(-60.0, 0.0)
                                        } else {
                                            -60.0
                                        };
                                        *db_store.lock() = db;

                                        let chunk = AudioChunk {
                                            channels,
                                            sample_rate,
                                            samples,
                                            timestamp_ns: start_time.elapsed().as_nanos() as u64,
                                        };
                                        let _ = tx.try_send(chunk);
                                    },
                                    err_fn,
                                    None,
                                ),
                                SampleFormat::I16 => device.build_input_stream(
                                    &stream_config,
                                    move |data: &[i16], _| {
                                        let mut samples = Vec::with_capacity(data.len());
                                        let mut sum_sq = 0.0f32;

                                        for &s in data {
                                            let f = (s as f32 / 32768.0) * volume;
                                            sum_sq += f * f;
                                            samples.push(f);
                                        }

                                        let rms = (sum_sq / (data.len().max(1) as f32)).sqrt();
                                        let db = if rms > 1e-4 {
                                            (20.0 * rms.log10()).clamp(-60.0, 0.0)
                                        } else {
                                            -60.0
                                        };
                                        *db_store.lock() = db;

                                        let chunk = AudioChunk {
                                            channels,
                                            sample_rate,
                                            samples,
                                            timestamp_ns: start_time.elapsed().as_nanos() as u64,
                                        };
                                        let _ = tx.try_send(chunk);
                                    },
                                    err_fn,
                                    None,
                                ),
                                _ => {
                                    warn!("Unsupported sample format");
                                    Err(cpal::BuildStreamError::DeviceNotAvailable)
                                }
                            };

                            if let Ok(stream) = stream_res {
                                if let Ok(()) = stream.play() {
                                    active_streams.push(stream);
                                    info!("Microphone stream active");
                                }
                            }
                        }
                    }
                }

                while is_running_flag.load(Ordering::Relaxed) {
                    if let Ok(cmd) = cmd_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                        match cmd {
                            AudioWorkerCommand::Stop => break,
                        }
                    }
                }

                drop(active_streams);
                info!("Audio worker thread terminated cleanly");
            })?;

        Ok(())
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}
