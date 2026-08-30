//! Claude "extra usage" (overage) credit adapter.
//!
//! Unlike [`claude_usage`], the field names here are not guesses: they are taken from a working
//! open-source implementation of the same two endpoints
//! (github.com/hamed-elfayome/Claude-Usage-Tracker), so this module is allowed to parse them.
//!
//! Both endpoints live on `claude.ai`, not `api.anthropic.com`, and authenticate with the
//! browser session cookie rather than the Code CLI's OAuth bearer token:
//!
//! * `GET /api/organizations/{org}/overage_spend_limit`
//!   -> `{ monthly_credit_limit, currency, used_credits, is_enabled }`
//! * `GET /api/organizations/{org}/overage_credit_grant`
//!   -> `{ remaining_balance, currency, total_granted }`
use crate::model::{ClaudeExtra, DataSection, DataSectionState, Money};

pub const CLAUDE_WEB_ORIGIN: &str = "https://claude.ai";

pub fn spend_limit_path(organization_uuid: &str) -> String {
    format!("/api/organizations/{organization_uuid}/overage_spend_limit")
}

pub fn credit_grant_path(organization_uuid: &str) -> String {
    format!("/api/organizations/{organization_uuid}/overage_credit_grant")
}

/// Amounts arrive as major units (dollars), while `Money` stores minor units so no rounding
/// error can accumulate in the UI. Values that are not finite, are negative, or overflow the
/// minor-unit range are dropped rather than clamped: a wrong number here is worse than none.
fn money(amount: Option<f64>, currency: Option<&str>) -> Option<Money> {
    let amount = amount.filter(|value| value.is_finite() && *value >= 0.0)?;
    let minor = (amount * 100.0).round();
    if minor > i64::MAX as f64 {
        return None;
    }
    Some(Money {
        minor_units: minor as i64,
        currency: currency.unwrap_or("USD").to_uppercase(),
    })
}

fn number(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get(key).and_then(serde_json::Value::as_f64)
}

fn currency(value: &serde_json::Value) -> Option<&str> {
    value.get("currency").and_then(serde_json::Value::as_str)
}

/// `is_enabled == false` means the user has not turned extra usage on, which is a real answer,
/// not a failure — the caller renders nothing rather than an error.
pub fn parse_spend_limit(value: &serde_json::Value) -> Option<(Option<Money>, Option<Money>)> {
    if value.get("is_enabled").and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }
    let currency = currency(value);
    Some((
        money(number(value, "used_credits"), currency),
        money(number(value, "monthly_credit_limit"), currency),
    ))
}

pub fn parse_credit_grant(value: &serde_json::Value) -> Option<Money> {
    money(number(value, "remaining_balance"), currency(value))
}

/// Folds both responses into one section. Either endpoint may be absent (a plan without extra
/// usage, a request that failed) without invalidating the other.
pub fn extra_section(
    spend_limit: Option<&serde_json::Value>,
    credit_grant: Option<&serde_json::Value>,
    now: i64,
) -> DataSection<ClaudeExtra> {
    let (spend, budget) = spend_limit
        .and_then(parse_spend_limit)
        .unwrap_or((None, None));
    let balance = credit_grant.and_then(parse_credit_grant);
    if spend.is_none() && budget.is_none() && balance.is_none() {
        return crate::providers::claude_usage::unavailable_extra(now);
    }
    DataSection {
        value: Some(ClaudeExtra {
            spend,
            budget,
            balance,
        }),
        fetched_at: now,
        state: DataSectionState::Fresh,
        error_code: None,
    }
}

/// Cookie header for a claude.ai request. Only the session key is sent: the Cloudflare
/// clearance cookies a browser would add live in the sign-in webview's cookie jar, which this
/// process does not share, so a challenge here degrades to "extra credit unavailable" rather
/// than to a wrong number.
pub fn session_cookie(session_key: &str) -> String {
    format!("sessionKey={session_key}")
}

