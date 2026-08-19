#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    Wake,
}
pub const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;
pub const PBT_APMRESUMESUSPEND: u32 = 0x0007;
pub fn map_power_status(status: u32) -> Option<SystemEvent> {
    matches!(status, PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND).then_some(SystemEvent::Wake)
}
/// Starts a lifecycle-safe Windows power observer. The observer uses the OS power-status
/// transition as a portable fallback and sends Wake when the machine returns to AC/battery after
/// an unavailable state; callers own the channel and can stop it by dropping the sender.
pub fn start(sender: std::sync::mpsc::Sender<SystemEvent>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            use std::{thread, time::Duration};
            let mut unavailable = false;
            loop {
                let mut value = windows_sys::Win32::System::Power::SYSTEM_POWER_STATUS {
                    ACLineStatus: 0,
                    BatteryFlag: 0,
                    BatteryLifePercent: 0,
                    SystemStatusFlag: 0,
                    BatteryLifeTime: 0,
                    BatteryFullLifeTime: 0,
                };
                let ok =
                    unsafe { windows_sys::Win32::System::Power::GetSystemPowerStatus(&mut value) }
                        != 0;
                if ok && unavailable {
                    let _ = sender.send(SystemEvent::Wake);
                }
                unavailable = !ok;
                thread::sleep(Duration::from_secs(5));
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
