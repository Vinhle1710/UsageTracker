use std::path::Path;

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE_NAME: &str = "Usage Tracker Overlay";

fn should_register(is_debug: bool, is_windows: bool) -> bool {
    !is_debug && is_windows
}

fn quoted_windows_command(executable: &Path) -> String {
    let path = executable.to_string_lossy();
    let mut command = String::with_capacity(path.len() + 2);
    let mut backslashes = 0;

    command.push('"');
    for character in path.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                command.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                command.push('"');
                backslashes = 0;
            }
            character => {
                command.extend(std::iter::repeat_n('\\', backslashes));
                command.push(character);
                backslashes = 0;
            }
        }
    }
    command.extend(std::iter::repeat_n('\\', backslashes * 2));
    command.push('"');
    command
}

fn run_registration_best_effort(register: impl FnOnce() -> Result<(), String>) {
    let _ = register();
}

/// Registers or unregisters the current executable to launch when Windows starts, reflecting
/// the user's "Launch at startup" setting. A no-op on debug builds and non-Windows targets.
pub fn set_registration(enabled: bool) {
    run_registration_best_effort(move || try_set_registration(enabled));
}

pub fn read_registration_with<F>(read: F) -> Result<bool, String>
where
    F: FnOnce(&str) -> Result<Option<String>, String>,
{
    Ok(read(RUN_VALUE_NAME)?.is_some())
}

#[cfg(target_os = "windows")]
fn try_set_registration(enabled: bool) -> Result<(), String> {
    if !should_register(cfg!(debug_assertions), true) {
        return Ok(());
    }

    if enabled {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let command = quoted_windows_command(&executable);
        write_run_value(&command)
    } else {
        delete_run_value()
    }
}

#[cfg(not(target_os = "windows"))]
fn try_set_registration(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn write_run_value(command: &str) -> Result<(), String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
        REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let key_path = wide_null(RUN_KEY_PATH);
    let value_name = wide_null(RUN_VALUE_NAME);
    let value_data = wide_null(command);
    let mut key: HKEY = null_mut();
    let create_result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if create_result != 0 {
        return Err(format!(
            "could not open startup registry key: {create_result}"
        ));
    }

    let set_result = unsafe {
        RegSetValueExW(
            key,
            value_name.as_ptr(),
            0,
            REG_SZ,
            value_data.as_ptr().cast(),
            (value_data.len() * std::mem::size_of::<u16>()) as u32,
        )
    };
    let close_result = unsafe { RegCloseKey(key) };
    if set_result != 0 {
        return Err(format!(
            "could not write startup registry value: {set_result}"
        ));
    }
    if close_result != 0 {
        return Err(format!(
            "could not close startup registry key: {close_result}"
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn delete_run_value() -> Result<(), String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    };

    let key_path = wide_null(RUN_KEY_PATH);
    let value_name = wide_null(RUN_VALUE_NAME);
    let mut key: HKEY = null_mut();
    let open_result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
    };
    if open_result != 0 {
        if open_result as u32 == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        return Err(format!(
            "could not open startup registry key: {open_result}"
        ));
    }

    let delete_result = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };
    let close_result = unsafe { RegCloseKey(key) };
    if delete_result != 0 && delete_result as u32 != ERROR_FILE_NOT_FOUND {
        return Err(format!(
            "could not delete startup registry value: {delete_result}"
        ));
    }
    if close_result != 0 {
        return Err(format!(
            "could not close startup registry key: {close_result}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        quoted_windows_command, read_registration_with, run_registration_best_effort,
        set_registration, should_register,
    };
    use std::cell::Cell;
    use std::path::Path;

    #[test]
    fn quotes_an_executable_path_containing_spaces() {
        let executable =
            Path::new(r"C:\Program Files\Usage Tracker Overlay\usage-tracker-overlay.exe");

        assert_eq!(
            quoted_windows_command(executable),
            r#""C:\Program Files\Usage Tracker Overlay\usage-tracker-overlay.exe""#
        );
    }

    #[test]
    fn escapes_embedded_quotes_and_backslashes() {
        let executable =
            Path::new(r#"C:\Program Files\Usage "Tracker"\usage-tracker-overlay.exe\"#);

        assert_eq!(
            quoted_windows_command(executable),
            "\"C:\\Program Files\\Usage \\\"Tracker\\\"\\usage-tracker-overlay.exe\\\\\""
        );
    }

    #[test]
    fn does_not_append_a_trailing_empty_argument() {
        let command = quoted_windows_command(Path::new(
            r"C:\Program Files\Usage Tracker Overlay\usage-tracker-overlay.exe",
        ));

        assert_eq!(
            command,
            r#""C:\Program Files\Usage Tracker Overlay\usage-tracker-overlay.exe""#
        );
        assert!(!command.ends_with(" \"\""));
    }

    #[test]
    fn registers_only_for_release_builds_on_windows() {
        assert!(!should_register(true, true));
        assert!(!should_register(true, false));
        assert!(should_register(false, true));
        assert!(!should_register(false, false));
    }

    #[test]
    fn registration_errors_do_not_escape_startup() {
        let attempted = Cell::new(false);

        run_registration_best_effort(|| {
            attempted.set(true);
            Err("registry unavailable".to_string())
        });

        assert!(attempted.get());
    }

    #[test]
    fn set_registration_does_not_panic_when_enabling_or_disabling() {
        set_registration(true);
        set_registration(false);
    }
    #[test]
    fn startup_status_reports_disabled_when_value_absent() {
        assert!(!read_registration_with(|_| Ok(None)).unwrap());
    }
}
