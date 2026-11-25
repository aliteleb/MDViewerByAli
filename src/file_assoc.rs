//! Per-user (HKCU) Windows file association registration so Windows suggests
//! MDViewerByAli in "Open with" and Default Apps for Markdown & Mermaid files,
//! and applies our document icon.

use std::path::Path;

pub fn normalize_exe_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
}

pub fn exe_filename(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("MDViewerByAli.exe")
        .to_string()
}

#[cfg(windows)]
mod win {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::{
        Win32::{
            Foundation::{ERROR_SUCCESS, WIN32_ERROR},
            System::Registry::{
                HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, KEY_WRITE,
                REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteTreeW,
                RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
            },
            UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify},
        },
        core::PCWSTR,
    };

    const PROGID_MARKDOWN: &str = "MDViewerByAli.Markdown";
    const PROGID_MERMAID: &str = "MDViewerByAli.Mermaid";
    const CAPABILITIES_KEY: &str = r"Software\MDViewerByAli\Capabilities";
    const BACKUP_VALUE: &str = "MDViewerByAli_backup";

    fn to_wide_null(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    fn err_msg(op: &str, code: WIN32_ERROR) -> String {
        format!("{op} failed with Win32 error {}", code.0)
    }

    fn read_sz(subkey: &str, value_name: Option<&str>) -> Option<String> {
        let subkey_w = to_wide_null(subkey);
        let mut hkey = HKEY::default();
        let res = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                None,
                KEY_READ,
                &mut hkey,
            )
        };
        if res != ERROR_SUCCESS {
            return None;
        }

        let name_w = value_name.map(to_wide_null);
        let name_ptr = name_w
            .as_ref()
            .map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));

        let mut byte_len: u32 = 0;
        let q1 = unsafe {
            RegQueryValueExW(hkey, name_ptr, None, None, None, Some(&mut byte_len))
        };
        if q1 != ERROR_SUCCESS || byte_len == 0 {
            unsafe {
                let _ = RegCloseKey(hkey);
            }
            return None;
        }

        let mut buf = vec![0u16; (byte_len as usize).div_ceil(2)];
        let q2 = unsafe {
            RegQueryValueExW(
                hkey,
                name_ptr,
                None,
                None,
                Some(buf.as_mut_ptr().cast::<u8>()),
                Some(&mut byte_len),
            )
        };
        unsafe {
            let _ = RegCloseKey(hkey);
        }

        if q2 != ERROR_SUCCESS {
            return None;
        }
        let u16_len = (byte_len as usize / 2).min(buf.len());
        Some(
            String::from_utf16_lossy(&buf[..u16_len])
                .trim_end_matches('\0')
                .to_string(),
        )
    }

    fn set_sz(subkey: &str, value_name: Option<&str>, data: &str) -> Result<(), String> {
        let subkey_w = to_wide_null(subkey);
        let mut hkey = HKEY::default();
        let res = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            )
        };
        if res != ERROR_SUCCESS {
            return Err(err_msg(&format!("RegCreateKeyExW({subkey})"), res));
        }

        let wide_data = to_wide_null(data);
        let bytes = unsafe {
            std::slice::from_raw_parts(wide_data.as_ptr().cast::<u8>(), wide_data.len() * 2)
        };
        let name_w = value_name.map(to_wide_null);
        let name_ptr = name_w
            .as_ref()
            .map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));

        let write_res = unsafe { RegSetValueExW(hkey, name_ptr, None, REG_SZ, Some(bytes)) };
        unsafe {
            let _ = RegCloseKey(hkey);
        }

        if write_res != ERROR_SUCCESS {
            return Err(err_msg(&format!("RegSetValueExW({subkey})"), write_res));
        }
        Ok(())
    }

    fn delete_value(subkey: &str, value_name: Option<&str>) {
        let subkey_w = to_wide_null(subkey);
        let mut hkey = HKEY::default();
        let res = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                None,
                KEY_SET_VALUE,
                &mut hkey,
            )
        };
        if res == ERROR_SUCCESS {
            let val_w = value_name.map(to_wide_null);
            let val_ptr = val_w
                .as_ref()
                .map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));
            unsafe {
                let _ = RegDeleteValueW(hkey, val_ptr);
                let _ = RegCloseKey(hkey);
            }
        }
    }

    fn delete_tree(subkey: &str) {
        let subkey_w = to_wide_null(subkey);
        unsafe {
            let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(subkey_w.as_ptr()));
        }
    }

    fn register_progid(progid: &str, type_label: &str, exe: &str) -> Result<(), String> {
        let base = format!(r"Software\Classes\{progid}");
        // Icon index 1 is embedded resource `2 ICON "md-doc.ico"`
        let doc_icon = format!("\"{exe}\",1");
        let cmd = format!("\"{exe}\" \"%1\"");

        set_sz(&base, None, type_label)?;
        set_sz(&base, Some("FriendlyTypeName"), type_label)?;
        set_sz(&format!(r"{base}\DefaultIcon"), None, &doc_icon)?;
        set_sz(&format!(r"{base}\shell\open"), Some("FriendlyAppName"), "MDViewerByAli")?;
        set_sz(&format!(r"{base}\shell\open\command"), None, &cmd)?;
        Ok(())
    }

    pub fn register_current_exe() -> Result<(), String> {
        let current = std::env::current_exe().map_err(|e| format!("current_exe failed: {e}"))?;
        let exe = super::normalize_exe_path(&current);
        let exe_name = super::exe_filename(&current);
        let app_key = format!(r"Software\Classes\Applications\{exe_name}");
        // Icon index 0 is embedded resource `1 ICON "icon.ico"`
        let app_icon = format!("\"{exe}\",0");
        let cmd = format!("\"{exe}\" \"%1\"");

        register_progid(PROGID_MARKDOWN, "Markdown Document", &exe)?;
        register_progid(PROGID_MERMAID, "Mermaid Diagram", &exe)?;

        set_sz(&app_key, Some("FriendlyAppName"), "MDViewerByAli")?;
        set_sz(&format!(r"{app_key}\DefaultIcon"), None, &app_icon)?;
        set_sz(
            &format!(r"{app_key}\shell\open"),
            Some("FriendlyAppName"),
            "MDViewerByAli",
        )?;
        set_sz(&format!(r"{app_key}\shell\open\command"), None, &cmd)?;

        for (ext, progid, content_type) in [
            (".md", PROGID_MARKDOWN, Some("text/markdown")),
            (".markdown", PROGID_MARKDOWN, Some("text/markdown")),
            (".mmd", PROGID_MERMAID, None),
            (".mermaid", PROGID_MERMAID, None),
        ] {
            let ext_key = format!(r"Software\Classes\{ext}");
            if let Some(prev) = read_sz(&ext_key, None)
                && !prev.is_empty()
                && !prev.starts_with("MDViewerByAli.")
                && read_sz(&ext_key, Some(BACKUP_VALUE)).is_none()
            {
                let _ = set_sz(&ext_key, Some(BACKUP_VALUE), &prev);
            }

            set_sz(&ext_key, None, progid)?;
            set_sz(&ext_key, Some("PerceivedType"), "text")?;
            if let Some(ct) = content_type {
                set_sz(&ext_key, Some("Content Type"), ct)?;
            }

            set_sz(&format!(r"{app_key}\SupportedTypes"), Some(ext), "")?;
            set_sz(&format!(r"{ext_key}\OpenWithProgids"), Some(progid), "")?;
            set_sz(
                &format!(r"{CAPABILITIES_KEY}\FileAssociations"),
                Some(ext),
                progid,
            )?;
        }

        set_sz(CAPABILITIES_KEY, Some("ApplicationName"), "MDViewerByAli")?;
        set_sz(
            CAPABILITIES_KEY,
            Some("ApplicationDescription"),
            "Offline Markdown & Mermaid Viewer",
        )?;
        set_sz(
            r"Software\RegisteredApplications",
            Some("MDViewerByAli"),
            CAPABILITIES_KEY,
        )?;

        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
        }
        Ok(())
    }

    pub fn unregister() -> Result<(), String> {
        delete_tree(&format!(r"Software\Classes\{PROGID_MARKDOWN}"));
        delete_tree(&format!(r"Software\Classes\{PROGID_MERMAID}"));

        if let Ok(current) = std::env::current_exe() {
            let exe_name = super::exe_filename(&current);
            delete_tree(&format!(r"Software\Classes\Applications\{exe_name}"));
        }
        delete_tree(r"Software\Classes\Applications\mdviewer-by-ali.exe");
        delete_tree(r"Software\Classes\Applications\MDViewerByAli.exe");

        delete_tree(r"Software\MDViewerByAli");
        delete_value(r"Software\RegisteredApplications", Some("MDViewerByAli"));

        for (ext, progid) in [
            (".md", PROGID_MARKDOWN),
            (".markdown", PROGID_MARKDOWN),
            (".mmd", PROGID_MERMAID),
            (".mermaid", PROGID_MERMAID),
        ] {
            let ext_key = format!(r"Software\Classes\{ext}");
            delete_value(&format!(r"{ext_key}\OpenWithProgids"), Some(progid));

            if let Some(cur) = read_sz(&ext_key, None)
                && cur == progid
            {
                if let Some(backup) = read_sz(&ext_key, Some(BACKUP_VALUE)) {
                    let _ = set_sz(&ext_key, None, &backup);
                } else {
                    delete_value(&ext_key, None);
                }
            }
            delete_value(&ext_key, Some(BACKUP_VALUE));
        }

        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None);
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use win::{register_current_exe, unregister};

#[cfg(not(windows))]
pub fn register_current_exe() -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn unregister() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_verbatim_prefix_from_exe_path() {
        let p = Path::new(r"\\?\C:\Apps\MDViewerByAli\MDViewerByAli.exe");
        assert_eq!(
            normalize_exe_path(p),
            r"C:\Apps\MDViewerByAli\MDViewerByAli.exe"
        );
        assert_eq!(exe_filename(p), "MDViewerByAli.exe");
    }
}
