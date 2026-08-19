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

/// Capability flags intentionally default to false until an endpoint has an authoritative,
/// redacted fixture. This prevents an undocumented Console web surface from being guessed.
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
        spend: false,
        prepaid_balance: false,
        daily: false,
        by_api_key: false,
        by_model: false,
    }
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
    fn capabilities_are_disabled_without_verified_evidence() {
        assert!(!source_capabilities().spend);
    }
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
}
