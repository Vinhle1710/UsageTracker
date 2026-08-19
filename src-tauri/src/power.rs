#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    Wake,
}
pub const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;
pub const PBT_APMRESUMESUSPEND: u32 = 0x0007;
pub fn map_power_status(status: u32) -> Option<SystemEvent> {
    matches!(status, PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND).then_some(SystemEvent::Wake)
}
/// Starts a lifecycle-safe Windows power observer. Windows broadcasts resume notifications to a
/// message-only window; no status polling is used because it cannot distinguish a resume event.
pub fn start(sender: std::sync::mpsc::Sender<SystemEvent>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            use std::ptr::null_mut;
            use windows_sys::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
            use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
            use windows_sys::Win32::UI::WindowsAndMessaging::*;
            const CLASS_NAME: &[u16] = &[
                85, 115, 97, 103, 101, 84, 114, 97, 99, 107, 101, 114, 80, 111, 119, 101, 114, 0,
            ];
            unsafe extern "system" fn window_proc(
                hwnd: HWND,
                msg: u32,
                _w: WPARAM,
                l: LPARAM,
            ) -> LRESULT {
                if msg == WM_POWERBROADCAST {
                    let status = l as u32;
                    if let Some(event) = map_power_status(status) {
                        let sender = GetWindowLongPtrW(hwnd, GWLP_USERDATA)
                            as *const std::sync::mpsc::Sender<SystemEvent>;
                        if !sender.is_null() {
                            let _ = (*sender).send(event);
                        }
                    }
                    return 1 as LRESULT;
                }
                DefWindowProcW(hwnd, msg, 0, 0)
            }
            unsafe {
                let instance: HINSTANCE = GetModuleHandleW(null_mut());
                let class = WNDCLASSW {
                    lpfnWndProc: Some(window_proc),
                    hInstance: instance,
                    lpszClassName: CLASS_NAME.as_ptr(),
                    ..std::mem::zeroed()
                };
                if RegisterClassW(&class) == 0 {
                    return;
                }
                let boxed = Box::new(sender);
                let ptr = Box::into_raw(boxed);
                let hwnd = CreateWindowExW(
                    0,
                    CLASS_NAME.as_ptr(),
                    CLASS_NAME.as_ptr(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    null_mut(),
                    instance,
                    null_mut(),
                );
                if hwnd.is_null() {
                    drop(Box::from_raw(ptr));
                    return;
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
                let mut message: MSG = std::mem::zeroed();
                while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                DestroyWindow(hwnd);
                drop(Box::from_raw(ptr));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = sender;
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resume_status_maps_to_wake() {
        assert_eq!(
            map_power_status(PBT_APMRESUMEAUTOMATIC),
            Some(SystemEvent::Wake)
        );
    }
}
