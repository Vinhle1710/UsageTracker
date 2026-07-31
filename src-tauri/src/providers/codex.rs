use crate::model::{label_for_minutes, SnapshotState, UsageSnapshot, UsageWindow};
use std::path::{Path, PathBuf};

pub fn parse_rate_limits(
    value: &serde_json::Value,
    fetched_at: i64,
    state: SnapshotState,
) -> UsageSnapshot {
    let mut windows = Vec::new();
    for key in ["primary", "secondary"] {
        let Some(window) = value.get(key).filter(|v| !v.is_null()) else {
            continue;
        };
        let (Some(used), Some(minutes)) = (
            window.get("used_percent").and_then(|v| v.as_f64()),
            window.get("window_minutes").and_then(|v| v.as_u64()),
        ) else {
            continue;
        };
        windows.push(UsageWindow {
            label: label_for_minutes(minutes as u32),
            used_percent: used as f32,
            resets_at: window
                .get("resets_at")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        });
    }
    UsageSnapshot {
        windows,
        fetched_at,
        state,
    }
}

pub fn last_rate_limits_from_jsonl(contents: &str) -> Option<serde_json::Value> {
    contents.lines().rev().find_map(|line| {
        let value = serde_json::from_str(line).ok()?;
        find_key(&value, "rate_limits")
    })
}
fn find_key(value: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => map
            .get(key)
            .cloned()
            .or_else(|| map.values().find_map(|v| find_key(v, key))),
        serde_json::Value::Array(items) => items.iter().find_map(|v| find_key(v, key)),
        _ => None,
    }
}

pub fn latest_rate_limits_from_sessions(dir: &Path) -> Option<serde_json::Value> {
    let mut files = Vec::new();
    collect_jsonl_files(dir, &mut files);
    files.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    });
    files.into_iter().rev().find_map(|path| {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|contents| last_rate_limits_from_jsonl(&contents))
    })
}

fn collect_jsonl_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_real_payload_with_null_secondary() {
        let v: serde_json::Value = serde_json::from_str(r#"{"primary":{"used_percent":25.0,"window_minutes":10080,"resets_at":1785978830},"secondary":null}"#).unwrap();
        let s = parse_rate_limits(&v, 100, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 1);
        assert_eq!(s.windows[0].label, "Weekly");
    }
    #[test]
    fn renders_both_windows_when_present() {
        let v: serde_json::Value = serde_json::from_str(r#"{"primary":{"used_percent":10.0,"window_minutes":300,"resets_at":1},"secondary":{"used_percent":40.0,"window_minutes":10080,"resets_at":2}}"#).unwrap();
        let s = parse_rate_limits(&v, 0, SnapshotState::Fresh);
        assert_eq!(s.windows.len(), 2);
        assert_eq!(s.windows[1].label, "Weekly");
    }
    #[test]
    fn zero_percent_still_renders() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"primary":{"used_percent":0.0,"window_minutes":300}}"#)
                .unwrap();
        assert_eq!(
            parse_rate_limits(&v, 0, SnapshotState::Fresh).windows.len(),
            1
        );
    }
    #[test]
    fn absent_window_produces_no_row() {
        let v: serde_json::Value = serde_json::from_str(r#"{"secondary":null}"#).unwrap();
        assert!(parse_rate_limits(&v, 0, SnapshotState::Fresh)
            .windows
            .is_empty());
    }
    #[test]
    fn extracts_last_rate_limits_from_jsonl() {
        let v = last_rate_limits_from_jsonl(
            r#"{"msg":{"rate_limits":{"primary":{"used_percent":9.0,"window_minutes":300}}}}"#,
        )
        .unwrap();
        assert_eq!(
            parse_rate_limits(&v, 0, SnapshotState::Stale).windows[0].used_percent,
            9.0
        );
    }
    #[test]
    fn jsonl_without_rate_limits_returns_none() {
        assert!(last_rate_limits_from_jsonl(r#"{"msg":"hello"}"#).is_none());
    }
    #[test]
    fn finds_latest_session_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.jsonl"),
            r#"{"rate_limits":{"primary":{"used_percent":7.0,"window_minutes":300}}}"#,
        )
        .unwrap();
        let value = latest_rate_limits_from_sessions(dir.path()).unwrap();
        assert_eq!(
            parse_rate_limits(&value, 0, SnapshotState::Stale).windows[0].used_percent,
            7.0
        );
    }
}
