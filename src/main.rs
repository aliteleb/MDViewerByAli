#![cfg_attr(windows, windows_subsystem = "windows")]

mod dwm;
mod emoji;
mod file_assoc;
mod folder;
mod font_assets;
mod katex_assets;
mod markdown;
mod printing;
mod session;
mod single_instance;
mod watcher;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use http::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy},
    window::{Window, WindowBuilder},
};
use wry::{DragDropEvent, WebContext, WebView, WebViewBuilder};

const INDEX_HTML: &[u8] = include_bytes!("../assets/app/index.html");
const STYLES_CSS: &[u8] = include_bytes!("../assets/app/styles.css");
const APP_JS: &[u8] = include_bytes!("../assets/app/app.js");
const MERMAID_JS: &[u8] = include_bytes!("../assets/vendor/mermaid.min.js");
const HIGHLIGHT_JS: &[u8] = include_bytes!("../assets/vendor/highlight.min.js");

fn inlined_index_html() -> &'static [u8] {
    static INLINED: OnceLock<Vec<u8>> = OnceLock::new();
    INLINED
        .get_or_init(|| {
            let html = String::from_utf8_lossy(INDEX_HTML);
            let css = String::from_utf8_lossy(STYLES_CSS);
            let js = String::from_utf8_lossy(APP_JS);
            let with_css = html.replacen(
                r#"<link rel="stylesheet" href="/styles.css" />"#,
                &format!("<style>{css}</style>"),
                1,
            );
            let with_js = with_css.replacen(
                r#"<script src="/app.js"></script>"#,
                &format!("<script>{js}</script>"),
                1,
            );
            with_js.into_bytes()
        })
        .as_slice()
}

