use anyhow::{anyhow, Result};
use crossbeam_channel::Sender;
use image::RgbaImage;
use lumaq_core::config::{CaptureMode, TargetSelection};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use xcap::{Monitor, Window};

#[derive(Debug, Clone)]
pub struct RawVideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub timestamp_ns: u64,
}

pub struct CaptureEngine {
    is_running: Arc<AtomicBool>,
}

impl CaptureEngine {
    pub fn new() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    pub fn start_capture(
        &self,
        target: TargetSelection,
        target_fps: u32,
        frame_tx: Sender<RawVideoFrame>,
    ) -> Result<()> {
        let running_flag = self.is_running.clone();
        running_flag.store(true, Ordering::SeqCst);

        let fps = target_fps.clamp(15, 120);
        let frame_duration = Duration::from_secs_f64(1.0 / (fps as f64));

        std::thread::Builder::new()
            .name("lumaq-capture-loop".into())
            .spawn(move || {
                info!("Capture thread started at {} FPS", fps);
                let start_time = Instant::now();

                let monitors = Monitor::all().unwrap_or_default();
                let selected_monitor = monitors.get(target.monitor_index).cloned();

                let mut next_frame_instant = Instant::now();

                while running_flag.load(Ordering::Relaxed) {
                    let now = Instant::now();
                    if now < next_frame_instant {
                        let remaining = next_frame_instant - now;
                        if remaining > Duration::from_millis(1) {
                            std::thread::sleep(remaining);
                        }
                    }
                    next_frame_instant += frame_duration;

                    let capture_res: Result<RgbaImage> = match target.mode {
                        CaptureMode::FullScreen => {
                            if let Some(ref m) = selected_monitor {
                                m.capture_image()
                                    .map_err(|e| anyhow!("Monitor capture error: {}", e))
                            } else {
                                Err(anyhow!("Selected monitor not found"))
                            }
                        }
                        CaptureMode::Window => {
                            if let Some(ref win_id) = target.window_id {
                                let windows = Window::all().unwrap_or_default();
                                let target_win = windows.into_iter().find(|w| {
                                    let id_str = w.id().to_string();
                                    id_str == *win_id
                                        || (target.window_title.as_deref().is_some()
                                            && w.title() == target.window_title.as_deref().unwrap_or(""))
                                });

                                if let Some(w) = target_win {
                                    w.capture_image()
                                        .map_err(|e| anyhow!("Window capture error: {}", e))
                                } else {
                                    Err(anyhow!("Target window not found"))
                                }
                            } else {
                                Err(anyhow!("Window ID not specified"))
                            }
                        }
                        CaptureMode::Area => {
                            if let Some(ref m) = selected_monitor {
                                match m.capture_image() {
                                    Ok(full_img) => {
                                        if let Some(ref reg) = target.region {
                                            let full_w = full_img.width();
                                            let full_h = full_img.height();
                                            let crop_x = (reg.x.max(0) as u32).min(full_w.saturating_sub(1));
                                            let crop_y = (reg.y.max(0) as u32).min(full_h.saturating_sub(1));
                                            let crop_w = reg.width.min(full_w.saturating_sub(crop_x)).max(2);
                                            let crop_h = reg.height.min(full_h.saturating_sub(crop_y)).max(2);

                                            let cropped = image::imageops::crop_imm(
                                                &full_img, crop_x, crop_y, crop_w, crop_h,
                                            )
                                            .to_image();
                                            Ok(cropped)
                                        } else {
                                            Ok(full_img)
                                        }
                                    }
                                    Err(e) => Err(anyhow!("Area base capture error: {}", e)),
                                }
                            } else {
                                Err(anyhow!("Monitor not found for area capture"))
                            }
                        }
                    };

                    match capture_res {
                        Ok(img) => {
                            let width = img.width();
                            let height = img.height();
                            let raw_bytes = img.into_raw();
                            let timestamp_ns = start_time.elapsed().as_nanos() as u64;

                            let frame = RawVideoFrame {
                                width,
                                height,
                                data: raw_bytes,
                                timestamp_ns,
                            };

                            if let Err(e) = frame_tx.try_send(frame) {
                                debug!("Dropping captured frame due to full queue: {:?}", e);
                            }
                        }
                        Err(err) => {
                            warn!("Frame capture failed: {}", err);
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                }

                info!("Capture thread ended cleanly");
            })?;

        Ok(())
    }
}

impl Default for CaptureEngine {
    fn default() -> Self {
        Self::new()
    }
}
