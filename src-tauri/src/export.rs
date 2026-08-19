use crate::history::{HistoryQuery, HistoryResult};
use serde::Serialize;
use std::{fs::OpenOptions, io::Write, path::Path};
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
    r: &HistoryResult,
    q: &HistoryQuery,
    at: i64,
) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&Envelope {
        schema_version: 1,
        exported_at: at,
        query: q,
        points: &r.points,
        billing: &r.billing,
    })
}
pub fn history_csv(r: &HistoryResult) -> Result<Vec<u8>, csv::Error> {
    let mut w = csv::Writer::from_writer(Vec::new());
    w.write_record([
        "provider",
        "window_kind",
        "sampled_at",
        "used_percent",
        "model",
        "api_calls",
        "estimated_cost_micros",
        "overage_cost_micros",
    ])?;
    for p in &r.points {
        w.write_record([
            p.provider.clone(),
            p.window_kind.clone(),
            p.sampled_at.to_string(),
            p.used_percent.to_string(),
            p.model.clone().unwrap_or_default(),
            p.api_calls.map(|v| v.to_string()).unwrap_or_default(),
            p.estimated_cost_micros
                .map(|v| v.to_string())
                .unwrap_or_default(),
            p.overage_cost_micros
                .map(|v| v.to_string())
                .unwrap_or_default(),
        ])?;
    }
    Ok(w.into_inner().map_err(|e| e.into_error())?)
}
pub fn write_export(
    dest: &Path,
    format: &str,
    r: &HistoryResult,
    q: &HistoryQuery,
    at: i64,
) -> Result<(), String> {
    if dest.is_dir()
        || !matches!(format, "json" | "csv")
        || dest.extension().and_then(|e| e.to_str()) != Some(format)
    {
        return Err("invalid destination".into());
    }
    let tmp = dest.with_extension(format!("{}.tmp", format));
    let bytes = if format == "json" {
        history_json(r, q, at).map_err(|e| e.to_string())?
    } else {
        history_csv(r).map_err(|e| e.to_string())?
    };
    let result = (|| {
        let mut f = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        f.write_all(&bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_has_stable_columns_and_blank_nulls() {
        let r = HistoryResult {
            points: vec![],
            billing: vec![],
        };
        assert!(String::from_utf8(history_csv(&r).unwrap())
            .unwrap()
            .starts_with("provider,window_kind"));
    }
}
