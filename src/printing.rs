use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tao::event_loop::EventLoopProxy;
use wry::WebView;

use crate::UserEvent;

#[derive(Debug, Clone, Deserialize)]
pub struct PrintOptions {
    #[serde(default)]
    pub destination: String,
    #[serde(default = "default_paper")]
    pub paper_size: String,
    #[serde(default)]
    pub landscape: bool,
    #[serde(default = "default_scale")]
    pub scale: u32,
    #[serde(default = "default_copies")]
    pub copies: i32,
    #[serde(default)]
    pub page_ranges: String,
}

fn default_paper() -> String {
    "a4".to_string()
}

fn default_scale() -> u32 {
    100
}

fn default_copies() -> i32 {
    1
}

#[derive(Debug, Clone, Serialize)]
pub struct PrintersInfo {
    pub default_printer: Option<String>,
    pub printers: Vec<String>,
}

pub fn paper_dimensions_inches(paper_size: &str) -> (f64, f64) {
    match paper_size.to_ascii_lowercase().as_str() {
        "letter" => (8.5, 11.0),
        "legal" => (8.5, 14.0),
        "a3" => (11.69, 16.54),
        _ => (8.27, 11.69), // A4 default
    }
}

#[cfg(windows)]
pub fn list_printers() -> PrintersInfo {
    use std::collections::BTreeSet;
    use windows::Win32::Graphics::Printing::{
        EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
        PRINTER_INFO_4W,
    };
    use windows::core::{PCWSTR, PWSTR};

    let mut default_printer = None;
    let mut def_len = 0u32;
    unsafe {
        let _ = GetDefaultPrinterW(None, &mut def_len);
    }
    if def_len > 1 {
        let mut buf = vec![0u16; def_len as usize];
        if unsafe { GetDefaultPrinterW(Some(PWSTR(buf.as_mut_ptr())), &mut def_len) }.as_bool() {
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let name = String::from_utf16_lossy(&buf[..end]).trim().to_string();
            if !name.is_empty() {
                default_printer = Some(name);
            }
        }
    }

    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    let mut needed = 0u32;
    let mut returned = 0u32;
    unsafe {
        let _ = EnumPrintersW(flags, PCWSTR::null(), 4, None, &mut needed, &mut returned);
    }

    let mut set = BTreeSet::new();
    if let Some(ref def) = default_printer {
        set.insert(def.clone());
    }

    if needed > 0 {
        let mut buf = vec![0u8; needed as usize];
        if unsafe {
            EnumPrintersW(
                flags,
                PCWSTR::null(),
                4,
                Some(&mut buf),
                &mut needed,
                &mut returned,
            )
        }
        .is_ok()
        {
            let count = returned as usize;
            let max_fit = buf.len() / std::mem::size_of::<PRINTER_INFO_4W>();
            let slice = unsafe {
                std::slice::from_raw_parts(buf.as_ptr() as *const PRINTER_INFO_4W, count.min(max_fit))
            };
            for info in slice {
                if !info.pPrinterName.is_null() {
                    let s = unsafe { info.pPrinterName.to_string() }.unwrap_or_default();
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        set.insert(trimmed.to_string());
                    }
                }
            }
        }
    }

    let mut printers: Vec<String> = set.into_iter().collect();
    printers.sort_by_key(|a| a.to_ascii_lowercase());

    PrintersInfo {
        default_printer,
        printers,
    }
}

#[cfg(not(windows))]
pub fn list_printers() -> PrintersInfo {
    PrintersInfo {
        default_printer: None,
        printers: Vec::new(),
    }
}

#[cfg(windows)]
fn build_print_settings(
    webview: &WebView,
    opts: &PrintOptions,
    include_printer: bool,
) -> Result<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2PrintSettings, String> {
    use std::os::windows::ffi::OsStrExt;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE, COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT,
        ICoreWebView2Environment6, ICoreWebView2PrintSettings2,
    };
    use windows::core::{Interface, PCWSTR};
    use wry::WebViewExtWindows;

    let env6: ICoreWebView2Environment6 = webview
        .environment()
        .cast()
        .map_err(|e| format!("WebView2 environment unavailable: {e}"))?;

    let settings = unsafe { env6.CreatePrintSettings() }
        .map_err(|e| format!("CreatePrintSettings failed: {e}"))?;

    let (w, h) = paper_dimensions_inches(&opts.paper_size);
    let orientation = if opts.landscape {
        COREWEBVIEW2_PRINT_ORIENTATION_LANDSCAPE
    } else {
        COREWEBVIEW2_PRINT_ORIENTATION_PORTRAIT
    };
    let scale = (opts.scale.clamp(50, 150) as f64) / 100.0;

    unsafe {
        let _ = settings.SetShouldPrintBackgrounds(true);
        let _ = settings.SetShouldPrintHeaderAndFooter(false);
        let _ = settings.SetMarginTop(0.0);
        let _ = settings.SetMarginBottom(0.0);
        let _ = settings.SetMarginLeft(0.0);
        let _ = settings.SetMarginRight(0.0);
        let _ = settings.SetPageWidth(w);
        let _ = settings.SetPageHeight(h);
        let _ = settings.SetOrientation(orientation);
        let _ = settings.SetScaleFactor(scale);
    }

    if let Ok(s2) = settings.cast::<ICoreWebView2PrintSettings2>() {
        let ranges = opts.page_ranges.trim();
        if !ranges.is_empty() {
            let wide: Vec<u16> = std::ffi::OsStr::new(ranges)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                let _ = s2.SetPageRanges(PCWSTR(wide.as_ptr()));
            }
        }
        if include_printer {
            unsafe {
                let _ = s2.SetCopies(opts.copies.clamp(1, 99));
            }
            let dest = opts.destination.trim();
            if !dest.is_empty() && dest != "__pdf__" {
                let wide: Vec<u16> = std::ffi::OsStr::new(dest)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                unsafe {
                    let _ = s2.SetPrinterName(PCWSTR(wide.as_ptr()));
                }
            }
        }
    }

    Ok(settings)
}

