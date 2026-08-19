#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    Wake,
}
pub const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;
pub const PBT_APMRESUMESUSPEND: u32 = 0x0007;
pub fn map_power_status(status: u32) -> Option<SystemEvent> {
    matches!(status, PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND).then_some(SystemEvent::Wake)
}
pub fn start() {}
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
