use std::path::PathBuf;

use tao::event_loop::EventLoopProxy;

use crate::UserEvent;

fn sanitize_username(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    if cleaned.is_empty() {
        "default".to_string()
    } else {
        cleaned
    }
}

fn current_user_tag() -> String {
    let raw = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    sanitize_username(&raw)
}

pub fn mutex_name() -> String {
    format!("Local\\MDViewerByAli_SingleInstance_v1_{}", current_user_tag())
}

pub fn pipe_name() -> String {
    format!(r"\\.\pipe\MDViewerByAli-SingleInstance-v1-{}", current_user_tag())
}

#[cfg(windows)]
pub struct InstanceGuard(isize);

#[cfg(windows)]
impl Drop for InstanceGuard {
    fn drop(&mut self) {
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        if self.0 != 0 {
            unsafe {
                let _ = CloseHandle(HANDLE(self.0 as *mut core::ffi::c_void));
            }
        }
    }
}

#[cfg(not(windows))]
pub struct InstanceGuard;

pub enum StartupRole {
    Primary(InstanceGuard),
    HandedOff,
}

fn serialize_paths(paths: &[PathBuf]) -> Option<Vec<u8>> {
    let list: Vec<String> = paths
        .iter()
        .map(|p| {
            let resolved = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
            crate::folder::normalize_path_str(&resolved)
        })
        .collect();
    serde_json::to_vec(&list).ok()
}

fn deserialize_paths(bytes: &[u8]) -> Vec<PathBuf> {
    let Ok(list) = serde_json::from_slice::<Vec<String>>(bytes) else {
        return Vec::new();
    };
    list.into_iter().map(PathBuf::from).collect()
}

#[cfg(windows)]
pub fn acquire_or_handoff(paths: &[PathBuf]) -> StartupRole {
    use std::io::Write;
    use std::os::windows::ffi::OsStrExt;
    use std::time::Duration;
    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow};
    use windows::core::PCWSTR;

    let name = mutex_name();
    let wide: Vec<u16> = std::ffi::OsStr::new(&name)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let handle = match unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr())) } {
        Ok(h) => h,
        Err(_) => return StartupRole::Primary(InstanceGuard(0)),
    };

    let already_exists = unsafe { GetLastError() == ERROR_ALREADY_EXISTS };
    if already_exists {
        unsafe {
            let _ = AllowSetForegroundWindow(ASFW_ANY);
        }
        let payload = serialize_paths(paths).unwrap_or_else(|| b"[]".to_vec());
        let pipe = pipe_name();
        for _ in 0..20 {
            if let Ok(mut file) = std::fs::OpenOptions::new().write(true).open(&pipe)
                && file.write_all(&payload).is_ok()
            {
                let _ = file.flush();
                unsafe {
                    let _ = CloseHandle(handle);
                }
                return StartupRole::HandedOff;
            }
            std::thread::sleep(Duration::from_millis(75));
        }
    }

    StartupRole::Primary(InstanceGuard(handle.0 as isize))
}

#[cfg(not(windows))]
pub fn acquire_or_handoff(_paths: &[PathBuf]) -> StartupRole {
    StartupRole::Primary(InstanceGuard)
}

#[cfg(windows)]
pub fn spawn_ipc_listener(proxy: EventLoopProxy<UserEvent>) {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_CONNECTED, GetLastError, INVALID_HANDLE_VALUE};
    use windows::Win32::Storage::FileSystem::{PIPE_ACCESS_INBOUND, ReadFile};
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
        PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };
    use windows::core::PCWSTR;

    let pipe_str = pipe_name();
    std::thread::spawn(move || {
        let wide: Vec<u16> = std::ffi::OsStr::new(&pipe_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        loop {
            let pipe = unsafe {
                CreateNamedPipeW(
                    PCWSTR(wide.as_ptr()),
                    PIPE_ACCESS_INBOUND,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                    PIPE_UNLIMITED_INSTANCES,
                    4096,
                    4096,
                    0,
                    None,
                )
            };
            if pipe == INVALID_HANDLE_VALUE {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }

            let connected = unsafe {
                ConnectNamedPipe(pipe, None).is_ok() || GetLastError() == ERROR_PIPE_CONNECTED
            };

            if connected {
                let mut total = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let mut read_bytes = 0u32;
                    let ok = unsafe { ReadFile(pipe, Some(&mut buf), Some(&mut read_bytes), None).is_ok() };
                    if !ok || read_bytes == 0 {
                        break;
                    }
                    total.extend_from_slice(&buf[..read_bytes as usize]);
                }
                let paths = deserialize_paths(&total);
                let _ = proxy.send_event(UserEvent::ExternalOpen(paths));
                unsafe {
                    let _ = DisconnectNamedPipe(pipe);
                }
            }

            unsafe {
                let _ = CloseHandle(pipe);
            }
        }
    });
}

#[cfg(not(windows))]
pub fn spawn_ipc_listener(_proxy: EventLoopProxy<UserEvent>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_username_for_pipe_and_mutex_names() {
        assert_eq!(sanitize_username("Ali Teleb-01"), "AliTeleb01");
        assert_eq!(sanitize_username("عربي!@#"), "default");
        assert_eq!(sanitize_username("dev_user_2"), "dev_user_2");
    }

    #[test]
    fn round_trips_handoff_paths() {
        let paths = vec![
            PathBuf::from(r"C:\Docs\readme.md"),
            PathBuf::from(r"C:\Docs\flow.mmd"),
        ];
        let encoded = serialize_paths(&paths).unwrap();
        let decoded = deserialize_paths(&encoded);
        assert_eq!(decoded, paths);
    }
}
