use anyhow::Result;
use crossbeam_channel::Receiver;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};
use lumaq_audio::AudioChunk;
use lumaq_capture::RawVideoFrame;
use lumaq_core::config::{AppConfig, VideoFormat};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::detect::{detect_capabilities, resolve_codec_flag};

pub struct EncoderSession {
    is_running: Arc<AtomicBool>,
    bytes_written: Arc<AtomicU64>,
    frames_written: Arc<AtomicU64>,
}

impl EncoderSession {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            bytes_written: Arc::new(AtomicU64::new(0)),
            frames_written: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn frames_written(&self) -> u64 {
        self.frames_written.load(Ordering::Relaxed)
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn start_encoding(
        &self,
        config: AppConfig,
        output_path: PathBuf,
        video_rx: Receiver<RawVideoFrame>,
        _audio_rx: Receiver<AudioChunk>,
    ) -> Result<()> {
        let is_running = self.is_running.clone();
        is_running.store(true, Ordering::SeqCst);

        let bytes_counter = self.bytes_written.clone();
        let frames_counter = self.frames_written.clone();

        let caps = detect_capabilities();
        let target_format = config.output.format;
        let fps = config.video.fps;

        std::thread::Builder::new()
            .name("lumaq-encoder-worker".into())
            .spawn(move || {
                info!("Encoder worker started for {:?}", output_path);

                match target_format {
                    VideoFormat::Gif => {
                        if let Ok(file) = File::create(&output_path) {
                            let mut encoder = GifEncoder::new_with_speed(file, 10);
                            let _ = encoder.set_repeat(Repeat::Infinite);
                            let frame_delay = Delay::from_saturating_duration(Duration::from_millis(
                                (1000.0 / fps.min(20) as f64) as u64,
                            ));

                            while is_running.load(Ordering::Relaxed) {
                                if let Ok(vframe) = video_rx.recv_timeout(Duration::from_millis(100)) {
                                    if let Some(img) = RgbaImage::from_raw(vframe.width, vframe.height, vframe.data) {
                                        let frame = Frame::from_parts(img, 0, 0, frame_delay);
                                        let _ = encoder.encode_frame(frame);
                                        frames_counter.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        if caps.ffmpeg_installed {
                            let codec = resolve_codec_flag(config.video.encoder, &caps);
                            info!("Using encoder codec: {}", codec);

                            if let Ok(first_frame) = video_rx.recv_timeout(Duration::from_secs(3)) {
                                let w = first_frame.width;
                                let h = first_frame.height;

                                let mut cmd = Command::new("ffmpeg");
                                cmd.arg("-y")
                                    .arg("-f").arg("rawvideo")
                                    .arg("-pixel_format").arg("rgba")
                                    .arg("-video_size").arg(format!("{}x{}", w, h))
                                    .arg("-framerate").arg(fps.to_string())
                                    .arg("-i").arg("-")
                                    .arg("-c:v").arg(codec)
                                    .arg("-pix_fmt").arg("yuv420p")
                                    .arg("-movflags").arg("+faststart+frag_keyframe+empty_moov")
                                    .arg(&output_path)
                                    .stdin(Stdio::piped())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null());

                                match cmd.spawn() {
                                    Ok(mut child) => {
                                        if let Some(mut stdin) = child.stdin.take() {
                                            let _ = stdin.write_all(&first_frame.data);
                                            bytes_counter.fetch_add(first_frame.data.len() as u64, Ordering::Relaxed);
                                            frames_counter.fetch_add(1, Ordering::Relaxed);

                                            while is_running.load(Ordering::Relaxed) {
                                                if let Ok(frame) = video_rx.recv_timeout(Duration::from_millis(50)) {
                                                    if stdin.write_all(&frame.data).is_err() {
                                                        break;
                                                    }
                                                    bytes_counter.fetch_add(frame.data.len() as u64, Ordering::Relaxed);
                                                    frames_counter.fetch_add(1, Ordering::Relaxed);
                                                }
                                            }
                                        }
                                        let _ = child.wait();
                                    }
                                    Err(err) => {
                                        error!("Failed to spawn ffmpeg: {}", err);
                                    }
                                }
                            }
                        } else {
                            warn!("ffmpeg not installed on system. Writing raw frames to container directory.");
                            let raw_dir = output_path.with_extension("frames");
                            let _ = std::fs::create_dir_all(&raw_dir);

                            let mut counter = 0u64;
                            while is_running.load(Ordering::Relaxed) {
                                if let Ok(vframe) = video_rx.recv_timeout(Duration::from_millis(50)) {
                                    if let Some(img) = RgbaImage::from_raw(vframe.width, vframe.height, vframe.data) {
                                        let p = raw_dir.join(format!("frame_{:06}.jpg", counter));
                                        let _ = img.save_with_format(p, image::ImageFormat::Jpeg);
                                        counter += 1;
                                        frames_counter.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                    }
                }

                info!("Encoder worker finished for {:?}", output_path);
            })?;

        Ok(())
    }
}

impl Default for EncoderSession {
    fn default() -> Self {
        Self::new()
    }
}
