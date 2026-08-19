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