#[cfg(windows)]
fn print_to_pdf_file<F>(
    webview: &WebView,
    output_path: PathBuf,
    opts: &PrintOptions,
    on_done: F,
) -> Result<(), String>
where
    F: FnOnce(bool) + Send + 'static,
{
    use std::os::windows::ffi::OsStrExt;
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_7;
    use webview2_com::PrintToPdfCompletedHandler;
    use windows::core::{Interface, PCWSTR};
    use wry::WebViewExtWindows;

    let wv7: ICoreWebView2_7 = webview
        .webview()
        .cast()
        .map_err(|e| format!("WebView2 PrintToPdf unavailable: {e}"))?;
    let settings = build_print_settings(webview, opts, false)?;

    let wide: Vec<u16> = output_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let handler = PrintToPdfCompletedHandler::create(Box::new(move |error_code, is_successful| {
        let ok = error_code.is_ok() && bool::from(is_successful);
        on_done(ok);
        Ok(())
    }));

    unsafe { wv7.PrintToPdf(PCWSTR(wide.as_ptr()), &settings, &handler) }
        .map_err(|e| format!("PrintToPdf failed: {e}"))
}

#[cfg(windows)]
pub fn export_to_pdf(
    webview: &WebView,
    output_path: PathBuf,
    opts: &PrintOptions,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<(), String> {
    let done_path = output_path.clone();
    print_to_pdf_file(webview, output_path, opts, move |success| {
        let _ = proxy.send_event(UserEvent::PdfExported {
            path: done_path,
            success,
        });
    })
}

#[cfg(windows)]
pub fn print_silently(
    webview: &WebView,
    opts: &PrintOptions,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<(), String> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PRINT_STATUS_PRINTER_UNAVAILABLE, COREWEBVIEW2_PRINT_STATUS_SUCCEEDED,
        ICoreWebView2_16,
    };
    use webview2_com::PrintCompletedHandler;
    use windows::core::Interface;
    use wry::WebViewExtWindows;

    let wv16: ICoreWebView2_16 = webview
        .webview()
        .cast()
        .map_err(|e| format!("WebView2 silent print unavailable: {e}"))?;
    let settings = build_print_settings(webview, opts, true)?;
    let dest_label = opts.destination.trim().to_string();

    let handler = PrintCompletedHandler::create(Box::new(move |error_code, status| {
        let (success, message) = if error_code.is_ok()
            && status == COREWEBVIEW2_PRINT_STATUS_SUCCEEDED
        {
            if dest_label.is_empty() {
                (true, "Sent document to printer".to_string())
            } else {
                (true, format!("Sent to printer: {dest_label}"))
            }
        } else if status == COREWEBVIEW2_PRINT_STATUS_PRINTER_UNAVAILABLE {
            (false, "Selected printer is unavailable".to_string())
        } else {
            (false, "Printing failed".to_string())
        };
        let _ = proxy.send_event(UserEvent::PrintCompleted { success, message });
        Ok(())
    }));

    unsafe { wv16.Print(&settings, &handler) }.map_err(|e| format!("Print failed: {e}"))
}

#[cfg(not(windows))]
pub fn export_to_pdf(
    _webview: &WebView,
    output_path: PathBuf,
    _opts: &PrintOptions,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<(), String> {
    let _ = proxy.send_event(UserEvent::PdfExported {
        path: output_path,
        success: false,
    });
    Ok(())
}

#[cfg(not(windows))]
pub fn print_silently(
    _webview: &WebView,
    _opts: &PrintOptions,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<(), String> {
    let _ = proxy.send_event(UserEvent::PrintCompleted {
        success: false,
        message: "Unsupported platform".to_string(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_paper_dimensions_in_inches() {
        assert_eq!(paper_dimensions_inches("a4"), (8.27, 11.69));
        assert_eq!(paper_dimensions_inches("Letter"), (8.5, 11.0));
        assert_eq!(paper_dimensions_inches("legal"), (8.5, 14.0));
        assert_eq!(paper_dimensions_inches("A3"), (11.69, 16.54));
    }

    #[test]
    fn deserializes_print_options_with_defaults() {
        let json = r#"{"destination":"__pdf__","landscape":true}"#;
        let opts: PrintOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.destination, "__pdf__");
        assert!(opts.landscape);
        assert_eq!(opts.paper_size, "a4");
        assert_eq!(opts.scale, 100);
        assert_eq!(opts.copies, 1);
        assert_eq!(opts.page_ranges, "");
    }
}
