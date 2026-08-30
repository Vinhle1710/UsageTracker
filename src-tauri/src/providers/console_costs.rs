use crate::model::{ConsoleMoney, CostPoint};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub enum CostParseError {
    MissingField,
    InvalidAmount,
    MixedCurrency,
    NegativeAmount,
    Overflow,
}

/// Capability flags gate which requests the client makes. Enabled only for routes proven to
/// exist by the 2026-08-30 live probe recorded in `tests/fixtures/console/README.md`.
/// `by_api_key` covers the *breakdown*, which `usage_cost` carries itself via `key_id`; the
/// separate `/api_keys` name lookup 404s and stays off, so keys show as redacted ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCapabilities {
    pub spend: bool,
    pub prepaid_balance: bool,
    pub daily: bool,
    pub by_api_key: bool,
    pub by_model: bool,
}
pub const fn source_capabilities() -> SourceCapabilities {
    SourceCapabilities {
        spend: true,
        prepaid_balance: true,
        daily: true,
        by_api_key: true,
        by_model: true,
    }
}

/// Console reports money in **cents**; this app carries it in **micro-units** (10^-6 of a
/// currency unit) so sub-cent API pricing survives aggregation. 1 cent = 10_000 micro-units,
/// and `formatMicros` on the frontend divides by 1e6.
pub const MICROS_PER_CENT: i128 = 10_000;

/// Converts a cent amount written as a decimal string into integer micro-units. Deliberately
/// string-based: `usage_cost` totals are fractional cents, and summing those as f64 is how
/// money reports drift.
pub fn cents_decimal_to_micros(amount: &str) -> Result<i128, CostParseError> {
    let value = amount.trim();
    let (sign, digits) = match value.strip_prefix('-') {
        Some(rest) => (-1i128, rest),
        None => (1i128, value),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty() && fraction.is_empty() {
        return Err(CostParseError::InvalidAmount);
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return Err(CostParseError::InvalidAmount);
    }
    // Beyond 4 decimal places of a cent there is no micro-unit left to hold the digit; refusing
    // beats silently truncating someone's bill.
    if fraction.len() > 4 {
        return Err(CostParseError::InvalidAmount);
    }
    let scaled: String = format!("{whole}{fraction:0<4}");
    let magnitude = scaled
        .parse::<i128>()
        .map_err(|_| CostParseError::Overflow)?;
    let micros = sign * magnitude;
    if micros < 0 {
        return Err(CostParseError::NegativeAmount);
    }
    Ok(micros)
}

pub fn micros_money(micros: i128, currency: &str) -> ConsoleMoney {
    ConsoleMoney {
        minor_units: micros.to_string(),
        currency: currency.to_owned(),
    }
}

/// Source DTOs. Field names are the Console web API's own; never rename them here.
pub mod dto {
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, Deserialize)]
    pub struct Organization {
        pub uuid: String,
        pub name: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct CurrentSpend {
        /// Integer cents.
        pub amount: i64,
        pub resets_at: Option<String>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct PrepaidCredits {
        /// Integer cents.
        pub amount: i64,
        pub currency: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct UsageCostEntry {
        pub key_id: Option<String>,
        pub model_name: Option<String>,
        /// Fractional cents. Held as a `Number` so the literal reaches the decimal parser
        /// without a detour through f64 arithmetic.
        pub total: Option<serde_json::Number>,
    }

    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct UsageCost {
        pub costs: Option<BTreeMap<String, Vec<UsageCostEntry>>>,
        pub web_search_costs: Option<BTreeMap<String, Vec<UsageCostEntry>>>,
        pub code_execution_costs: Option<BTreeMap<String, Vec<UsageCostEntry>>>,
    }
}

/// One pass over every cost map, bucketed three ways at once. `web_search_costs` and
/// `code_execution_costs` carry no model name, so they are labelled by their source instead of
/// being dropped — they are real spend.
pub struct CostBreakdown {
    pub total_micros: i128,
    pub daily: Vec<CostPoint>,
    pub by_api_key: Vec<CostPoint>,
    pub by_model: Vec<CostPoint>,
}

pub fn breakdown(usage: &dto::UsageCost, currency: &str) -> Result<CostBreakdown, CostParseError> {
    use std::collections::BTreeMap;
    let mut total: i128 = 0;
    let mut daily: BTreeMap<String, i128> = BTreeMap::new();
    let mut by_key: BTreeMap<String, i128> = BTreeMap::new();
    let mut by_model: BTreeMap<String, i128> = BTreeMap::new();

    let groups = [
        (usage.costs.as_ref(), None),
        (usage.web_search_costs.as_ref(), Some("Web Search")),
        (usage.code_execution_costs.as_ref(), Some("Code Execution")),
    ];
    for (map, fallback_model) in groups {
        let Some(map) = map else { continue };
        for (date, entries) in map {
            for entry in entries {
                let Some(number) = entry.total.as_ref() else {
                    continue;
                };
                let micros = cents_decimal_to_micros(&number.to_string())?;
                total = total.checked_add(micros).ok_or(CostParseError::Overflow)?;
                *daily.entry(date.clone()).or_default() += micros;
                let key = entry.key_id.clone().unwrap_or_else(|| "unknown".into());
                *by_key.entry(key).or_default() += micros;
                let model = fallback_model
                    .map(str::to_owned)
                    .or_else(|| {
                        entry
                            .model_name
                            .as_deref()
                            .map(|name| name.replace(" Usage", ""))
                    })
                    .unwrap_or_else(|| "Unknown".into());
                *by_model.entry(model).or_default() += micros;
            }
        }
    }

    let points = |map: BTreeMap<String, i128>, redact: bool| {
        map.into_iter()
            .map(|(key, micros)| CostPoint {
                label: if redact {
                    redact_key_label(&key)
                } else {
                    key.clone()
                },
                key,
                amount: micros_money(micros, currency),
            })
            .collect::<Vec<_>>()
    };
    Ok(CostBreakdown {
        total_micros: total,
        daily: points(daily, false),
        by_api_key: points(by_key, true),
        by_model: points(by_model, false),
    })
}

pub fn unavailable_reason_for_status(status: u16) -> &'static str {
    match status {
        401 => "noCredential",
        403 => "insufficientRole",
        404 => "unsupportedBySource",
        _ => "providerUnavailable",
    }
}

/// Parses only an already verified, source-specific amount object. The adapter deliberately
/// accepts decimal strings and never performs floating-point arithmetic.
pub fn parse_money(value: &Value) -> Result<ConsoleMoney, CostParseError> {
    let amount = value
        .get("amount")
        .and_then(Value::as_str)
        .ok_or(CostParseError::MissingField)?;
    let currency = value
        .get("currency")
        .and_then(Value::as_str)
        .ok_or(CostParseError::MissingField)?;
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(CostParseError::MixedCurrency);
    }
    let (whole, fraction) = amount.split_once('.').unwrap_or((amount, ""));
    if whole.starts_with('-') {
        return Err(CostParseError::NegativeAmount);
    }
    if !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
        || fraction.len() > 3
    {
        return Err(CostParseError::InvalidAmount);
    }
    let minor = whole
        .parse::<u128>()
        .map_err(|_| CostParseError::Overflow)?
        .checked_mul(1000)
        .ok_or(CostParseError::Overflow)?
        .checked_add(fraction.parse::<u128>().unwrap_or(0) * 10u128.pow(3 - fraction.len() as u32))
        .ok_or(CostParseError::Overflow)?;
    Ok(ConsoleMoney {
        minor_units: minor.to_string(),
        currency: currency.into(),
    })
}

