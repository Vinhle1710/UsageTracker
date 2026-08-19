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
pub fn start() {}
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
