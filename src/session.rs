use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    #[serde(default = "default_win_width")]
    pub width: f64,
    #[serde(default = "default_win_height")]
    pub height: f64,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default)]
    pub maximized: bool,
}

fn default_win_width() -> f64 {
    1120.0
}

fn default_win_height() -> f64 {
    740.0
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: default_win_width(),
            height: default_win_height(),
            x: None,
            y: None,
            maximized: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SavedTab {
    NewTab,
    Demo {
        #[serde(default)]
        scroll_top: f64,
    },
    File {
        path: String,
        #[serde(default)]
        scroll_top: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionState {
    #[serde(default = "default_tabs")]
    pub tabs: Vec<SavedTab>,
    #[serde(default)]
    pub active_index: usize,
    #[serde(default = "default_true")]
    pub dark: bool,
    #[serde(default = "default_true")]
    pub toc_visible: bool,
    #[serde(default = "default_zoom")]
    pub zoom: u32,
    #[serde(default)]
    pub window: WindowState,
    #[serde(default)]
    pub open_folder: Option<String>,
    #[serde(default = "default_sidebar_tab")]
    pub sidebar_tab: String,
}

fn default_sidebar_tab() -> String {
    "outline".to_string()
}

fn default_tabs() -> Vec<SavedTab> {
    vec![SavedTab::NewTab]
}

fn default_true() -> bool {
    true
}

fn default_zoom() -> u32 {
    100
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            tabs: default_tabs(),
            active_index: 0,
            dark: true,
            toc_visible: true,
            zoom: 100,
            window: WindowState::default(),
            open_folder: None,
            sidebar_tab: default_sidebar_tab(),
        }
    }
}

pub fn session_file_path() -> PathBuf {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let dir = PathBuf::from(local_app_data).join("MDViewerByAli");
        let _ = fs::create_dir_all(&dir);
        return dir.join("session.json");
    }
    std::env::temp_dir().join("MDViewerByAli-session.json")
}

pub fn load() -> SessionState {
    let path = session_file_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return SessionState::default();
    };
    let mut parsed: SessionState = serde_json::from_str(&raw).unwrap_or_default();
    if parsed.tabs.is_empty() {
        parsed.tabs = default_tabs();
        parsed.active_index = 0;
    } else if parsed.active_index >= parsed.tabs.len() {
        parsed.active_index = parsed.tabs.len() - 1;
    }
    parsed.zoom = parsed.zoom.clamp(70, 200);
    parsed.window.width = parsed.window.width.clamp(680.0, 7680.0);
    parsed.window.height = parsed.window.height.clamp(440.0, 4320.0);
    parsed
}

pub fn save(state: &SessionState) {
    let path = session_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = fs::write(&path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_session_state() {
        let state = SessionState {
            tabs: vec![
                SavedTab::NewTab,
                SavedTab::Demo { scroll_top: 142.5 },
                SavedTab::File {
                    path: r"E:\Docs\readme.md".to_string(),
                    scroll_top: 480.0,
                },
            ],
            active_index: 2,
            dark: false,
            toc_visible: false,
            zoom: 120,
            window: WindowState {
                width: 1280.0,
                height: 820.0,
                x: Some(100),
                y: Some(60),
                maximized: true,
            },
            open_folder: Some(r"E:\Docs".to_string()),
            sidebar_tab: "files".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let decoded: SessionState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, decoded);
    }

    #[test]
    fn deserializes_older_session_without_window_field() {
        let legacy = r#"{"tabs":[{"kind":"new_tab"}],"active_index":0,"dark":true,"toc_visible":true,"zoom":100}"#;
        let decoded: SessionState = serde_json::from_str(legacy).unwrap();
        assert_eq!(decoded.window, WindowState::default());
        assert_eq!(decoded.open_folder, None);
        assert_eq!(decoded.sidebar_tab, "outline");
    }
}
