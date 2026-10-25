use crate::config::AppConfig;
use crate::events::CoreEvent;
use crate::state::RecordingState;
use crate::telemetry::RecordingStats;
use anyhow::{bail, Result};
use chrono::Utc;
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sysinfo::Disks;
use tracing::{info, warn};

pub struct CoreController {
    config: Arc<RwLock<AppConfig>>,
    state: Arc<RwLock<RecordingState>>,
    stats: Arc<RwLock<RecordingStats>>,
    event_tx: Sender<CoreEvent>,
    event_rx: Receiver<CoreEvent>,
}

impl CoreController {
    pub fn new(initial_config: Option<AppConfig>) -> Self {
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        let config = initial_config.unwrap_or_default();

        Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(RecordingState::Idle)),
            stats: Arc::new(RwLock::new(RecordingStats::new())),
            event_tx,
            event_rx,
        }
    }

    pub fn event_receiver(&self) -> Receiver<CoreEvent> {
        self.event_rx.clone()
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.read().clone()
    }

    pub fn update_config<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut cfg = self.config.write();
        update_fn(&mut cfg);
    }

    pub fn get_state(&self) -> RecordingState {
        self.state.read().clone()
    }

    pub fn get_stats(&self) -> RecordingStats {
        self.stats.read().clone()
    }

    pub fn emit_event(&self, event: CoreEvent) {
        let _ = self.event_tx.send(event);
    }

    pub fn set_state(&self, new_state: RecordingState) {
        {
            let mut state = self.state.write();
            *state = new_state.clone();
        }
        self.emit_event(CoreEvent::StateChanged(new_state));
    }

    pub fn prepare_output_path(&self) -> Result<PathBuf> {
        let config = self.config.read();
        let dir = &config.output.output_dir;

        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        let now = Utc::now();
        let timestamp = now.format("%Y%m%d_%H%M%S");
        let ext = config.output.format.extension();
        let filename = format!("{}_{}.{}", config.output.filename_prefix, timestamp, ext);

        Ok(dir.join(filename))
    }

    pub fn check_disk_space(&self, path: &Path) -> u64 {
        let disks = Disks::new_with_refreshed_list();
        for disk in &disks {
            if path.starts_with(disk.mount_point()) {
                return disk.available_space() / (1024 * 1024);
            }
        }
        10_000
    }

    pub fn transition_to_countdown(&self, seconds: u32) -> Result<()> {
        let current = self.get_state();
        if !current.can_start() {
            bail!("Cannot start countdown from state: {:?}", current);
        }

        self.set_state(RecordingState::Countdown {
            remaining_seconds: seconds,
        });
        Ok(())
    }

    pub fn transition_to_recording(&self, output_path: PathBuf) -> Result<()> {
        let free_mb = self.check_disk_space(&output_path);
        let min_mb = self.config.read().output.min_disk_space_mb;

        if free_mb < min_mb {
            let err = format!(
                "Insufficient disk space! Available: {} MB, required minimum: {} MB",
                free_mb, min_mb
            );
            warn!("{}", err);
            self.set_state(RecordingState::Failed { error: err.clone() });
            bail!(err);
        }

        info!("Starting recording -> output: {:?}", output_path);
        self.set_state(RecordingState::Recording {
            started_at: Utc::now(),
            output_path,
        });
        Ok(())
    }

    pub fn transition_to_paused(&self) -> Result<()> {
        let current = self.get_state();
        if let RecordingState::Recording {
            started_at,
            output_path,
        } = current
        {
            self.set_state(RecordingState::Paused {
                started_at,
                paused_at: Utc::now(),
                output_path,
            });
            Ok(())
        } else {
            bail!("Cannot pause when not actively recording!");
        }
    }

    pub fn transition_to_resumed(&self) -> Result<()> {
        let current = self.get_state();
        if let RecordingState::Paused {
            started_at,
            output_path,
            ..
        } = current
        {
            self.set_state(RecordingState::Recording {
                started_at,
                output_path,
            });
            Ok(())
        } else {
            bail!("Cannot resume when not paused!");
        }
    }

    pub fn transition_to_completed(
        &self,
        output_path: PathBuf,
        duration_seconds: f64,
        file_size_bytes: u64,
    ) {
        self.set_state(RecordingState::Completed {
            output_path,
            duration_seconds,
            file_size_bytes,
        });
    }

    pub fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut RecordingStats),
    {
        let mut stats = self.stats.write();
        updater(&mut stats);
        self.emit_event(CoreEvent::StatsUpdated(stats.clone()));
    }
}