pub fn redact_key_label(id: &str) -> String {
    let suffix: String = id
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("Key …{suffix}")
}

pub fn aggregate_points(
    points: impl IntoIterator<Item = CostPoint>,
) -> Result<Vec<CostPoint>, CostParseError> {
    let mut out: Vec<CostPoint> = Vec::new();
    for point in points {
        if let Some(existing) = out.iter_mut().find(|candidate| candidate.key == point.key) {
            if existing.amount.currency != point.amount.currency {
                return Err(CostParseError::MixedCurrency);
            }
            let sum = existing
                .amount
                .minor_units
                .parse::<u128>()
                .map_err(|_| CostParseError::InvalidAmount)?
                .checked_add(
                    point
                        .amount
                        .minor_units
                        .parse::<u128>()
                        .map_err(|_| CostParseError::InvalidAmount)?,
                )
                .ok_or(CostParseError::Overflow)?;
            existing.amount.minor_units = sum.to_string();
        } else {
            out.push(point);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_amount_is_integer_minor_units() {
        let m = parse_money(&serde_json::json!({"amount":"12.345","currency":"USD"})).unwrap();
        assert_eq!(m.minor_units, "12345");
    }
    #[test]
    fn labels_never_expose_full_key() {
        assert_eq!(redact_key_label("secret-key-AB12"), "Key …AB12");
    }
    #[test]
    fn mixed_currency_is_rejected() {
        let a = CostPoint {
            key: "x".into(),
            label: "x".into(),
            amount: ConsoleMoney {
                minor_units: "1".into(),
                currency: "USD".into(),
            },
        };
        let mut b = a.clone();
        b.amount.currency = "EUR".into();
        assert_eq!(aggregate_points([a, b]), Err(CostParseError::MixedCurrency));
    }

    const USAGE_COST: &str = include_str!("../../tests/fixtures/console/usage-cost.json");
    const CURRENT_SPEND: &str = include_str!("../../tests/fixtures/console/current-spend.json");
    const PREPAID: &str = include_str!("../../tests/fixtures/console/prepaid-credits.json");
    const ORGS: &str = include_str!("../../tests/fixtures/console/organizations.json");

    fn usage() -> dto::UsageCost {
        serde_json::from_str(USAGE_COST).expect("usage-cost fixture parses")
    }

    #[test]
    fn source_dtos_match_the_recorded_field_names() {
        let orgs: Vec<dto::Organization> = serde_json::from_str(ORGS).unwrap();
        assert_eq!(orgs[0].uuid, "00000000-0000-0000-0000-000000000000");
        let spend: dto::CurrentSpend = serde_json::from_str(CURRENT_SPEND).unwrap();
        assert_eq!(spend.amount, 123_456);
        let credits: dto::PrepaidCredits = serde_json::from_str(PREPAID).unwrap();
        assert_eq!((credits.amount, credits.currency.as_str()), (5_000, "USD"));
    }

    #[test]
    fn fractional_cents_become_exact_micro_units() {
        assert_eq!(cents_decimal_to_micros("1234.5678").unwrap(), 12_345_678);
        assert_eq!(cents_decimal_to_micros("800").unwrap(), 8_000_000);
        assert_eq!(cents_decimal_to_micros("0.5").unwrap(), 5_000);
        assert_eq!(
            cents_decimal_to_micros("123456").unwrap(),
            123_456 * MICROS_PER_CENT
        );
    }

    #[test]
    fn amounts_finer_than_a_micro_unit_are_refused_not_truncated() {
        assert_eq!(
            cents_decimal_to_micros("1.23456"),
            Err(CostParseError::InvalidAmount)
        );
        assert_eq!(
            cents_decimal_to_micros("-1.0"),
            Err(CostParseError::NegativeAmount)
        );
        assert_eq!(cents_decimal_to_micros("1e5"), Err(CostParseError::InvalidAmount));
        assert_eq!(cents_decimal_to_micros(""), Err(CostParseError::InvalidAmount));
    }

    #[test]
    fn breakdown_sums_every_cost_map_without_floating_point_drift() {
        let b = breakdown(&usage(), "USD").unwrap();
        // 161.1718 + 898.3803 + 1098.8843 + 1766.7688 + 10 = 3935.2052 cents, exactly.
        // Summed as f64 the same values give 3935.2052000000003 — these figures are chosen
        // precisely because the float path drifts, so this asserts the integer path.
        assert_eq!(b.total_micros, 39_352_052);
    }

    #[test]
    fn breakdown_buckets_by_day_key_and_model() {
        let b = breakdown(&usage(), "USD").unwrap();
        let day: Vec<_> = b.daily.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(day, ["2026-08-01", "2026-08-02"]);
        let models: Vec<_> = b.by_model.iter().map(|p| p.key.as_str()).collect();
        // " Usage" stripped, web-search labelled by source, null model preserved as Unknown.
        assert_eq!(
            models,
            ["Unknown", "Web Search", "claude-opus-4", "claude-sonnet-4"]
        );
    }

    #[test]
    fn api_key_buckets_never_expose_the_full_key_id() {
        let b = breakdown(&usage(), "USD").unwrap();
        let full = "apikey_01ABCDEFGHIJKLMNOPQRSTUV";
        let point = b.by_api_key.iter().find(|p| p.key == full).unwrap();
        assert_eq!(point.label, "Key …STUV");
        assert!(!point.label.contains("apikey_01"));
    }

    #[test]
    fn a_missing_total_is_skipped_rather_than_counted_as_zero() {
        let usage: dto::UsageCost = serde_json::from_str(
            r#"{"costs":{"2026-08-01":[{"key_id":"k","model_name":"m","total":null}]}}"#,
        )
        .unwrap();
        let b = breakdown(&usage, "USD").unwrap();
        assert_eq!(b.total_micros, 0);
        assert!(b.by_model.is_empty());
    }

    #[test]
    fn capabilities_follow_the_live_probe() {
        let c = source_capabilities();
        assert!(c.spend && c.prepaid_balance && c.daily && c.by_model && c.by_api_key);
    }
}
