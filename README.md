# MDViewerByAli

A fast, 100% offline Windows desktop **Markdown**, **KaTeX Math**, and **Mermaid** viewer built with Rust (`tao` + `wry` WebView2 + `pulldown-cmark`). Ships as a single standalone `.exe` with zero runtime dependencies.

## Key Features

- **100% Offline & Single Binary** — Embedded Markdown parser (`pulldown-cmark`), syntax highlighting (`highlight.js`), Mermaid diagrams (`mermaid.js`), and offline **KaTeX** (`$inline$` and `$$display$$`) math rendering with all fonts bundled.
- **Unlimited File Size & Fast Startup** — Instant native splash reveal, deferred lazy asset loading, and smooth rendering for large technical documents.
- **Native BiDi & Arabic Support** — Per-block `dir="auto"` and `unicode-bidi: plaintext` so Arabic, English, and mixed RTL/LTR documents read naturally.
- **Multi-Tab Workspace & Session Restore** — Unlimited tabs with horizontal scrolling, scroll position memory, window size/maximized state persistence, and clean empty-tab start.
- **File Tree & Outline Sidebar** — Toggleable sidebar switching between **Outline** (interactive ToC with scroll sync) and **Files** (`Ctrl+Shift+O` folder browser with live filter, collapsible directories, and drag-and-drop folders).
- **Rich Markdown & Table UX** — GitHub Alerts/Callouts (`[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`), sticky table headers (`position: sticky`), bare URL autolinking, internal `#anchor` & relative `.md` link navigation.
- **Interactive Zoom Lightbox** — Click any Mermaid diagram or image to open a full-screen pan/zoom lightbox.
- **Live Reload & PDF Export** — Automatic reload on external file edits (`notify`), Dark/Light themes with Windows DWM titlebar sync, and clean PDF export (`Ctrl+P`).
- **Windows File Associations** — Dedicated icons and Open With registration for `.md`, `.markdown`, `.mmd`, and `.mermaid`.

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + O` | Open file(s) |
| `Ctrl + Shift + O` | Open folder in File Tree |
| `Ctrl + T` | New tab |
| `Ctrl + W` | Close current tab |
| `Ctrl + B` | Toggle Sidebar (Outline / Files) |
| `Ctrl + F` | Find in document |
| `Ctrl + P` | Print / Export PDF |
| `Ctrl + +` / `Ctrl + -` / `Ctrl + 0` | Zoom in / out / reset |

## Download & Install

Download the latest Windows installer (`MDViewerByAli-0.2.0-Setup.exe`) from the [Releases](https://github.com/aliteleb/MDViewerByAli/releases) page.

## Build from Source

### Prerequisites
- Windows 10/11 (x64) with Microsoft Edge WebView2 Runtime (pre-installed on Windows 10/11)
- [Rust toolchain](https://rustup.rs/) (`edition = "2024"`) & Windows 10/11 SDK (`rc.exe`)
- Optional: [Inno Setup 6](https://jrsoftware.org/isinfo.php) to build the Windows installer

### Commands

```powershell
# Run unit tests
cargo test

# Build release standalone binary (target\release\mdviewer-by-ali.exe)
cargo build --release

# Or run tests + release build + Inno Setup installer (dist\MDViewerByAli-0.2.0-Setup.exe)
.\scripts\build-installer.ps1
```

## License

Licensed under the [MIT License](LICENSE).
