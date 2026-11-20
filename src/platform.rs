/// Set process awareness before creating windows or capture workers. WGC uses
/// physical frame bounds; mixing these with DPI-virtualized sizes times out.
pub fn prepare_dpi() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::*;
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub struct AppInstance {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}
impl AppInstance {
    /// Reopening the EXE restores the existing coach instead of creating a
    /// second capture worker and overlapping arrow windows.
    pub fn acquire() -> Option<Self> {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::{
                Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*,
            };
            let name: Vec<u16> = "Local\\RbChessAssistDesktop"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
            if !handle.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
                let title: Vec<u16> = "RbChessAssist".encode_utf16().chain(Some(0)).collect();
                let window = FindWindowW(std::ptr::null(), title.as_ptr());
                if !window.is_null() {
                    ShowWindow(window, SW_RESTORE);
                    SetForegroundWindow(window);
                }
                CloseHandle(handle);
                return None;
            }
            Some(Self { handle })
        }
        #[cfg(not(windows))]
        Some(Self {})
    }
}
impl Drop for AppInstance {
    fn drop(&mut self) {
        #[cfg(windows)]
        if !self.handle.is_null() {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}
