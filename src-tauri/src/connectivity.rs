#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    NetworkOnline,
    NetworkOffline,
}
pub struct OnlineState(bool);
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
/// Probes only the configured provider host. It emits edge-triggered connectivity events and
/// never performs a usage request. A 5-second interval bounds recovery latency without creating
/// an unbounded retry loop; disabled monitoring cancels naturally when the task is dropped.
pub fn start(
    provider_host: String,
    monitoring_enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    sender: tokio::sync::mpsc::Sender<SystemEvent>,
) -> tokio::task::JoinHandle<()> {
    #[cfg(target_os = "windows")]
    {
        let host = provider_host.clone();
        let monitor = monitoring_enabled.clone();
        let fallback_monitor = monitoring_enabled.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let com_sender = sender.clone();
        let handle = std::thread::spawn(move || {
            use windows::Win32::Networking::NetworkListManager::{
                INetworkListManager, NetworkListManager,
            };
            use windows::Win32::System::Com::{
                CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
            };
            struct ComGuard;
            impl Drop for ComGuard {
                fn drop(&mut self) {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
            let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
            if !initialized {
                let _ = ready_tx.send(false);
                return;
            }
            let _guard = ComGuard;
            let manager: windows::core::Result<INetworkListManager> =
                unsafe { CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL) };
            let Ok(manager) = manager else {
                let _ = ready_tx.send(false);
                return;
            };
            let _ = ready_tx.send(true);
            let mut state = OnlineState::new(true);
            loop {
                if monitor.load(std::sync::atomic::Ordering::Acquire) {
                    let online = unsafe { manager.IsConnected() }
                        .map(|v| v.0 != 0)
                        .unwrap_or(false);
                    if let Some(event) = state.update(online) {
                        if com_sender.blocking_send(event).is_err() {
                            break;
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
        });
        return tokio::spawn(async move {
            // The COM observer owns the channel for its lifetime. If it cannot initialize (for
            // example on a restricted desktop), use the configured host reachability fallback.
            let com_ready = tokio::task::spawn_blocking(move || ready_rx.recv().unwrap_or(false))
                .await
                .unwrap_or(false);
            if com_ready {
                let _ = handle.join();
                return;
            }
            let client = reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(2))
                .timeout(std::time::Duration::from_secs(3))
                .build()
                .unwrap_or_default();
            let mut state = OnlineState::new(true);
            loop {
                if fallback_monitor.load(std::sync::atomic::Ordering::Acquire) {
                    let online = client.get(&host).send().await.is_ok();
                    if let Some(event) = state.update(online) {
                        if sender.send(event).await.is_err() {
                            break;
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });
    }
    #[allow(unreachable_code)]
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(2))
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap_or_default();
        let mut state = OnlineState::new(true);
        loop {
            if monitoring_enabled.load(std::sync::atomic::Ordering::Acquire) {
                let online = client.get(&provider_host).send().await.is_ok();
                if let Some(event) = state.update(online) {
                    if sender.send(event).await.is_err() {
                        break;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connectivity_is_edge_triggered() {
        let mut s = OnlineState::new(false);
        assert_eq!(s.update(true), Some(SystemEvent::NetworkOnline));
        assert_eq!(s.update(true), None);
    }
}
