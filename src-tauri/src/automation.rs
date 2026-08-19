#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    PollDue,
    ManualRefresh,
    Wake,
    NetworkOnline,
    NetworkOffline,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WakePoller,
    FetchNow,
    Wait,
    None,
}
pub struct Coordinator {
    online: bool,
    refresh_on_wake: bool,
    monitor_network: bool,
}
impl Coordinator {
    pub fn new(refresh_on_wake: bool, monitor_network: bool) -> Self {
        Self {
            online: true,
            refresh_on_wake,
            monitor_network,
        }
    }
    pub fn set_config(&mut self, refresh_on_wake: bool, monitor_network: bool) {
        self.refresh_on_wake = refresh_on_wake;
        self.monitor_network = monitor_network;
        if !monitor_network {
            self.online = true;
        }
    }
    pub fn on_event(&mut self, event: Event) -> Action {
        match event {
            Event::NetworkOffline if self.monitor_network => {
                self.online = false;
                Action::None
            }
            Event::NetworkOnline if self.monitor_network && !self.online => {
                self.online = true;
                Action::WakePoller
            }
            Event::NetworkOnline => {
                self.online = true;
                Action::None
            }
            Event::Wake if self.refresh_on_wake && self.online => Action::WakePoller,
            Event::ManualRefresh => Action::FetchNow,
            Event::PollDue if self.online => Action::WakePoller,
            Event::PollDue => Action::Wait,
            _ => Action::None,
        }
    }
    pub fn online(&self) -> bool {
        self.online
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn online_and_wake_request_one_refresh_each() {
        let mut c = Coordinator::new(true, true);
        assert_eq!(c.on_event(Event::NetworkOnline), Action::None);
        c.on_event(Event::NetworkOffline);
        assert_eq!(c.on_event(Event::NetworkOnline), Action::WakePoller);
        assert_eq!(c.on_event(Event::NetworkOnline), Action::None);
        assert_eq!(c.on_event(Event::Wake), Action::WakePoller);
    }
    #[test]
    fn offline_suppresses_scheduled_fetch() {
        let mut c = Coordinator::new(true, true);
        c.on_event(Event::NetworkOffline);
        assert_eq!(c.on_event(Event::PollDue), Action::Wait);
    }
    #[test]
    fn manual_refresh_is_explicit_even_offline() {
        let mut c = Coordinator::new(true, true);
        c.on_event(Event::NetworkOffline);
        assert_eq!(c.on_event(Event::ManualRefresh), Action::FetchNow);
    }
}