#[derive(Debug)]
pub enum UserEvent {
    Ipc(String),
    DropPaths(Vec<PathBuf>),
    PickedFiles(Vec<PathBuf>),
    PickedFolder(PathBuf),
    PickedPdfOutput {
        path: PathBuf,
        options: printing::PrintOptions,
    },
    PickedSaveAs {
        old_path: String,
        new_path: PathBuf,
        content: String,
    },
    PdfExported {
        path: PathBuf,
        success: bool,
    },
    PrintCompleted {
        success: bool,
        message: String,
    },
    ExternalOpen(Vec<PathBuf>),
    FileChanged(PathBuf),
    DragHover(bool),
    TitleChanged(String),
    InitWebView,
    ShowWebView,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum IpcMessage {
    Ready,
    OpenDialog,
    OpenFolderDialog,
    OpenFilePath { path: String },
    RefreshFolder { path: String },
    OpenDemo,
    CloseTab { path: String },
    SetTheme { dark: bool },
    OpenExternal { url: String },
    OpenRelativeLink { current_path: String, href: String },
    GetPrinters,
    RenderLivePreview {
        path: String,
        content: String,
    },
    SaveFile {
        path: String,
        content: String,
    },
    PrintDocument {
        options: printing::PrintOptions,
    },
    ExportPdf {
        suggested_name: String,
        current_path: Option<String>,
        options: printing::PrintOptions,
    },
    SaveSession { session: session::SessionState },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum RestoredTab {
    NewTab,
    Doc {
        doc: markdown::RenderedDoc,
        scroll_top: f64,
    },
}

#[derive(Debug, Serialize)]
struct RestoredPayload {
    tabs: Vec<RestoredTab>,
    active_index: usize,
    dark: bool,
    toc_visible: bool,
    zoom: u32,
    folder: Option<folder::FolderTree>,
    sidebar_tab: String,
    wide_mode: bool,
    print: session::SavedPrintSettings,
    font_family: String,
    line_height: String,
}

fn webview_data_dir() -> PathBuf {
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let dir = PathBuf::from(local_app_data)
            .join("MDViewerByAli")
            .join("WebView2");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }
    let fallback = std::env::temp_dir().join("MDViewerByAli-WebView2");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

fn url_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
                    && let Ok(val) = u8::from_str_radix(hex, 16)
                {
                    out.push(val);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn mime_for_path(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn static_response(body: &'static [u8], content_type: &'static str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cache-Control", "no-store")
        .body(Cow::Borrowed(body))
        .unwrap()
}

fn static_cached_response(
    body: &'static [u8],
    content_type: &'static str,
) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("Cache-Control", "public, max-age=31536000, immutable")
        .body(Cow::Borrowed(body))
        .unwrap()
}

fn error_response(status: StatusCode, msg: &'static str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Cow::Borrowed(msg.as_bytes()))
        .unwrap()
}

fn handle_local_file(
    query: Option<&str>,
    allowed_dirs: &Arc<Mutex<HashSet<PathBuf>>>,
) -> Response<Cow<'static, [u8]>> {
    let Some(q) = query else {
        return error_response(StatusCode::BAD_REQUEST, "Missing path query");
    };

    let raw_path = q
        .split('&')
        .find_map(|pair| pair.strip_prefix("path="))
        .map(url_decode);

    let Some(path_str) = raw_path else {
        return error_response(StatusCode::BAD_REQUEST, "Missing path parameter");
    };

    let Ok(canonical) = fs::canonicalize(&path_str) else {
        return error_response(StatusCode::NOT_FOUND, "File not found");
    };

    let allowed = allowed_dirs
        .lock()
        .map(|dirs| dirs.iter().any(|dir| canonical.starts_with(dir)))
        .unwrap_or(false);
    if !allowed {
        return error_response(StatusCode::FORBIDDEN, "Path outside open document directory");
    }

    let Ok(bytes) = fs::read(&canonical) else {
        return error_response(StatusCode::NOT_FOUND, "Unable to read file");
    };

    let mime = mime_for_path(&canonical);
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", mime)
        .body(Cow::Owned(bytes))
        .unwrap()
}

fn push_doc_to_webview(webview: &WebView, fn_name: &str, doc: &markdown::RenderedDoc) {
    if let Ok(json) = serde_json::to_string(doc) {
        let script = format!("window.{fn_name}({json});");
        let _ = webview.evaluate_script(&script);
    }
}

fn push_folder_to_webview(webview: &WebView, tree: &folder::FolderTree) {
    if let Ok(json) = serde_json::to_string(tree) {
        let script = format!("window.__MDV_SET_FOLDER__({json});");
        let _ = webview.evaluate_script(&script);
    }
}

fn load_and_watch_file(
    raw_path: &Path,
    allowed_dirs: &Arc<Mutex<HashSet<PathBuf>>>,
    watcher: &mut Option<watcher::FileWatcher>,
) -> Option<markdown::RenderedDoc> {
    if !markdown::is_supported_file(raw_path) {
        return None;
    }
    let path = fs::canonicalize(raw_path).unwrap_or_else(|_| raw_path.to_path_buf());
    let content = fs::read_to_string(&path).ok()?;

    if let Some(parent) = path.parent()
        && let Ok(mut dirs) = allowed_dirs.lock()
    {
        dirs.insert(parent.to_path_buf());
    }
    if let Some(w) = watcher.as_mut() {
        w.watch(&path);
    }

    Some(markdown::render_document(&content, &path))
}

fn open_file_paths(
    paths: &[PathBuf],
    webview: &WebView,
    allowed_dirs: &Arc<Mutex<HashSet<PathBuf>>>,
    watcher: &mut Option<watcher::FileWatcher>,
) {
    let mut folder_sent = false;
    for raw_path in paths {
        if raw_path.is_dir() {
            if !folder_sent && let Some(tree) = folder::scan_folder(raw_path) {
                push_folder_to_webview(webview, &tree);
                folder_sent = true;
            }
        } else if let Some(rendered) = load_and_watch_file(raw_path, allowed_dirs, watcher) {
            push_doc_to_webview(webview, "__MDV_OPEN_DOC__", &rendered);
        }
    }
}

fn build_restored_payload(
    saved: &session::SessionState,
    allowed_dirs: &Arc<Mutex<HashSet<PathBuf>>>,
    watcher: &mut Option<watcher::FileWatcher>,
) -> RestoredPayload {
    let mut tabs = Vec::new();
    let mut active_index = 0usize;

    for (i, saved_tab) in saved.tabs.iter().enumerate() {
        let restored = match saved_tab {
            session::SavedTab::NewTab => Some(RestoredTab::NewTab),
            session::SavedTab::Demo { scroll_top } => Some(RestoredTab::Doc {
                doc: markdown::render_document(
                    markdown::demo_markdown(),
                    Path::new("Welcome-Demo.md"),
                ),
                scroll_top: *scroll_top,
            }),
            session::SavedTab::File { path, scroll_top } => {
                load_and_watch_file(Path::new(path), allowed_dirs, watcher).map(|doc| {
                    RestoredTab::Doc {
                        doc,
                        scroll_top: *scroll_top,
                    }
                })
            }
        };

        if let Some(tab) = restored {
            if i == saved.active_index {
                active_index = tabs.len();
            }
            tabs.push(tab);
        }
    }

    if tabs.is_empty() {
        tabs.push(RestoredTab::NewTab);
        active_index = 0;
    } else if active_index >= tabs.len() {
        active_index = tabs.len() - 1;
    }

    let folder = saved
        .open_folder
        .as_deref()
        .and_then(|p| folder::scan_folder(Path::new(p)));

    RestoredPayload {
        tabs,
        active_index,
        dark: saved.dark,
        toc_visible: saved.toc_visible,
        zoom: saved.zoom,
        folder,
        sidebar_tab: saved.sidebar_tab.clone(),
        wide_mode: saved.wide_mode,
        print: saved.print.clone(),
        font_family: saved.font_family.clone(),
        line_height: saved.line_height.clone(),
    }
}

fn spawn_file_picker(proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        let picked = rfd::FileDialog::new()
            .set_title("Open Markdown or Mermaid File")
            .add_filter(
                "Markdown & Mermaid",
                &["md", "markdown", "mmd", "mermaid", "txt"],
            )
            .add_filter("All Files", &["*"])
            .pick_files();

        if let Some(paths) = picked {
            let _ = proxy.send_event(UserEvent::PickedFiles(paths));
        }
    });
}

