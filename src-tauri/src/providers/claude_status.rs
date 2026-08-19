use crate::model::{ClaudeIncident, ClaudeServiceStatus};

pub async fn fetch_status(client: &reqwest::Client, url: &str) -> Result<ClaudeServiceStatus, super::FetchError> {
    let response = client.get(url).timeout(std::time::Duration::from_secs(10)).header(reqwest::header::USER_AGENT, "UsageTracker/0.1").send().await.map_err(|_| super::FetchError::Network)?;
    if !response.status().is_success() { return Err(super::FetchError::Network); }
    let value = response.json().await.map_err(|_| super::FetchError::Malformed)?;
    Ok(parse_status(&value))
}

pub fn parse_status(value: &serde_json::Value) -> ClaudeServiceStatus {
    let raw = value
        .pointer("/status/indicator")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let indicator = match raw {
        "none" => "Operational",
        "minor" => "Degraded",
        "major" => "MajorOutage",
        "critical" => "PartialOutage",
        _ => "Unknown",
    }
    .to_string();
    let description = value
        .pointer("/status/description")
        .and_then(|v| v.as_str())
        .unwrap_or("Status unavailable")
        .to_string();
    let incidents = value
        .get("incidents")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|incident| {
            let name = incident.get("name")?.as_str()?.to_string();
            let status = incident
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let url = incident
                .get("shortlink")
                .and_then(|v| v.as_str())
                .filter(|url| url.starts_with("https://status.claude.com/"))
                .map(str::to_string);
            Some(ClaudeIncident { name, status, url })
        })
        .collect();
    ClaudeServiceStatus {
        indicator,
        description,
        incidents,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_status_and_allowlists_incident_links() {
        let result = parse_status(
            &serde_json::json!({"status":{"indicator":"minor","description":"Some issues"},"incidents":[{"name":"Incident","status":"investigating","shortlink":"https://status.claude.com/incidents/a"},{"name":"Bad","status":"x","shortlink":"https://evil.example/a"}]}),
        );
        assert_eq!(result.indicator, "Degraded");
        assert_eq!(result.incidents.len(), 2);
        assert!(result.incidents[1].url.is_none());
    }
    #[test]
    fn unknown_indicator_is_explicit() {
        assert_eq!(
            parse_status(&serde_json::json!({"status":{"indicator":"new"}})).indicator,
            "Unknown"
        );
    }
}
