//! Native Windows DWM titlebar theming, window icon, background brush, and shell helpers.

#[cfg(windows)]
pub fn apply_titlebar_theme(dark: bool) {
    use windows::{
        Win32::{
            Foundation::{COLORREF, HWND, LPARAM},
            Graphics::{
                Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute},
                Gdi::CreateSolidBrush,
            },
            System::Threading::GetCurrentThreadId,
            UI::WindowsAndMessaging::{
                EnumThreadWindows, GCLP_HBRBACKGROUND, SetClassLongPtrW,
            },
        },
        core::BOOL,
    };

    struct ThemeParam {
        dark: bool,
        brush: isize,
    }

    unsafe extern "system" fn apply(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let param = unsafe { &*(lparam.0 as *const ThemeParam) };
        let dark_mode = BOOL(if param.dark { 1 } else { 0 });
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                std::ptr::from_ref(&dark_mode).cast(),
                std::mem::size_of::<BOOL>() as u32,
            );
            SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, param.brush);
        }
        BOOL(1)
    }

    // COLORREF is 0x00BBGGRR: #0D1017 => 0x0017100D
    let color = if dark {
        COLORREF(0x0017100D)
    } else {
        COLORREF(0x00FFFFFF)
    };
    let brush = unsafe { CreateSolidBrush(color) };
    let param = ThemeParam {
        dark,
        brush: brush.0 as isize,
    };

    unsafe {
        let _ = EnumThreadWindows(
            GetCurrentThreadId(),
            Some(apply),
            LPARAM(std::ptr::from_ref(&param) as isize),
        );
    }
}

#[cfg(not(windows))]
pub fn apply_titlebar_theme(_dark: bool) {}

#[cfg(windows)]
pub fn apply_window_icon() {
    use windows::{
        Win32::{
            Foundation::{HINSTANCE, HWND, LPARAM, WPARAM},
            System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
            UI::WindowsAndMessaging::{
                EnumThreadWindows, ICON_BIG, ICON_SMALL, LoadIconW, SendMessageW, WM_SETICON,
            },
        },
        core::PCWSTR,
    };

    let Ok(hmod) = (unsafe { GetModuleHandleW(PCWSTR::null()) }) else {
        return;
    };
    // Resource ID 1 is `1 ICON "icon.ico"` from assets/app/app.rc.
    let Ok(hicon) = (unsafe { LoadIconW(Some(HINSTANCE(hmod.0)), PCWSTR(1 as *const u16)) }) else {
        return;
    };

    unsafe extern "system" fn apply_icon(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
        unsafe {
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_SMALL as usize)), Some(lparam));
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_BIG as usize)), Some(lparam));
        }
        windows::core::BOOL(1)
    }

    unsafe {
        let _ = EnumThreadWindows(
            GetCurrentThreadId(),
            Some(apply_icon),
            LPARAM(hicon.0 as isize),
        );
    }
}

#[cfg(not(windows))]
pub fn apply_window_icon() {}

#[cfg(windows)]
pub fn paint_loading_splash(dark: bool) {
    use windows::{
        Win32::{
            Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, RECT},
            Graphics::Gdi::{
                BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC,
                DeleteObject, FillRect, GetDC, ReleaseDC, SRCCOPY, SelectObject, ValidateRect,
            },
            System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
            UI::WindowsAndMessaging::{
                DI_NORMAL, DestroyIcon, DrawIconEx, EnumThreadWindows, GetClientRect, HICON,
                IMAGE_ICON, IsWindowVisible, LR_DEFAULTCOLOR, LoadImageW,
            },
        },
        core::{BOOL, PCWSTR},
    };

    struct SplashParam {
        bg: COLORREF,
        hicon: isize,
    }

    let Ok(hmod) = (unsafe { GetModuleHandleW(PCWSTR::null()) }) else {
        return;
    };
    let Ok(handle) = (unsafe {
        LoadImageW(
            Some(HINSTANCE(hmod.0)),
            PCWSTR(1 as *const u16),
            IMAGE_ICON,
            64,
            64,
            LR_DEFAULTCOLOR,
        )
    }) else {
        return;
    };

    let param = SplashParam {
        bg: if dark {
            COLORREF(0x0017100D)
        } else {
            COLORREF(0x00FFFFFF)
        },
        hicon: handle.0 as isize,
    };

    unsafe extern "system" fn paint_one(hwnd: HWND, lparam: LPARAM) -> BOOL {
        if unsafe { !IsWindowVisible(hwnd).as_bool() } {
            return BOOL(1);
        }
        let param = unsafe { &*(lparam.0 as *const SplashParam) };
        let mut rect = RECT::default();
        if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
            return BOOL(1);
        }
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if w <= 0 || h <= 0 {
            return BOOL(1);
        }

        let hdc = unsafe { GetDC(Some(hwnd)) };
        if hdc.is_invalid() {
            return BOOL(1);
        }

        let mem_dc = unsafe { CreateCompatibleDC(Some(hdc)) };
        if !mem_dc.is_invalid() {
            let mem_bmp = unsafe { CreateCompatibleBitmap(hdc, w, h) };
            if !mem_bmp.is_invalid() {
                let old_bmp = unsafe { SelectObject(mem_dc, mem_bmp.into()) };
                let bg_brush = unsafe { CreateSolidBrush(param.bg) };
                unsafe {
                    let _ = FillRect(mem_dc, &rect, bg_brush);
                    let _ = DeleteObject(bg_brush.into());

                    let _ = DrawIconEx(
                        mem_dc,
                        (w - 64) / 2,
                        (h - 64) / 2,
                        HICON(param.hicon as *mut core::ffi::c_void),
                        64,
                        64,
                        0,
                        None,
                        DI_NORMAL,
                    );

                    let _ = BitBlt(hdc, 0, 0, w, h, Some(mem_dc), 0, 0, SRCCOPY);
                    SelectObject(mem_dc, old_bmp);
                    let _ = DeleteObject(mem_bmp.into());
                }
            }
            unsafe {
                let _ = DeleteDC(mem_dc);
            }
        }

        unsafe {
            let _ = ReleaseDC(Some(hwnd), hdc);
            let _ = ValidateRect(Some(hwnd), None);
        }
        BOOL(1)
    }

    unsafe {
        let _ = EnumThreadWindows(
            GetCurrentThreadId(),
            Some(paint_one),
            LPARAM(std::ptr::from_ref(&param) as isize),
        );
        let _ = DestroyIcon(HICON(handle.0));
    }
}

#[cfg(not(windows))]
pub fn paint_loading_splash(_dark: bool) {}

#[cfg(windows)]
pub fn open_external_url(url: &str) {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !(lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("mailto:"))
    {
        return;
    }

    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
        core::PCWSTR,
    };

    let verb: Vec<u16> = "open\0".encode_utf16().collect();
    let target: Vec<u16> = std::ffi::OsStr::new(trimmed)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let _ = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(not(windows))]
pub fn open_external_url(_url: &str) {}