fn spawn_folder_picker(proxy: EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        if let Some(dir) = rfd::FileDialog::new()
            .set_title("Open Documentation Folder")
            .pick_folder()
        {
            let _ = proxy.send_event(UserEvent::PickedFolder(dir));
        }
    });
}

fn spawn_save_as_dialog(proxy: EventLoopProxy<UserEvent>, old_path: String, content: String) {
    std::thread::spawn(move || {
        if let Some(new_path) = rfd::FileDialog::new()
            .set_title("Save Markdown File")
            .add_filter("Markdown & Mermaid", &["md", "markdown", "mmd", "mermaid"])
            .set_file_name("Untitled.md")
            .save_file()
        {
            let _ = proxy.send_event(UserEvent::PickedSaveAs {
                old_path,
                new_path,
                content,
            });
        }
    });
}

fn spawn_pdf_save_dialog(
    proxy: EventLoopProxy<UserEvent>,
    suggested_name: String,
    current_path: Option<String>,
    options: printing::PrintOptions,
) {
    std::thread::spawn(move || {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Export PDF")
            .add_filter("PDF Document", &["pdf"])
            .set_file_name(&suggested_name);
        if let Some(dir) = current_path
            .as_deref()
            .and_then(|p| Path::new(p).parent())
            .filter(|d| d.is_dir())
        {
            dialog = dialog.set_directory(dir);
        }
        if let Some(out_path) = dialog.save_file() {
            let _ = proxy.send_event(UserEvent::PickedPdfOutput {
                path: out_path,
                options,
            });
        }
    });
}

fn full_webview_bounds(window: &Window) -> wry::Rect {
    let sz = window.inner_size();
    wry::Rect {
        position: wry::dpi::PhysicalPosition::new(0, 0).into(),
        size: wry::dpi::PhysicalSize::new(sz.width.max(1), sz.height.max(1)).into(),
    }
}

