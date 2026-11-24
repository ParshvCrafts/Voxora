//! The window that will receive the text: process name, title, and whether Windows UIPI will
//! block our synthetic input (target elevated, Voxora not).

use std::ffi::c_void;

use voxora_core::context::AppContext;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
use windows::core::PWSTR;

#[derive(Debug, Clone)]
pub struct Target {
    pub ctx: AppContext,
}

pub fn foreground() -> Target {
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, &mut buf).max(0) as usize;
        let title = String::from_utf16_lossy(&buf[..n]);
        let (exe, elevated) = process_info(pid);
        Target { ctx: AppContext { exe, title, elevated: elevated && !self_elevated() } }
    }
}

/// False on the secure desktop / lock screen, where there is nothing to paste into.
pub fn has_foreground() -> bool {
    unsafe { !GetForegroundWindow().0.is_null() }
}

/// (exe file name, elevated). A process we cannot even query is treated as elevated: that is
/// the case UIPI will block.
fn process_info(pid: u32) -> (String, bool) {
    if pid == 0 {
        return (String::new(), false);
    }
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return (String::new(), true);
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let exe = if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            full.rsplit('\\').next().unwrap_or_default().to_string()
        } else {
            String::new()
        };
        let elevated = token_elevated(h).unwrap_or(true);
        let _ = CloseHandle(h);
        (exe, elevated)
    }
}

fn token_elevated(process: HANDLE) -> Option<bool> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut e = TOKEN_ELEVATION::default();
        let mut ret = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut e as *mut _ as *mut c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret,
        );
        let _ = CloseHandle(token);
        ok.ok()?;
        Some(e.TokenIsElevated != 0)
    }
}

pub fn self_elevated() -> bool {
    token_elevated(unsafe { GetCurrentProcess() }).unwrap_or(false)
}
