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
pub struct SavedPrintSettings {
    #[serde(default = "default_print_dest")]
    pub destination: String,
    #[serde(default)]
    pub theme_dark: bool,
    #[serde(default = "default_paper_size")]
    pub paper_size: String,
    #[serde(default)]
    pub landscape: bool,
    #[serde(default = "default_margin_mm")]
    pub margin_mm: u32,
    #[serde(default = "default_zoom")]
    pub scale: u32,
}

fn default_print_dest() -> String {
    "__pdf__".to_string()
}

fn default_paper_size() -> String {
    "a4".to_string()
}

fn default_margin_mm() -> u32 {
    14
}

impl Default for SavedPrintSettings {
    fn default() -> Self {
        Self {
            destination: default_print_dest(),
            theme_dark: false,
            paper_size: default_paper_size(),
            landscape: false,
            margin_mm: default_margin_mm(),
            scale: 100,
        }
    }
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
    #[serde(default)]
    pub wide_mode: bool,
    #[serde(default)]
    pub print: SavedPrintSettings,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_line_height")]
    pub line_height: String,
}

fn default_font_family() -> String {
    "system".to_string()
}

fn default_line_height() -> String {
    "normal".to_string()
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
            wide_mode: false,
            print: SavedPrintSettings::default(),
            font_family: default_font_family(),
            line_height: default_line_height(),
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
    parsed.print.scale = parsed.print.scale.clamp(50, 150);
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
            wide_mode: true,
            print: SavedPrintSettings {
                destination: "Microsoft Print to PDF".to_string(),
                theme_dark: true,
                paper_size: "letter".to_string(),
                landscape: true,
                margin_mm: 7,
                scale: 90,
            },
            font_family: "cairo".to_string(),
            line_height: "relaxed".to_string(),
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
        assert!(!decoded.wide_mode);
        assert_eq!(decoded.print, SavedPrintSettings::default());
        assert_eq!(decoded.font_family, "system");
        assert_eq!(decoded.line_height, "normal");
        assert!(!decoded.print.theme_dark);
        assert_eq!(decoded.print.destination, "__pdf__");
        assert_eq!(decoded.print.paper_size, "a4");
        assert!(!decoded.print.landscape);
        assert_eq!(decoded.print.margin_mm, 14);
        assert_eq!(decoded.print.scale, 100);
    }
}
