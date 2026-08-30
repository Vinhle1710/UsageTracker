use crate::history::{HistoryQuery, HistoryResult};
use serde::Serialize;
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
struct Envelope<'a> {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    exported_at: i64,
    query: &'a HistoryQuery,
    points: &'a Vec<crate::history::HistoryPoint>,
    billing: &'a Vec<crate::history::BillingEntry>,
}

pub fn history_json(
    result: &HistoryResult,
    query: &HistoryQuery,
    exported_at: i64,
) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&Envelope {
        schema_version: 1,
        exported_at,
        query,
        points: &result.points,
        billing: &result.billing,
    })
}

pub fn history_csv(result: &HistoryResult) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    write_csv_stream(&mut output, result)?;
    Ok(output)
}

fn sibling_temp(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("history");
    destination.with_file_name(format!(".{name}.tmp-{}", std::process::id()))
}

fn sibling_backup(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("history");
    destination.with_file_name(format!(".{name}.bak-{}", std::process::id()))
}

pub fn write_export(
    destination: &Path,
    format: &str,
    result: &HistoryResult,
    query: &HistoryQuery,
    exported_at: i64,
) -> Result<(), String> {
    if destination.is_dir()
        || destination.parent().is_some_and(|parent| !parent.exists())
        || !matches!(format, "json" | "csv")
        || destination
            .extension()
            .and_then(|extension| extension.to_str())
            != Some(format)
    {
        return Err("invalid destination".into());
    }

    let temporary = sibling_temp(destination);
    let prepared = (|| {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        match format {
            "json" => write_json_stream(&mut file, result, query, exported_at)?,
            "csv" => write_csv_stream(&mut file, result)?,
            _ => unreachable!("format validated above"),
        }
        file.sync_all().map_err(|error| error.to_string())
    })();
    if let Err(error) = prepared {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    publish_file(&temporary, destination)
}

fn publish_file(temporary: &Path, destination: &Path) -> Result<(), String> {
    let backup = sibling_backup(destination);
    let had_destination = destination.exists();
    if had_destination {
        std::fs::rename(destination, &backup).map_err(|error| error.to_string())?;
    }
    match std::fs::rename(temporary, destination) {
        Ok(()) => {
            let _ = std::fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            if had_destination {
                let _ = std::fs::rename(&backup, destination);
            }
            let _ = std::fs::remove_file(temporary);
            Err(error.to_string())
        }
    }
}

fn write_json_stream(
    mut output: impl Write,
    result: &HistoryResult,
    query: &HistoryQuery,
    exported_at: i64,
) -> Result<(), String> {
    out(&mut output, b"{\"schemaVersion\":1,\"exportedAt\":")?;
    serde_json::to_writer(&mut output, &exported_at).map_err(|error| error.to_string())?;
    out(&mut output, b",\"query\":")?;
    serde_json::to_writer(&mut output, query).map_err(|error| error.to_string())?;
    out(&mut output, b",\"points\":[")?;
    for (index, point) in result.points.iter().enumerate() {
        if index > 0 {
            out(&mut output, b",")?;
        }
        serde_json::to_writer(&mut output, point).map_err(|error| error.to_string())?;
    }
    out(&mut output, b"],\"billing\":[")?;
    for (index, billing) in result.billing.iter().enumerate() {
        if index > 0 {
            out(&mut output, b",")?;
        }
        serde_json::to_writer(&mut output, billing).map_err(|error| error.to_string())?;
    }
    out(&mut output, b"]}")
}

fn out(output: &mut impl Write, bytes: &[u8]) -> Result<(), String> {
    output.write_all(bytes).map_err(|error| error.to_string())
}

fn write_csv_stream(mut output: impl Write, result: &HistoryResult) -> Result<(), String> {
    let mut writer = csv::Writer::from_writer(&mut output);
    writer
        .write_record([
            "record_type",
            "provider",
            "window_kind",
            "sampled_at",
            "used_percent",
            "model",
            "api_calls",
            "estimated_cost_micros",
            "overage_cost_micros",
            "period_start",
            "period_end",
            "amount_micros",
            "currency",
            "source",
        ])
        .map_err(|error| error.to_string())?;
    for point in &result.points {
        writer
            .write_record([
                "usage".to_string(),
                point.provider.clone(),
                point.window_kind.clone(),
                point.sampled_at.to_string(),
                point.used_percent.to_string(),
                point.model.clone().unwrap_or_default(),
                point
                    .api_calls
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                point
                    .estimated_cost_micros
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                point
                    .overage_cost_micros
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ])
            .map_err(|error| error.to_string())?;
    }
    for billing in &result.billing {
        writer
            .write_record([
                "billing".to_string(),
                billing.provider.clone(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                billing.period_start.to_string(),
                billing.period_end.to_string(),
                billing.amount_micros.to_string(),
                billing.currency.clone(),
                billing.source.clone(),
            ])
            .map_err(|error| error.to_string())?;
    }
    writer.flush().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fixture() -> HistoryResult {
        HistoryResult {
            points: vec![crate::history::HistoryPoint {
                provider: "claude".into(),
                window_kind: "session_5h".into(),
                sampled_at: 1000,
                used_percent: 25.0,
                model: Some("model,one".into()),
                api_calls: None,
                estimated_cost_micros: None,
                overage_cost_micros: None,
            }],
            billing: vec![crate::history::BillingEntry {
                provider: "claude".into(),
                period_start: 1,
                period_end: 2,
                amount_micros: 3,
                currency: "USD".into(),
                source: "provider".into(),
            }],
        }
    }

    fn query() -> HistoryQuery {
        HistoryQuery {
            from: 0,
            to: 10,
            provider: None,
            window_kind: None,
        }
    }

    #[test]
    fn csv_has_stable_columns_and_blank_nulls() {
        let text = String::from_utf8(history_csv(&fixture()).unwrap()).unwrap();
        assert_eq!(text.lines().next().unwrap(), "record_type,provider,window_kind,sampled_at,used_percent,model,api_calls,estimated_cost_micros,overage_cost_micros,period_start,period_end,amount_micros,currency,source");
        assert!(text.contains("usage,claude,session_5h"));
        assert!(text.contains("billing,claude"));
        assert!(text.contains("\"model,one\""));
    }

    #[test]
    fn json_is_versioned_and_csv_contains_billing_in_the_chosen_file() {
        let value: serde_json::Value =
            serde_json::from_slice(&history_json(&fixture(), &query(), 99).unwrap()).unwrap();
        assert_eq!(value["schemaVersion"], 1);
        let csv = String::from_utf8(history_csv(&fixture()).unwrap()).unwrap();
        assert!(csv.contains("billing,claude"));
        assert!(csv.contains(",1,2,3,USD,provider"));
    }

    #[test]
    fn filesystem_export_is_atomic_streamed_and_overwrites_one_file() {
        let directory = tempdir().unwrap();
        let destination = directory.path().join("history.csv");
        write_export(&destination, "csv", &fixture(), &query(), 99).unwrap();
        let text = std::fs::read_to_string(&destination).unwrap();
        assert!(text.contains("usage,claude,session_5h"));
        assert!(text.contains("billing,claude"));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);

        write_export(
            &destination,
            "csv",
            &HistoryResult {
                points: vec![],
                billing: vec![],
            },
            &query(),
            100,
        )
        .unwrap();
        assert!(!std::fs::read_to_string(&destination)
            .unwrap()
            .contains("model,one"));
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn invalid_destination_is_rejected_without_artifacts() {
        let directory = tempdir().unwrap();
        assert!(write_export(
            &directory.path().join("missing").join("history.csv"),
            "csv",
            &fixture(),
            &query(),
            1,
        )
        .is_err());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn injected_stream_write_failure_is_returned() {
        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("injected"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(write_json_stream(FailingWriter, &fixture(), &query(), 1).is_err());
        assert!(write_csv_stream(FailingWriter, &fixture()).is_err());
    }
}