fn is_position_on_any_monitor(event_loop: &EventLoop<UserEvent>, x: i32, y: i32) -> bool {
    event_loop.available_monitors().any(|m| {
        let pos = m.position();
        let sz = m.size();
        x >= pos.x - 60
            && x < pos.x + sz.width as i32 - 60
            && y >= pos.y - 20
            && y < pos.y + sz.height as i32 - 60
    })
}

fn sync_window_state(window: &Window, state: &mut session::WindowState) {
    if window.is_minimized() {
        return;
    }
    let is_max = window.is_maximized();
    state.maximized = is_max;
    if !is_max {
        let size = window.inner_size().to_logical::<f64>(window.scale_factor());
        if size.width >= 680.0 && size.height >= 440.0 {
            state.width = size.width;
            state.height = size.height;
        }
        if let Ok(pos) = window.outer_position()
            && pos.x > -10000
            && pos.y > -10000
        {
            state.x = Some(pos.x);
            state.y = Some(pos.y);
        }
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        if let Ok(dir) = std::env::var("LOCALAPPDATA") {
            let log_dir = PathBuf::from(dir).join("MDViewerByAli");
            let _ = fs::create_dir_all(&log_dir);
            let _ = fs::write(log_dir.join("last-panic.log"), info.to_string());
        }
    }));

    if std::env::args_os().any(|arg| arg == "--register") {
        if let Err(err) = file_assoc::register_current_exe() {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }
    if std::env::args_os().any(|arg| arg == "--unregister") {
        if let Err(err) = file_assoc::unregister() {
            eprintln!("{err}");
            std::process::exit(1);
        }
        return;
    }

    let mut cli_paths: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .filter(|arg| !arg.to_string_lossy().starts_with("--"))
        .map(PathBuf::from)
        .collect();

    let _instance_guard = match single_instance::acquire_or_handoff(&cli_paths) {
        single_instance::StartupRole::Primary(guard) => guard,
        single_instance::StartupRole::HandedOff => return,
    };

    let mut live_session = session::load();
    let initial_session = live_session.clone();

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    single_instance::spawn_ipc_listener(proxy.clone());

    let mut builder = WindowBuilder::new()
        .with_title("MDViewerByAli")
        .with_inner_size(LogicalSize::new(
            initial_session.window.width,
            initial_session.window.height,
        ))
        .with_min_inner_size(LogicalSize::new(680.0, 440.0))
        .with_maximized(initial_session.window.maximized)
        .with_visible(false);

    if let (Some(x), Some(y)) = (initial_session.window.x, initial_session.window.y)
        && is_position_on_any_monitor(&event_loop, x, y)
    {
        builder = builder.with_position(PhysicalPosition::new(x, y));
    }

    let window = builder
        .build(&event_loop)
        .expect("failed to build application window");

    dwm::apply_titlebar_theme(initial_session.dark);
    dwm::apply_window_icon();
    if initial_session.window.maximized {
        window.set_maximized(true);
    }
    window.set_visible(true);
    dwm::paint_loading_splash(initial_session.dark);

    let allowed_dirs: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut web_context = WebContext::new(Some(webview_data_dir()));
    let mut webview: Option<WebView> = None;

    let bg_rgba = if initial_session.dark {
        (0x0D, 0x10, 0x17, 0xFF)
    } else {
        (0xFF, 0xFF, 0xFF, 0xFF)
    };

    let splash_stop = Arc::new(AtomicBool::new(false));
    let splash_stop_thread = Arc::clone(&splash_stop);
    let splash_proxy = proxy.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        if !splash_stop_thread.load(Ordering::Relaxed) {
            let _ = splash_proxy.send_event(UserEvent::InitWebView);
        }
        std::thread::sleep(Duration::from_millis(3000));
        if !splash_stop_thread.load(Ordering::Relaxed) {
            let _ = splash_proxy.send_event(UserEvent::ShowWebView);
        }
    });

    let mut file_watcher = watcher::FileWatcher::new(proxy.clone()).ok();
    let mut last_self_saves: HashMap<String, Instant> = HashMap::new();
    let mut initialized = false;
    let mut webview_visible = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                splash_stop.store(true, Ordering::Relaxed);
                sync_window_state(&window, &mut live_session.window);
                session::save(&live_session);
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                sync_window_state(&window, &mut live_session.window);
                if webview_visible {
                    if let Some(wv) = webview.as_ref() {
                        let _ = wv.set_bounds(full_webview_bounds(&window));
                    }
                } else {
                    dwm::paint_loading_splash(live_session.dark);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Moved(_),
                ..
            } => {
                sync_window_state(&window, &mut live_session.window);
                if !webview_visible {
                    dwm::paint_loading_splash(live_session.dark);
                }
            }
            Event::RedrawRequested(_) => {
                if !webview_visible {
                    dwm::paint_loading_splash(live_session.dark);
                }
            }
            Event::UserEvent(user_event) => match user_event {
                UserEvent::InitWebView => {
                    if webview.is_none() {
                        let allowed_dirs_proto = Arc::clone(&allowed_dirs);
                        let ipc_proxy = proxy.clone();
                        let drop_proxy = proxy.clone();
                        let title_proxy = proxy.clone();

                        let built = WebViewBuilder::new_with_web_context(&mut web_context)
                            .with_visible(true)
                            .with_bounds(wry::Rect {
                                position: wry::dpi::PhysicalPosition::new(0, 0).into(),
                                size: wry::dpi::PhysicalSize::new(1, 1).into(),
                            })
                            .with_background_color(bg_rgba)
                            .with_custom_protocol("mdviewer".into(), move |_webview_id, request| {
                                let uri = request.uri();
                                let path = uri.path();
                                match path {
                                    "/" | "/index.html" => static_response(
                                        inlined_index_html(),
                                        "text/html; charset=utf-8",
                                    ),
                                    "/styles.css" => {
                                        static_response(STYLES_CSS, "text/css; charset=utf-8")
                                    }
                                    "/app.js" => static_response(
                                        APP_JS,
                                        "application/javascript; charset=utf-8",
                                    ),
                                    "/vendor/mermaid.min.js" => static_cached_response(
                                        MERMAID_JS,
                                        "application/javascript; charset=utf-8",
                                    ),
                                    "/vendor/highlight.min.js" => static_cached_response(
                                        HIGHLIGHT_JS,
                                        "application/javascript; charset=utf-8",
                                    ),
                                    "/local-file" => {
                                        handle_local_file(uri.query(), &allowed_dirs_proto)
                                    }
                                    _ if katex_assets::lookup_katex_asset(path).is_some() => {
                                        let (bytes, mime) =
                                            katex_assets::lookup_katex_asset(path).unwrap();
                                        static_cached_response(bytes, mime)
                                    }
                                    _ if font_assets::lookup_font_asset(path).is_some() => {
                                        let (bytes, mime) =
                                            font_assets::lookup_font_asset(path).unwrap();
                                        static_cached_response(bytes, mime)
                                    }
                                    _ => error_response(StatusCode::NOT_FOUND, "Not found"),
                                }
                            })
                            .with_url("http://mdviewer.localhost/index.html")
                            .with_document_title_changed_handler(move |title| {
                                let _ = title_proxy.send_event(UserEvent::TitleChanged(title));
                            })
                            .with_ipc_handler(move |req| {
                                let _ = ipc_proxy.send_event(UserEvent::Ipc(req.body().to_string()));
                            })
                            .with_drag_drop_handler(move |event| match event {
                                DragDropEvent::Enter { .. } | DragDropEvent::Over { .. } => {
                                    let _ = drop_proxy.send_event(UserEvent::DragHover(true));
                                    true
                                }
                                DragDropEvent::Leave => {
                                    let _ = drop_proxy.send_event(UserEvent::DragHover(false));
                                    true
                                }
                                DragDropEvent::Drop { paths, .. } => {
                                    let _ = drop_proxy.send_event(UserEvent::DragHover(false));
                                    let _ = drop_proxy.send_event(UserEvent::DropPaths(paths));
                                    true
                                }
                                _ => false,
                            })
                            .build_as_child(&window)
                            .expect("failed to build WebView2");

                        webview = Some(built);
                    }
                }
                UserEvent::ShowWebView => {
                    if !webview_visible {
                        webview_visible = true;
                        splash_stop.store(true, Ordering::Relaxed);
                        if let Some(wv) = webview.as_ref() {
                            let _ = wv.set_bounds(full_webview_bounds(&window));
                            let _ = wv.focus();
                        }
                    }
                }
                UserEvent::DragHover(active) => {
                    if let Some(wv) = webview.as_ref() {
                        let flag = if active { "true" } else { "false" };
                        let _ = wv.evaluate_script(&format!("window.__MDV_DRAG__({flag});"));
                    }
                }
                UserEvent::TitleChanged(title) => {
                    if !title.is_empty() {
                        window.set_title(&title);
                    }
                }
                UserEvent::DropPaths(paths) | UserEvent::PickedFiles(paths) => {
                    if let Some(wv) = webview.as_ref() {
                        open_file_paths(&paths, wv, &allowed_dirs, &mut file_watcher);
                    }
                }
                UserEvent::ExternalOpen(paths) => {
                    let _keep_alive = &_instance_guard;
                    window.set_minimized(false);
                    window.set_focus();
                    dwm::bring_window_to_front();
                    if !paths.is_empty() {
                        if initialized && let Some(wv) = webview.as_ref() {
                            open_file_paths(&paths, wv, &allowed_dirs, &mut file_watcher);
                        } else {
                            cli_paths.extend(paths);
                        }
                    }
                }
                UserEvent::PickedFolder(dir) => {
                    if let Some(wv) = webview.as_ref()
                        && let Some(tree) = folder::scan_folder(&dir)
                    {
                        push_folder_to_webview(wv, &tree);
                    }
                }
                UserEvent::PickedPdfOutput { path, options } => {
                    if let Some(wv) = webview.as_ref()
                        && let Err(err) =
                            printing::export_to_pdf(wv, path, &options, proxy.clone())
                        && let Ok(msg_json) = serde_json::to_string(&err)
                    {
                        let _ = wv.evaluate_script(&format!(
                            "window.__MDV_PRINT_DONE__(false, {msg_json});"
                        ));
                    }
                }
                UserEvent::PickedSaveAs {
                    old_path,
                    new_path,
                    content,
                } => {
                    if let Some(wv) = webview.as_ref() {
                        let canonical =
                            fs::canonicalize(&new_path).unwrap_or_else(|_| new_path.clone());
                        if let Err(err) = fs::write(&canonical, &content) {
                            if let (Ok(op), Ok(e)) = (
                                serde_json::to_string(&old_path),
                                serde_json::to_string(&err.to_string()),
                            ) {
                                let _ = wv.evaluate_script(&format!(
                                    "window.__MDV_FILE_SAVED__({op}, null, false, {e});"
                                ));
                            }
                        } else {
                            if let Some(parent) = canonical.parent()
                                && let Ok(mut dirs) = allowed_dirs.lock()
                            {
                                dirs.insert(parent.to_path_buf());
                            }
                            if let Some(w) = file_watcher.as_mut() {
                                w.watch(&canonical);
                            }
                            last_self_saves.insert(
                                folder::normalize_path_str(&canonical).to_ascii_lowercase(),
                                Instant::now(),
                            );
                            let rendered = markdown::render_document(&content, &canonical);
                            if let (Ok(op), Ok(doc_json)) = (
                                serde_json::to_string(&old_path),
                                serde_json::to_string(&rendered),
                            ) {
                                let _ = wv.evaluate_script(&format!(
                                    "window.__MDV_FILE_SAVED__({op}, {doc_json}, true, null);"
                                ));
                            }
                        }
                    }
                }
                UserEvent::PdfExported { path, success } => {
                    if let Some(wv) = webview.as_ref() {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("document.pdf");
                        let msg = if success {
                            format!("Exported PDF: {name}")
                        } else {
                            format!("Failed to export PDF: {name}")
                        };
                        if let Ok(msg_json) = serde_json::to_string(&msg) {
                            let is_ok = if success { "true" } else { "false" };
                            let _ = wv.evaluate_script(&format!(
                                "window.__MDV_PRINT_DONE__({is_ok}, {msg_json});"
                            ));
                        }
                    }
                }
                UserEvent::PrintCompleted { success, message } => {
                    if let Some(wv) = webview.as_ref()
                        && let Ok(msg_json) = serde_json::to_string(&message)
                    {
                        let is_ok = if success { "true" } else { "false" };
                        let _ = wv.evaluate_script(&format!(
                            "window.__MDV_PRINT_DONE__({is_ok}, {msg_json});"
                        ));
                    }
                }
                UserEvent::FileChanged(path) => {
                    let key = folder::normalize_path_str(&path).to_ascii_lowercase();
                    if let Some(t) = last_self_saves.get(&key)
                        && t.elapsed() < Duration::from_millis(1200)
                    {
                        return;
                    }
                    if let Some(wv) = webview.as_ref()
                        && let Ok(content) = fs::read_to_string(&path)
                    {
                        let rendered = markdown::render_document(&content, &path);
                        push_doc_to_webview(wv, "__MDV_UPDATE_DOC__", &rendered);
                    }
                }
                UserEvent::Ipc(raw) => {
                    let Ok(msg) = serde_json::from_str::<IpcMessage>(&raw) else {
                        return;
                    };
                    match msg {
                        IpcMessage::Ready => {
                            if let Some(wv) = webview.as_ref() {
                                if !initialized {
                                    initialized = true;
                                    let payload = build_restored_payload(
                                        &initial_session,
                                        &allowed_dirs,
                                        &mut file_watcher,
                                    );
                                    if let Ok(json) = serde_json::to_string(&payload) {
                                        let _ = wv.evaluate_script(&format!(
                                            "window.__MDV_RESTORE_SESSION__({json});"
                                        ));
                                    }
                                    if !cli_paths.is_empty() {
                                        open_file_paths(
                                            &cli_paths,
                                            wv,
                                            &allowed_dirs,
                                            &mut file_watcher,
                                        );
                                    }
                                }
                                let _ = wv.evaluate_script("window.__MDV_HIDE_LOADER__();");
                                if !webview_visible {
                                    webview_visible = true;
                                    splash_stop.store(true, Ordering::Relaxed);
                                    let _ = wv.set_bounds(full_webview_bounds(&window));
                                    let _ = wv.focus();
                                }
                            }
                        }
                        IpcMessage::OpenDialog => {
                            spawn_file_picker(proxy.clone());
                        }
                        IpcMessage::OpenFolderDialog => {
                            spawn_folder_picker(proxy.clone());
                        }
                        IpcMessage::OpenFilePath { path } => {
                            if let Some(wv) = webview.as_ref() {
                                open_file_paths(
                                    &[PathBuf::from(path)],
                                    wv,
                                    &allowed_dirs,
                                    &mut file_watcher,
                                );
                            }
                        }
                        IpcMessage::RefreshFolder { path } => {
                            if let Some(wv) = webview.as_ref()
                                && let Some(tree) = folder::scan_folder(Path::new(&path))
                            {
                                push_folder_to_webview(wv, &tree);
                            }
                        }
                        IpcMessage::OpenDemo => {
                            if let Some(wv) = webview.as_ref() {
                                let demo = markdown::render_document(
                                    markdown::demo_markdown(),
                                    Path::new("Welcome-Demo.md"),
                                );
                                push_doc_to_webview(wv, "__MDV_OPEN_DOC__", &demo);
                            }
                        }
                        IpcMessage::CloseTab { path } => {
                            if let Some(w) = file_watcher.as_mut() {
                                w.unwatch(Path::new(&path));
                            }
                        }
                        IpcMessage::SetTheme { dark } => {
                            dwm::apply_titlebar_theme(dark);
                        }
                        IpcMessage::OpenExternal { url } => {
                            dwm::open_external_url(&url);
                        }
                        IpcMessage::GetPrinters => {
                            if let Some(wv) = webview.as_ref() {
                                let info = printing::list_printers();
                                if let Ok(json) = serde_json::to_string(&info) {
                                    let _ = wv.evaluate_script(&format!(
                                        "window.__MDV_SET_PRINTERS__({json});"
                                    ));
                                }
                            }
                        }
                        IpcMessage::RenderLivePreview { path, content } => {
                            if let Some(wv) = webview.as_ref() {
                                let rendered = markdown::render_document(&content, Path::new(&path));
                                push_doc_to_webview(wv, "__MDV_LIVE_PREVIEW__", &rendered);
                            }
                        }
                        IpcMessage::SaveFile { path, content } => {
                            if let Some(wv) = webview.as_ref() {
                                if path.is_empty() || path == "Welcome-Demo.md" {
                                    spawn_save_as_dialog(proxy.clone(), path, content);
                                } else {
                                    let target = PathBuf::from(&path);
                                    if let Err(err) = fs::write(&target, &content) {
                                        if let (Ok(op), Ok(e)) = (
                                            serde_json::to_string(&path),
                                            serde_json::to_string(&err.to_string()),
                                        ) {
                                            let _ = wv.evaluate_script(&format!(
                                                "window.__MDV_FILE_SAVED__({op}, null, false, {e});"
                                            ));
                                        }
                                    } else {
                                        last_self_saves.insert(
                                            folder::normalize_path_str(&target).to_ascii_lowercase(),
                                            Instant::now(),
                                        );
                                        let rendered = markdown::render_document(&content, &target);
                                        if let (Ok(op), Ok(doc_json)) = (
                                            serde_json::to_string(&path),
                                            serde_json::to_string(&rendered),
                                        ) {
                                            let _ = wv.evaluate_script(&format!(
                                                "window.__MDV_FILE_SAVED__({op}, {doc_json}, true, null);"
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        IpcMessage::PrintDocument { options } => {
                            if let Some(wv) = webview.as_ref()
                                && let Err(err) =
                                    printing::print_silently(wv, &options, proxy.clone())
                                && let Ok(msg_json) = serde_json::to_string(&err)
                            {
                                let _ = wv.evaluate_script(&format!(
                                    "window.__MDV_PRINT_DONE__(false, {msg_json});"
                                ));
                            }
                        }
                        IpcMessage::ExportPdf {
                            suggested_name,
                            current_path,
                            options,
                        } => {
                            spawn_pdf_save_dialog(
                                proxy.clone(),
                                suggested_name,
                                current_path,
                                options,
                            );
                        }
                        IpcMessage::OpenRelativeLink { current_path, href } => {
                            if let Some(wv) = webview.as_ref() {
                                let raw_rel = href.split(['#', '?']).next().unwrap_or("");
                                let decoded = url_decode(raw_rel);
                                let trimmed = decoded.trim();
                                if trimmed.is_empty() {
                                    return;
                                }
                                let candidate = Path::new(trimmed);
                                let resolved = if candidate.is_absolute() {
                                    candidate.to_path_buf()
                                } else if let Some(parent) = Path::new(&current_path).parent() {
                                    parent.join(candidate)
                                } else {
                                    return;
                                };
                                if resolved.is_dir() {
                                    if let Some(tree) = folder::scan_folder(&resolved) {
                                        push_folder_to_webview(wv, &tree);
                                    }
                                } else if resolved.is_file()
                                    && markdown::is_supported_file(&resolved)
                                {
                                    open_file_paths(
                                        &[resolved],
                                        wv,
                                        &allowed_dirs,
                                        &mut file_watcher,
                                    );
                                }
                            }
                        }
                        IpcMessage::SaveSession { mut session } => {
                            sync_window_state(&window, &mut live_session.window);
                            session.window = live_session.window.clone();
                            live_session = session;
                            session::save(&live_session);
                        }
                    }
                }
            },
            _ => {}
        }
    });
}
