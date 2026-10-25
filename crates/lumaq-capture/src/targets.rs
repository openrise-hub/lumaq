use serde::{Deserialize, Serialize};
use tracing::error;
use xcap::{Monitor, Window};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
    pub scale_factor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_minimized: bool,
}

pub fn list_monitors() -> Vec<MonitorInfo> {
    match Monitor::all() {
        Ok(monitors) => monitors
            .into_iter()
            .enumerate()
            .map(|(idx, m)| MonitorInfo {
                id: idx,
                name: m.name().to_string(),
                width: m.width(),
                height: m.height(),
                is_primary: m.is_primary(),
                scale_factor: m.scale_factor(),
            })
            .collect(),
        Err(err) => {
            error!("Failed to enumerate monitors: {}", err);
            vec![MonitorInfo {
                id: 0,
                name: "Primary Display".to_string(),
                width: 1920,
                height: 1080,
                is_primary: true,
                scale_factor: 1.0,
            }]
        }
    }
}

pub fn list_windows() -> Vec<WindowInfo> {
    match Window::all() {
        Ok(windows) => windows
            .into_iter()
            .filter_map(|w| {
                let title = w.title().to_string();
                let app_name = w.app_name().to_string();
                let width = w.width();
                let height = w.height();
                let is_minimized = w.is_minimized();

                if title.trim().is_empty() && app_name.trim().is_empty() {
                    return None;
                }
                if width < 50 || height < 50 {
                    return None;
                }

                Some(WindowInfo {
                    id: format!("{}:{}", w.id(), app_name),
                    title: if title.is_empty() {
                        app_name.clone()
                    } else {
                        title
                    },
                    app_name,
                    x: w.x(),
                    y: w.y(),
                    width,
                    height,
                    is_minimized,
                })
            })
            .collect(),
        Err(err) => {
            error!("Failed to enumerate windows: {}", err);
            Vec::new()
        }
    }
}