/// Both requests run concurrently and each is allowed to fail on its own — a plan with a credit
/// grant but no spend limit (or the reverse) still reports the half that answered.
pub async fn fetch_extra(
    client: &reqwest::Client,
    origin: &str,
    organization_uuid: &str,
    session_key: &str,
    now: i64,
) -> DataSection<ClaudeExtra> {
    let cookie = session_cookie(session_key);
    let spend_url = format!("{origin}{}", spend_limit_path(organization_uuid));
    let grant_url = format!("{origin}{}", credit_grant_path(organization_uuid));
    let (spend_limit, credit_grant) = tokio::join!(
        crate::providers::fetch_json_with_cookie(client, &spend_url, &cookie),
        crate::providers::fetch_json_with_cookie(client, &grant_url, &cookie),
    );
    extra_section(spend_limit.ok().as_ref(), credit_grant.ok().as_ref(), now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_scoped_to_the_organization() {
        assert_eq!(
            spend_limit_path("org-1"),
            "/api/organizations/org-1/overage_spend_limit"
        );
        assert_eq!(
            credit_grant_path("org-1"),
            "/api/organizations/org-1/overage_credit_grant"
        );
    }

    #[test]
    fn parses_an_enabled_spend_limit_into_minor_units() {
        let value = serde_json::json!({
            "monthly_credit_limit": 50.0,
            "currency": "usd",
            "used_credits": 12.34,
            "is_enabled": true
        });

        let (spend, budget) = parse_spend_limit(&value).unwrap();

        assert_eq!(spend.unwrap().minor_units, 1234);
        let budget = budget.unwrap();
        assert_eq!(budget.minor_units, 5000);
        assert_eq!(budget.currency, "USD");
    }

    #[test]
    fn a_disabled_spend_limit_reports_nothing_rather_than_zero() {
        // Extra usage switched off is not "you have spent $0 of $0"; rendering that would
        // claim a limit exists when none does.
        let value = serde_json::json!({
            "monthly_credit_limit": 50.0,
            "used_credits": 0.0,
            "is_enabled": false
        });

        assert!(parse_spend_limit(&value).is_none());
    }

    #[test]
    fn a_spend_limit_without_the_enabled_flag_is_not_assumed_enabled() {
        let value = serde_json::json!({"monthly_credit_limit": 50.0, "used_credits": 1.0});
        assert!(parse_spend_limit(&value).is_none());
    }

    #[test]
    fn parses_a_credit_grant_balance() {
        let value =
            serde_json::json!({"remaining_balance": 7.5, "currency": "USD", "total_granted": 20.0});
        assert_eq!(parse_credit_grant(&value).unwrap().minor_units, 750);
    }

    #[test]
    fn nonsense_amounts_are_dropped_instead_of_clamped() {
        for amount in [
            serde_json::json!(-1.0),
            serde_json::json!("12.00"),
            serde_json::json!(null),
        ] {
            let value = serde_json::json!({"remaining_balance": amount});
            assert!(
                parse_credit_grant(&value).is_none(),
                "accepted a bad amount: {amount}"
            );
        }
    }

    #[test]
    fn a_missing_currency_defaults_to_usd_rather_than_dropping_the_amount() {
        let value = serde_json::json!({"remaining_balance": 3.0});
        assert_eq!(parse_credit_grant(&value).unwrap().currency, "USD");
    }

    #[test]
    fn both_endpoints_missing_leaves_the_section_unavailable() {
        let section = extra_section(None, None, 42);
        assert_eq!(section.state, DataSectionState::Unavailable);
        assert!(section.value.is_none());
    }

    #[test]
    fn one_endpoint_answering_is_enough_to_report_the_section() {
        let grant = serde_json::json!({"remaining_balance": 7.5, "currency": "USD"});

        let section = extra_section(None, Some(&grant), 42);

        assert_eq!(section.state, DataSectionState::Fresh);
        let extra = section.value.unwrap();
        assert_eq!(extra.balance.unwrap().minor_units, 750);
        assert!(extra.spend.is_none());
        assert!(extra.budget.is_none());
    }

    #[test]
    fn a_disabled_limit_alongside_a_grant_still_reports_the_grant() {
        let limit = serde_json::json!({"is_enabled": false, "monthly_credit_limit": 50.0});
        let grant = serde_json::json!({"remaining_balance": 2.0});

        let extra = extra_section(Some(&limit), Some(&grant), 1).value.unwrap();

        assert!(extra.budget.is_none());
        assert_eq!(extra.balance.unwrap().minor_units, 200);
    }
}
