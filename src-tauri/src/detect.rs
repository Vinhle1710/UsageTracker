use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use sysinfo::System;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct ActiveSources {
    pub claude: bool,
    pub openai: bool,
}

#[derive(Debug, PartialEq)]
pub struct IdeLock {
    pub pid: u32,
    pub ide_name: String,
}

pub fn parse_lock(contents: &str) -> Option<IdeLock> {
    let value: serde_json::Value = serde_json::from_str(contents).ok()?;
    Some(IdeLock {
        pid: value.get("pid")?.as_u64()? as u32,
        ide_name: value.get("ideName")?.as_str()?.into(),
    })
}
pub fn has_live_ide_lock(dir: &Path, live_pids: &HashSet<u32>) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry.path().extension().and_then(|x| x.to_str()) == Some("lock")
            && std::fs::read_to_string(entry.path())
                .ok()
                .and_then(|s| parse_lock(&s))
                .is_some_and(|lock| live_pids.contains(&lock.pid))
    })
}
pub fn resolve(process_names: &[String], has_live_lock: bool) -> ActiveSources {
    let has = |name: &str| {
        process_names
            .iter()
            .any(|process| process.eq_ignore_ascii_case(name))
    };
    ActiveSources {
        claude: has("claude.exe") || has("claude") || has_live_lock,
        openai: has("ChatGPT.exe")
            || has("ChatGPT")
            || has("codex.exe")
            || has("codex")
            || has("codex-code-mode-host.exe"),
    }
}
pub fn scan_processes(system: &mut System) -> (Vec<String>, HashSet<u32>) {
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
        .processes()
        .iter()
        .map(|(pid, process)| (process.name().to_string_lossy().into_owned(), pid.as_u32()))
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    const LOCK: &str = r#"{"pid":11472,"ideName":"Visual Studio Code"}"#;
    #[test]
    fn parses_lock() {
        let lock = parse_lock(LOCK).unwrap();
        assert_eq!(lock.pid, 11472);
        assert_eq!(lock.ide_name, "Visual Studio Code");
    }
    #[test]
    fn stale_lock_is_not_live() {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("1.lock"), LOCK).unwrap();
        assert!(!has_live_ide_lock(d.path(), &HashSet::from([99999])));
    }
    #[test]
    fn live_lock_is_live() {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("1.lock"), LOCK).unwrap();
        assert!(has_live_ide_lock(d.path(), &HashSet::from([11472])));
    }
    #[test]
    fn missing_dir_is_not_live() {
        assert!(!has_live_ide_lock(
            Path::new("/nonexistent"),
            &HashSet::new()
        ));
    }
    #[test]
    fn non_lock_files_are_ignored() {
        let d = tempdir().unwrap();
        std::fs::write(d.path().join("notes.txt"), LOCK).unwrap();
        assert!(!has_live_ide_lock(d.path(), &HashSet::from([11472])));
    }
    #[test]
    fn claude_process_activates_claude_only() {
        let active = resolve(&["claude.exe".into()], false);
        assert!(active.claude && !active.openai);
    }
    #[test]
    fn chatgpt_process_activates_openai_only() {
        let active = resolve(&["ChatGPT.exe".into()], false);
        assert!(active.openai && !active.claude);
    }
    #[test]
    fn live_lock_activates_claude() {
        assert!(resolve(&[], true).claude);
    }
    #[test]
    fn no_signals_activate_nothing() {
        assert_eq!(resolve(&[], false), ActiveSources::default());
    }
    #[test]
    fn scan_finds_current_process() {
        let mut system = System::new();
        let (names, pids) = scan_processes(&mut system);
        assert!(!names.is_empty());
        assert!(pids.contains(&std::process::id()));
    }
}
