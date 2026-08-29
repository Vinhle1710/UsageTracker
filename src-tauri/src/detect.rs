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
/// Ticks a source stays reported as active after the last scan that saw its process.
///
/// The detection loop runs once a second, and the CLIs it looks for are not daemons: `codex`
/// exits between invocations, `claude` restarts, and an IDE lock file is rewritten whenever a
/// session turns over. Treating one empty scan as "the provider is gone" is what made the
/// overlay blink out at random — the window was hidden and then shown again a second later.
/// Holding a source for a grace period costs a stale card for at most this long after a real
/// exit, and removes the flicker entirely.
pub const SOURCE_GRACE_TICKS: u8 = 60;

/// Debounces *deactivation* only. A newly seen source activates on the very first scan, the
/// same as before; only the disappearance is delayed, because that is the direction whose
/// false positives were visible to the user.
#[derive(Debug, Default)]
pub struct SourceHold {
    grace_ticks: u8,
    claude_remaining: u8,
    openai_remaining: u8,
}

impl SourceHold {
    pub fn new(grace_ticks: u8) -> Self {
        Self {
            grace_ticks,
            claude_remaining: 0,
            openai_remaining: 0,
        }
    }

    /// Feeds one raw scan in and returns what the rest of the app should treat as active.
    pub fn observe(&mut self, scanned: ActiveSources) -> ActiveSources {
        ActiveSources {
            claude: Self::hold(&mut self.claude_remaining, self.grace_ticks, scanned.claude),
            openai: Self::hold(&mut self.openai_remaining, self.grace_ticks, scanned.openai),
        }
    }

    fn hold(remaining: &mut u8, grace_ticks: u8, seen: bool) -> bool {
        if seen {
            *remaining = grace_ticks;
            return true;
        }
        // Checked before the decrement, so `grace_ticks` names exactly how many absent scans
        // are tolerated. A source never seen sits at 0 and is reported inactive immediately,
        // which is also what makes a zero grace period pass the raw scan straight through.
        if *remaining == 0 {
            return false;
        }
        *remaining -= 1;
        true
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
    fn a_source_stays_active_through_a_brief_process_gap() {
        // The reported symptom: the overlay vanishing at random while nothing was wrong. The
        // CLIs are not long-lived daemons — `codex` exits between invocations and `claude`
        // restarts — so a single scan that misses them used to hide the window outright.
        let mut hold = SourceHold::new(3);
        assert_eq!(
            hold.observe(ActiveSources {
                claude: true,
                openai: true
            }),
            ActiveSources {
                claude: true,
                openai: true
            }
        );

        for tick in 0..3 {
            assert_eq!(
                hold.observe(ActiveSources::default()),
                ActiveSources {
                    claude: true,
                    openai: true
                },
                "tick {tick} dropped a source still inside its grace period"
            );
        }
    }

    #[test]
    fn a_source_that_stays_gone_past_the_grace_period_deactivates() {
        let mut hold = SourceHold::new(2);
        hold.observe(ActiveSources {
            claude: true,
            openai: false,
        });

        hold.observe(ActiveSources::default());
        hold.observe(ActiveSources::default());

        assert_eq!(
            hold.observe(ActiveSources::default()),
            ActiveSources::default()
        );
    }

    #[test]
    fn reappearing_within_the_grace_period_resets_the_countdown() {
        let mut hold = SourceHold::new(2);
        hold.observe(ActiveSources {
            claude: true,
            openai: false,
        });
        hold.observe(ActiveSources::default());
        hold.observe(ActiveSources {
            claude: true,
            openai: false,
        });

        // Without the reset, the next two absent ticks would exhaust a countdown that the
        // sighting should have restarted.
        assert!(hold.observe(ActiveSources::default()).claude);
        assert!(hold.observe(ActiveSources::default()).claude);
        assert!(!hold.observe(ActiveSources::default()).claude);
    }

    #[test]
    fn each_source_holds_independently() {
        let mut hold = SourceHold::new(2);
        hold.observe(ActiveSources {
            claude: true,
            openai: true,
        });
        hold.observe(ActiveSources {
            claude: true,
            openai: false,
        });
        hold.observe(ActiveSources {
            claude: true,
            openai: false,
        });

        // Claude keeps being seen, so its hold never counts down; only openai's expires.
        let settled = hold.observe(ActiveSources {
            claude: true,
            openai: false,
        });
        assert!(settled.claude);
        assert!(!settled.openai);
    }

    #[test]
    fn a_source_never_seen_is_never_held_active() {
        let mut hold = SourceHold::new(5);
        assert_eq!(
            hold.observe(ActiveSources::default()),
            ActiveSources::default()
        );
    }

    #[test]
    fn a_zero_grace_period_reports_the_raw_scan() {
        let mut hold = SourceHold::new(0);
        hold.observe(ActiveSources {
            claude: true,
            openai: false,
        });
        assert_eq!(
            hold.observe(ActiveSources::default()),
            ActiveSources::default()
        );
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
