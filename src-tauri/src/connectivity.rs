use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    NetworkOnline,
    NetworkOffline,
}

pub struct OnlineState(bool);
pub fn map_connectivity(flags: u32) -> bool {
    flags != 0
}
impl OnlineState {
    pub fn new(online: bool) -> Self {
        Self(online)
    }
    pub fn update(&mut self, online: bool) -> Option<SystemEvent> {
        if self.0 == online {
            return None;
        }
        self.0 = online;
        Some(if online {
            SystemEvent::NetworkOnline
        } else {
            SystemEvent::NetworkOffline
        })
    }
}

#[cfg(target_os = "windows")]
mod windows_source {
    use super::*;
    use std::{ffi::c_void, ptr::null_mut};
    use windows::core::{IUnknown, Interface, GUID};
    use windows::Win32::Networking::NetworkListManager::{INetworkListManager, NetworkListManager};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IConnectionPoint,
        IConnectionPointContainer, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    const IID_NLM_EVENTS: GUID = GUID::from_u128(0xdcb00001_570f_4a9b_8d69_199fdba5723b);
    const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_c000_000000000046);

    struct SinkState {
        refs: std::sync::atomic::AtomicU32,
        sender: tokio::sync::mpsc::Sender<SystemEvent>,
        state: Mutex<OnlineState>,
        enabled: Arc<AtomicBool>,
    }
    #[repr(C)]
    struct Sink {
        vtable: *const SinkVtable,
        state: SinkState,
    }
    #[repr(C)]
    struct SinkVtable {
        query: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        connectivity_changed: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    }
    unsafe extern "system" fn query(
        this: *mut c_void,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> i32 {
        if iid.is_null() || out.is_null() {
            return -2147467261_i32;
        }
        if *iid == IID_IUNKNOWN || *iid == IID_NLM_EVENTS {
            *out = this;
            add_ref(this);
            0
        } else {
            *out = null_mut();
            -2147467262_i32
        }
    }
    unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
        let sink = &*(this as *mut Sink);
        sink.state.refs.fetch_add(1, Ordering::Relaxed) + 1
    }
    unsafe extern "system" fn release(this: *mut c_void) -> u32 {
        let sink = &*(this as *mut Sink);
        let count = sink.state.refs.fetch_sub(1, Ordering::Release) - 1;
        if count == 0 {
            std::sync::atomic::fence(Ordering::Acquire);
            drop(Box::from_raw(this as *mut Sink));
        }
        count
    }
    unsafe extern "system" fn connectivity_changed(this: *mut c_void, connectivity: u32) -> i32 {
        let sink = &*(this as *mut Sink);
        if sink.state.enabled.load(Ordering::Acquire) {
            let online = map_connectivity(connectivity);
            if let Some(event) = sink
                .state
                .state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .update(online)
            {
                let _ = sink.state.sender.blocking_send(event);
            }
        }
        0
    }
    static VTABLE: SinkVtable = SinkVtable {
        query,
        add_ref,
        release,
        connectivity_changed,
    };

    struct AdviseGuard {
        point: IConnectionPoint,
        cookie: u32,
        _sink: IUnknown,
    }
    impl Drop for AdviseGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = self.point.Unadvise(self.cookie);
            }
        }
    }

    pub fn subscribe(
        enabled: Arc<AtomicBool>,
        sender: tokio::sync::mpsc::Sender<SystemEvent>,
        shutdown: std::sync::mpsc::Receiver<()>,
    ) -> Result<(), String> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| e.to_string())?;
            struct ComGuard;
            impl Drop for ComGuard {
                fn drop(&mut self) {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
            let _com = ComGuard;
            let manager: INetworkListManager =
                CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL)
                    .map_err(|e| e.to_string())?;
            let container: IConnectionPointContainer = manager.cast().map_err(|e| e.to_string())?;
            let point = container
                .FindConnectionPoint(&IID_NLM_EVENTS)
                .map_err(|e| e.to_string())?;
            let sink = Box::new(Sink {
                vtable: &VTABLE,
                state: SinkState {
                    refs: std::sync::atomic::AtomicU32::new(1),
                    sender,
                    state: Mutex::new(OnlineState::new(true)),
                    enabled,
                },
            });
            let raw = Box::into_raw(sink) as *mut c_void;
            let unknown = IUnknown::from_raw(raw);
            let cookie = match point.Advise(&unknown) {
                Ok(value) => value,
                Err(error) => {
                    drop(unknown);
                    return Err(error.to_string());
                }
            };
            let _guard = AdviseGuard {
                point,
                cookie,
                _sink: unknown,
            };
            let _ = shutdown.recv();
            Ok(())
        }
    }
}

pub fn start(
    host: String,
    enabled: Arc<AtomicBool>,
    sender: tokio::sync::mpsc::Sender<SystemEvent>,
) -> (
    tauri::async_runtime::JoinHandle<()>,
    std::sync::mpsc::Sender<()>,
) {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (host, enabled, sender);
        return (
            tauri::async_runtime::spawn(async { std::future::pending::<()>().await }),
            std::sync::mpsc::channel().0,
        );
    }
    #[cfg(target_os = "windows")]
    {
        let fallback_enabled = enabled.clone();
        let fallback_sender = sender.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = windows_source::subscribe(enabled, sender, shutdown_rx);
            let _ = ready_tx.send(result.is_ok());
        });
        (
            tauri::async_runtime::spawn(async move {
                let subscribed =
                    tokio::task::spawn_blocking(move || ready_rx.recv().unwrap_or(false))
                        .await
                        .unwrap_or(false);
                if subscribed {
                    std::future::pending::<()>().await;
                }
                let client = reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(2))
                    .timeout(std::time::Duration::from_secs(3))
                    .build()
                    .unwrap_or_default();
                let mut state = OnlineState::new(true);
                loop {
                    if fallback_enabled.load(Ordering::Acquire) {
                        let online = client.get(&host).send().await.is_ok();
                        if let Some(event) = state.update(online) {
                            if fallback_sender.send(event).await.is_err() {
                                break;
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }),
            shutdown_tx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_does_not_require_an_entered_tokio_runtime() {
        let enabled = Arc::new(AtomicBool::new(false));
        let (events, _receiver) = tokio::sync::mpsc::channel(1);

        let (task, shutdown) = start("https://example.invalid".into(), enabled, events);

        let _ = shutdown.send(());
        task.abort();
    }

    #[test]
    fn connectivity_is_edge_triggered() {
        let mut s = OnlineState::new(false);
        assert_eq!(s.update(true), Some(SystemEvent::NetworkOnline));
        assert_eq!(s.update(true), None);
    }
    #[test]
    fn connectivity_flags_map_to_online_state() {
        assert!(map_connectivity(1));
        assert!(!map_connectivity(0));
    }
}
