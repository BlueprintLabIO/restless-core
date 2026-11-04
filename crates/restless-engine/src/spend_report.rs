//! Best-effort owner reporting over the existing company ledgers. Estimates never change the
//! accounting fuse, and a missing measurement is never manufactured into a free request.
use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Utc};
use restless_model_gateway::{SpendRecord, SpendSettlement};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{finance, model_catalog, Daemon};

#[derive(Debug, Default, Deserialize)]
pub struct SpendReportQuery {
    pub days: Option<u16>,
    pub actor: Option<String>,
    pub model: Option<String>,
    pub before: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ModelTotals {
    pub calls: u64,
    pub tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    pub calls_without_tokens: u64,
    pub calls_without_split: u64,
    pub calls_without_cache: u64,
    pub calls_without_estimate: u64,
    pub unknown_charges: u64,
    pub subscription_calls: u64,
    /// What calls through a subscription sign-in consumed. Their limit is the plan's usage
    /// windows (`SpendReport::plan`), not dollars, so they are counted in tokens and kept out of
    /// `estimated_usd`.
    pub subscription_tokens: u64,
    pub subscription_input_tokens: u64,
    pub subscription_cached_input_tokens: u64,
    pub subscription_output_tokens: u64,
    /// Accounted ledger value, which can include tariffs and older records of unknown provenance.
    pub recorded_usd: f64,
    pub provider_reported_usd: Option<f64>,
    /// API-equivalent value of the calls billed per token (API keys). Subscription calls are not
    /// in it: pricing them at API rates showed the owner a dollar figure nobody pays.
    pub estimated_usd: Option<f64>,
}

impl ModelTotals {
    fn add(&mut self, row: &ModelSpendRow) {
        if row.billing == "subscription" {
            self.subscription_tokens = self
                .subscription_tokens
                .saturating_add(row.tokens.unwrap_or_default());
            self.subscription_input_tokens = self
                .subscription_input_tokens
                .saturating_add(row.input_tokens.unwrap_or_default());
            self.subscription_cached_input_tokens = self
                .subscription_cached_input_tokens
                .saturating_add(row.cached_input_tokens.unwrap_or_default());
            self.subscription_output_tokens = self
                .subscription_output_tokens
                .saturating_add(row.output_tokens.unwrap_or_default());
        } else {
            self.calls_without_estimate += u64::from(row.estimated_usd.is_none());
        }
        self.calls += 1;
        self.tokens = self.tokens.saturating_add(row.tokens.unwrap_or_default());
        self.input_tokens = self
            .input_tokens
            .saturating_add(row.input_tokens.unwrap_or_default());
        self.output_tokens = self
            .output_tokens
            .saturating_add(row.output_tokens.unwrap_or_default());
        self.cached_input_tokens = self
            .cached_input_tokens
            .saturating_add(row.cached_input_tokens.unwrap_or_default());
        self.calls_without_tokens += u64::from(row.tokens.is_none());
        self.calls_without_split +=
            u64::from(row.input_tokens.is_none() || row.output_tokens.is_none());
        self.calls_without_cache += u64::from(row.cached_input_tokens.is_none());
        self.unknown_charges += u64::from(row.cost_basis == "unreported");
        self.subscription_calls += u64::from(row.billing == "subscription");
        self.recorded_usd += row.recorded_usd.unwrap_or_default();
        if let Some(value) = row.estimated_usd {
            *self.estimated_usd.get_or_insert(0.0) += value;
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelSpendRow {
    pub id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub actor: String,
    pub person: String,
    pub model: String,
    pub responsibility: String,
    pub session_id: String,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub tokens: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub reasoning_output_tokens: Option<u64>,
    pub billing: String,
    pub billing_inferred: bool,
    pub recorded_usd: Option<f64>,
    pub cost_basis: &'static str,
    /// API-equivalent value of a call billed per token. None for a subscription call.
    pub estimated_usd: Option<f64>,
    pub estimate_basis: Option<String>,
    /// What a subscription call would have cost at API rates: shown on the call for comparison,
    /// never summed into a total.
    pub api_equivalent_usd: Option<f64>,
}

/// One usage window of a subscription sign-in this company uses, as the provider last reported
/// it on a call (passively; nothing is probed), with this company's share of it.
#[derive(Debug, Clone, Serialize)]
pub struct PlanWindow {
    pub account: String,
    pub provider: String,
    /// "5-hour" or "Weekly".
    pub label: String,
    pub used_percent: f64,
    pub resets_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    /// Percentage points of this window this company used, from the samples since its start.
    pub company_points: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SpendGroup {
    pub key: String,
    pub label: String,
    pub totals: ModelTotals,
}

#[derive(Debug, Serialize)]
pub struct SpendPayment {
    pub id: String,
    pub purpose: String,
    pub actor: String,
    pub work_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub state: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize)]
pub struct PaymentTotal {
    pub currency: String,
    pub settled_minor: i64,
    pub pending_minor: i64,
}

/// Team leads' model calls in the period, and how many ran on the everyday model (owner, 4 Nov:
/// "Lead turns: 70% everyday").
#[derive(Debug, Default, Clone, Serialize, PartialEq, Eq)]
pub struct LeadSplit {
    pub calls: u64,
    pub everyday_calls: u64,
    pub tokens: u64,
    pub everyday_tokens: u64,
    /// The everyday model those calls were counted against.
    pub everyday_model: String,
}

/// Over `rows`: the calls of the current team leads, and those on the everyday model (compared by
/// model id, whichever route served it: `anthropic/x` and `native-claude-oauth/x` are one model).
/// None when no lead made a call or the company has no everyday model.
pub fn lead_split(
    rows: &[ModelSpendRow],
    leads: &std::collections::HashSet<String>,
    everyday: Option<&str>,
) -> Option<LeadSplit> {
    let everyday = everyday?;
    let bare = |model: &str| model.rsplit('/').next().unwrap_or(model).to_string();
    let everyday_id = bare(everyday);
    let mut split = LeadSplit {
        everyday_model: everyday.to_string(),
        ..LeadSplit::default()
    };
    for row in rows.iter().filter(|row| leads.contains(&row.actor)) {
        let tokens = row.tokens.unwrap_or_default();
        split.calls += 1;
        split.tokens = split.tokens.saturating_add(tokens);
        if bare(&row.model) == everyday_id {
            split.everyday_calls += 1;
            split.everyday_tokens = split.everyday_tokens.saturating_add(tokens);
        }
    }
    (split.calls > 0).then_some(split)
}

#[derive(Debug, Serialize)]
pub struct SpendReport {
    pub observed_at: DateTime<Utc>,
    pub since: Option<DateTime<Utc>>,
    pub totals: ModelTotals,
    pub by_actor: Vec<SpendGroup>,
    pub by_model: Vec<SpendGroup>,
    pub by_day: Vec<SpendGroup>,
    pub activity: Vec<ModelSpendRow>,
    pub next_before: Option<String>,
    pub payments: Vec<SpendPayment>,
    pub payment_totals: Vec<PaymentTotal>,
    pub payments_status: &'static str,
    pub pricing_updated_at: Option<u64>,
    /// Catalogue rates for the models in this report, so a price shown is never a design value.
    pub prices: Vec<ModelPrice>,
    /// The subscription plans this company's calls count against: their limit is these windows.
    pub plan: Vec<PlanWindow>,
    /// Team leads' calls and the share on the everyday model, when leads made any.
    pub lead_turns: Option<LeadSplit>,
    pub coverage: Vec<String>,
}

/// A model's current public catalogue rates, in US dollars per million tokens: the same rates the
/// API-equivalent estimate uses. None when the catalogue has no input and output price for it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ModelPrice {
    pub model: String,
    pub currency: &'static str,
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cached_input_per_million: Option<f64>,
}

fn catalogue_price(model_id: &str, catalog: &serde_json::Value) -> Option<ModelPrice> {
    let (provider, model) = model_id.split_once('/')?;
    let provider = match provider {
        "openai-codex" | "native-codex-api" | "native-codex-oauth" | "litellm" => "openai",
        "native-claude-api" | "native-claude-oauth" => "anthropic",
        value => value,
    };
    let rates = &catalog["catalog"][provider]["models"][model]["cost"];
    let rate = |key: &str| {
        rates[key]
            .as_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
    };
    Some(ModelPrice {
        model: model_id.to_string(),
        currency: "USD",
        input_per_million: rate("input")?,
        output_per_million: rate("output")?,
        cached_input_per_million: rate("cache_read"),
    })
}

/// API-equivalent value at the current public catalogue snapshot. It is not an invoice, a
/// historical price assertion or a conversion to a ChatGPT allowance percentage.
fn estimate(record: &SpendRecord, catalog: Option<&serde_json::Value>) -> Option<(f64, String)> {
    if let Some(value) = record
        .native
        .as_ref()
        .and_then(|native| native.estimated_cost_micro_usd)
    {
        return Some((
            value as f64 / 1_000_000.0,
            "Harness-reported estimate".into(),
        ));
    }
    if record.native.is_some() && record.input_tokens == 0 && record.output_tokens == 0 {
        return None;
    }
    if record.input_tokens == 0 && record.output_tokens == 0 && record.total_tokens > 0 {
        return None;
    }
    if record.input_tokens == 0 && record.output_tokens == 0 && record.total_tokens == 0 {
        return None;
    }
    let ModelPrice {
        input_per_million: input_rate,
        output_per_million: output_rate,
        cached_input_per_million: cache_rate,
        ..
    } = catalogue_price(&record.model, catalog?)?;
    let cached = record.cached_input_tokens.unwrap_or_default();
    let ordinary = record.input_tokens.checked_sub(cached)?;
    let value = (ordinary as f64 * input_rate
        + cached as f64 * cache_rate.unwrap_or(input_rate)
        + record.output_tokens as f64 * output_rate)
        / 1_000_000.0;
    if !value.is_finite() {
        return None;
    }
    Some((
        value,
        if record.cached_input_tokens.is_none() || (cached > 0 && cache_rate.is_none()) {
            "Current catalogue rates; cache discounts unavailable; cache-write charges excluded"
        } else {
            "Current catalogue rates; cache-write charges excluded"
        }
        .into(),
    ))
}

/// The providers this company reaches through an account sign-in (`omp-oauth:`), from its
/// credential bindings. A relay record does not carry its billing (the spool rejects unknown
/// fields, so adding one would break a rollback); the binding that served it does.
pub fn subscription_providers(config: &crate::runtime::CompanyConfig) -> BTreeSet<String> {
    let mut providers = BTreeSet::new();
    for (key, reference) in &config.credentials {
        if !reference.starts_with("omp-oauth:") {
            continue;
        }
        if let Some(provider) = key.strip_prefix("model.inference.") {
            providers.insert(provider.to_string());
        } else if key == "model.inference" {
            if let Some((provider, _)) = config.model.split_once('/') {
                providers.insert(provider.to_string());
            }
        }
    }
    providers
}

/// How a relay record (no native turn) was billed. A call through a sign-in is accounted at zero
/// (`MeteredStream`: `ModelBilling::Subscription => Some(0)`); a per-token call with tokens never
/// is. So a zero-charge accounted record on a provider the company reaches by sign-in was a
/// subscription call. Codex is sign-in only.
fn relay_billing(record: &SpendRecord, subscription: &BTreeSet<String>) -> &'static str {
    let provider = record.model.split_once('/').map(|(provider, _)| provider);
    if provider == Some("openai-codex") {
        return "subscription";
    }
    if record.cost_micro_usd == 0
        && record.settlement == SpendSettlement::Accounted
        && provider.is_some_and(|provider| subscription.contains(provider))
    {
        return "subscription";
    }
    "unknown"
}

fn model_row(
    record: SpendRecord,
    catalog: Option<&serde_json::Value>,
    people: &HashMap<String, String>,
    subscription: &BTreeSet<String>,
) -> ModelSpendRow {
    let estimated = estimate(&record, catalog);
    let billing = record
        .native
        .as_ref()
        .map(|native| native.billing.clone())
        .unwrap_or_else(|| relay_billing(&record, subscription).into());
    let (estimated, api_equivalent) = if billing == "subscription" {
        (None, estimated.map(|(value, _)| value))
    } else {
        (estimated, None)
    };
    let cost_basis = if record.settlement == SpendSettlement::MeteringUnknown {
        "unreported"
    } else if billing == "subscription" {
        "subscription_or_external"
    } else if record.native.is_some() || (billing == "unknown" && record.cost_micro_usd == 0) {
        "unreported"
    } else {
        "legacy_recorded"
    };
    let split = record.native.is_none() && (record.input_tokens > 0 || record.output_tokens > 0);
    let tokens = record
        .native
        .as_ref()
        .and_then(|native| native.tokens)
        .or_else(|| {
            (record.total_tokens > 0 || split).then_some(
                record
                    .total_tokens
                    .max(record.input_tokens.saturating_add(record.output_tokens)),
            )
        });
    ModelSpendRow {
        id: record.request_id,
        occurred_at: record.occurred_at,
        person: people
            .get(&record.actor_id)
            .cloned()
            .unwrap_or_else(|| record.actor_id.clone()),
        actor: record.actor_id,
        model: record.model,
        responsibility: record.responsibility,
        session_id: record.session_id,
        work_id: record.work_id,
        attempt_id: record.attempt_id,
        tokens,
        input_tokens: split.then_some(record.input_tokens),
        output_tokens: split.then_some(record.output_tokens),
        cached_input_tokens: record.cached_input_tokens,
        reasoning_output_tokens: None,
        billing_inferred: record.native.is_none(),
        billing,
        recorded_usd: (record.native.is_none()
            && cost_basis != "subscription_or_external"
            && cost_basis != "unreported")
            .then_some(record.cost_micro_usd as f64 / 1_000_000.0),
        cost_basis,
        estimated_usd: estimated.as_ref().map(|(value, _)| *value),
        estimate_basis: estimated.map(|(_, basis)| basis),
        api_equivalent_usd: api_equivalent,
    }
}

/// The windows of each sign-in this company uses, with its own share of each.
async fn plan_windows(daemon: &Daemon, config: &crate::runtime::CompanyConfig) -> Vec<PlanWindow> {
    let accounts: BTreeMap<String, String> = config
        .credentials
        .iter()
        .filter(|(key, reference)| {
            key.starts_with("model.inference") && reference.starts_with("omp-oauth:")
        })
        .filter_map(|(_, reference)| {
            let account = crate::account_usage::account_of(reference)?;
            let provider = account.split_once('@')?.0.to_string();
            Some((account, provider))
        })
        .collect();
    if accounts.is_empty() {
        return Vec::new();
    }
    let views = match crate::account_usage::accounts(daemon.authority.pool(), Utc::now()).await {
        Ok(views) => views,
        Err(error) => {
            tracing::debug!(error = %format!("{error:#}"), "plan usage unavailable for the spend report");
            return Vec::new();
        }
    };
    views
        .into_iter()
        .filter_map(|view| Some((accounts.get(&view.account)?.clone(), view)))
        .flat_map(|(provider, view)| {
            let account = view.account;
            view.windows.into_iter().map(move |window| PlanWindow {
                account: account.clone(),
                provider: provider.clone(),
                company_points: window
                    .by_company
                    .iter()
                    .find(|(name, _)| name == &config.name)
                    .map(|(_, points)| *points),
                label: window.label,
                used_percent: window.used_percent,
                resets_at: window.resets_at,
                observed_at: window.observed_at,
            })
        })
        .collect()
}

fn groups(
    rows: &[ModelSpendRow],
    key: impl Fn(&ModelSpendRow) -> (String, String),
) -> Vec<SpendGroup> {
    let mut groups: BTreeMap<String, SpendGroup> = BTreeMap::new();
    for row in rows {
        let (key, label) = key(row);
        groups
            .entry(key.clone())
            .or_insert_with(|| SpendGroup {
                key,
                label,
                totals: ModelTotals::default(),
            })
            .totals
            .add(row);
    }
    groups.into_values().collect()
}

fn add_payment_total(
    totals: &mut BTreeMap<String, PaymentTotal>,
    currency: &str,
    amount: i64,
    state: finance::PaymentState,
) {
    let total = totals
        .entry(currency.into())
        .or_insert_with(|| PaymentTotal {
            currency: currency.into(),
            ..Default::default()
        });
    match state {
        finance::PaymentState::Settled => {
            total.settled_minor = total.settled_minor.saturating_add(amount)
        }
        finance::PaymentState::Rejected
        | finance::PaymentState::Cancelled
        | finance::PaymentState::Failed => {}
        _ => total.pending_minor = total.pending_minor.saturating_add(amount),
    }
}

pub async fn report(
    daemon: &Daemon,
    company: &str,
    query: SpendReportQuery,
) -> Result<SpendReport> {
    let days = query.days.unwrap_or(7);
    if days > 3660 {
        bail!("Choose a period of up to ten years, or all time.");
    }
    let now = Utc::now();
    let since = (days > 0).then(|| now - Duration::days(i64::from(days)));
    let cursor = query
        .before
        .as_ref()
        .map(|value| -> Result<(DateTime<Utc>, Uuid)> {
            let (time, id) = value
                .split_once('|')
                .ok_or_else(|| anyhow::anyhow!("Invalid spend cursor"))?;
            Ok((
                DateTime::parse_from_rfc3339(time)?.with_timezone(&Utc),
                Uuid::parse_str(id)?,
            ))
        })
        .transpose()?;
    let catalog = model_catalog::pricing_snapshot().await;
    let config = crate::runtime::CompanyConfig::load(&daemon.root, company).ok();
    let subscription = config
        .as_ref()
        .map(subscription_providers)
        .unwrap_or_default();
    let plan = match &config {
        Some(config) => plan_windows(daemon, config).await,
        None => Vec::new(),
    };
    let org = daemon.orgintel.get(company).await.ok();
    let people = match &org {
        Some(org) => org
            .list_actors_including_retired()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|actor| (actor.id, actor.display))
            .collect(),
        None => HashMap::new(),
    };
    let leads = match &org {
        Some(org) => org
            .list_teams()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|team| team.lead_actor_id)
            .collect(),
        None => std::collections::HashSet::new(),
    };
    let mut rows: Vec<_> = daemon
        .spend
        .report_records(company)?
        .into_iter()
        .filter(|record| since.is_none_or(|since| record.occurred_at >= since))
        .filter(|record| {
            query
                .actor
                .as_ref()
                .is_none_or(|actor| actor == &record.actor_id)
        })
        .filter(|record| {
            query
                .model
                .as_ref()
                .is_none_or(|model| model == &record.model)
        })
        .map(|record| model_row(record, catalog.as_ref(), &people, &subscription))
        .collect();
    rows.sort_by_key(|row| std::cmp::Reverse((row.occurred_at, row.id)));
    let mut totals = ModelTotals::default();
    for row in &rows {
        totals.add(row);
    }
    let lead_turns = lead_split(
        &rows,
        &leads,
        config
            .as_ref()
            .and_then(|config| config.everyday_for(config.coordination_harness))
            .as_deref(),
    );
    let mut by_actor = groups(&rows, |row| (row.actor.clone(), row.person.clone()));
    by_actor.sort_by_key(|group| std::cmp::Reverse(group.totals.tokens));
    let by_model = groups(&rows, |row| (row.model.clone(), row.model.clone()));
    // UTC day keys are explicit; exact activity timestamps are rendered in the owner's timezone.
    let by_day = groups(&rows, |row| {
        let day = row.occurred_at.format("%Y-%m-%d").to_string();
        (day.clone(), day)
    });
    let mut activity: Vec<_> = rows
        .into_iter()
        .filter(|row| cursor.is_none_or(|cursor| (row.occurred_at, row.id) < cursor))
        .take(101)
        .collect();
    let next_before = if activity.len() > 100 {
        activity.truncate(100);
        activity
            .last()
            .map(|row| format!("{}|{}", row.occurred_at.to_rfc3339(), row.id))
    } else {
        None
    };
    let payment_source = finance::payments(&daemon.authority, company).await;
    let payments_status = if payment_source.is_ok() {
        "available"
    } else {
        "unavailable"
    };
    let mut payment_totals: BTreeMap<String, PaymentTotal> = BTreeMap::new();
    let mut payments = Vec::new();
    for payment in payment_source.unwrap_or_default() {
        let time = payment.settled_at.unwrap_or(payment.updated_at);
        if since.is_some_and(|since| time < since)
            || query
                .actor
                .as_ref()
                .is_some_and(|actor| actor != &payment.request.requesting_actor)
            || query.model.is_some()
        {
            continue;
        }
        add_payment_total(
            &mut payment_totals,
            &payment.request.currency,
            payment.request.amount_minor,
            payment.state,
        );
        payments.push(SpendPayment {
            id: payment.request.idempotency_key,
            purpose: payment.request.purpose,
            actor: payment.request.requesting_actor,
            work_id: payment.request.work_id,
            amount_minor: payment.request.amount_minor,
            currency: payment.request.currency,
            state: payment.state.as_str().into(),
            occurred_at: time,
        });
    }
    payments.sort_by_key(|payment| std::cmp::Reverse(payment.occurred_at));
    let prices = catalog
        .as_ref()
        .map(|catalog| {
            by_model
                .iter()
                .filter_map(|group| catalogue_price(&group.key, catalog))
                .collect()
        })
        .unwrap_or_default();
    Ok(SpendReport {
        observed_at: now, since, totals, by_actor, by_model, by_day, activity, next_before, payments,
        payment_totals: payment_totals.into_values().collect(), payments_status,
        pricing_updated_at: catalog.as_ref().and_then(|catalog| catalog["updatedAt"].as_u64()),
        prices,
        plan,
        lead_turns,
        coverage: vec![
            "Calls through a subscription sign-in (Claude or ChatGPT) are counted in tokens and against the plan's usage windows, not in dollars. Older records are attributed by the company's current sign-in binding.".into(),
            "Estimates are API-equivalent token values at the current models.dev catalogue rates for calls billed per token, not invoices. They are separate from recorded spend and must not be added to it.".into(),
            "Catalogue base rates may omit cache writes, context/service-tier surcharges, negotiated discounts, fees and taxes. Their pricing provenance is not present in older request records.".into(),
            "Missing usage is unknown, not zero. Older records may lack billing provenance, cache-write counts or token splits. Reasoning tokens are already included in output tokens.".into(),
            "Payments are grouped by their recorded currency, with pending amounts separate from settled amounts. No exchange rate is assumed.".into(),
            "Only activity recorded by this company is covered. External Codex chats, unconnected provider bills, hosting, tool fees and off-platform purchases are not automatically observed.".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> SpendRecord {
        SpendRecord {
            request_id: Uuid::new_v4(),
            company_id: "usage_test".into(),
            model: "openai-codex/test".into(),
            input_tokens: 1000,
            output_tokens: 200,
            total_tokens: 1200,
            cached_input_tokens: Some(800),
            cost_micro_usd: 0,
            actor_id: "exec".into(),
            session_id: "session".into(),
            responsibility: "conversation".into(),
            work_id: None,
            attempt_id: None,
            settlement: SpendSettlement::Accounted,
            native: None,
            occurred_at: Utc::now(),
        }
    }
    fn catalog() -> serde_json::Value {
        serde_json::json!({"catalog":{"openai":{"models":{"test":{"cost":{"input":2,"output":10,"cache_read":0.1,"cache_write":2.5}}}}}})
    }

    fn none() -> BTreeSet<String> {
        BTreeSet::new()
    }

    /// Owner, 4 Nov: the Tokens view shows the share of lead turns on the everyday model, the
    /// same model whichever route served it, and only the current leads' calls.
    #[test]
    fn lead_turns_count_the_everyday_share() {
        let call = |actor: &str, model: &str| {
            let mut source = record();
            source.actor_id = actor.into();
            source.model = model.into();
            model_row(source, None, &HashMap::new(), &none())
        };
        let rows = vec![
            call("docs-lead", "anthropic/claude-sonnet-5-5"),
            call("docs-lead", "native-claude-oauth/claude-sonnet-5-5"),
            call("docs-lead", "anthropic/claude-opus-5-5"),
            call("maya", "anthropic/claude-sonnet-5-5"),
            call("exec", "anthropic/claude-opus-5-5"),
        ];
        let leads = std::collections::HashSet::from(["docs-lead".to_string()]);
        let split = lead_split(&rows, &leads, Some("anthropic/claude-sonnet-5-5")).unwrap();
        assert_eq!((split.calls, split.everyday_calls), (3, 2));
        assert_eq!((split.tokens, split.everyday_tokens), (3600, 2400));
        assert_eq!(lead_split(&rows, &leads, None), None);
        assert_eq!(
            lead_split(&rows, &std::collections::HashSet::new(), Some("anthropic/claude-sonnet-5-5")),
            None
        );
    }

    #[test]
    fn subscription_has_tokens_and_a_comparison_value_but_no_invented_charge() {
        let row = model_row(record(), Some(&catalog()), &HashMap::new(), &none());
        assert_eq!(row.tokens, Some(1200));
        assert_eq!(row.recorded_usd, None);
        assert_eq!(row.estimated_usd, None);
        assert!((row.api_equivalent_usd.unwrap() - 0.00248).abs() < 1e-10);
        let mut totals = ModelTotals::default();
        totals.add(&row);
        assert_eq!(totals.tokens, 1200);
        assert_eq!(totals.output_tokens, 200); // reasoning not counted twice
        assert_eq!(totals.subscription_calls, 1);
        assert_eq!(totals.subscription_tokens, 1200);
        assert_eq!(totals.subscription_cached_input_tokens, 800);
        assert_eq!(totals.recorded_usd, 0.0);
        assert_eq!(totals.estimated_usd, None);
        assert_eq!(totals.calls_without_estimate, 0);
    }

    /// Reno, 4 Nov: 18,464 Claude sign-in calls were reported as 18,468 unknown charges and
    /// US$917 of "estimated" spend. A zero-charge relay record on a provider the company reaches by
    /// sign-in is a subscription call; the same model on an API key keeps its dollars.
    #[test]
    fn claude_sign_in_calls_count_as_subscription_and_api_key_calls_keep_usd() {
        let config: crate::runtime::CompanyConfig = serde_json::from_value(serde_json::json!({
            "name": "reno_test", "mission": "Spend fixture", "model": "anthropic/claude-opus-5-5",
            "credentials": {"model.inference.anthropic": "omp-oauth:anthropic@0123abcd"}
        }))
        .unwrap();
        let signed_in = subscription_providers(&config);
        assert_eq!(signed_in, BTreeSet::from(["anthropic".to_string()]));
        let catalog = serde_json::json!({"catalog":{"anthropic":{"models":{"claude-opus-5-5":{"cost":{"input":5,"output":25,"cache_read":0.5}}}}}});
        let mut source = record();
        source.model = "anthropic/claude-opus-5-5".into();
        let subscription = model_row(source.clone(), Some(&catalog), &HashMap::new(), &signed_in);
        assert_eq!(subscription.billing, "subscription");
        assert_eq!(subscription.cost_basis, "subscription_or_external");
        assert_eq!(subscription.estimated_usd, None);
        assert!(subscription.api_equivalent_usd.is_some());
        let mut keyed = source.clone();
        keyed.cost_micro_usd = 4_500;
        let api = model_row(keyed, Some(&catalog), &HashMap::new(), &signed_in);
        assert_eq!(api.billing, "unknown");
        assert!(api.estimated_usd.is_some());
        let unbound = model_row(source, Some(&catalog), &HashMap::new(), &none());
        assert_eq!(unbound.cost_basis, "unreported");
        let mut totals = ModelTotals::default();
        for row in [&subscription, &api] {
            totals.add(row);
        }
        assert_eq!((totals.subscription_calls, totals.unknown_charges), (1, 0));
        assert_eq!(totals.estimated_usd, api.estimated_usd);

        let keyed_config: crate::runtime::CompanyConfig = serde_json::from_value(serde_json::json!({
            "name": "keyed_test", "mission": "Spend fixture", "model": "anthropic/claude-opus-5-5",
            "credentials": {"model.inference": "env:ANTHROPIC_API_KEY"}
        }))
        .unwrap();
        assert!(subscription_providers(&keyed_config).is_empty());
    }

    #[test]
    fn absent_usage_and_unpriced_model_remain_unknown() {
        let mut source = record();
        source.model = "another-provider/test".into();
        source.input_tokens = 0;
        source.output_tokens = 0;
        source.total_tokens = 0;
        source.cached_input_tokens = None;
        let row = model_row(source, Some(&catalog()), &HashMap::new(), &none());
        assert_eq!(row.tokens, None);
        assert_eq!(row.estimated_usd, None);
        let mut totals = ModelTotals::default();
        totals.add(&row);
        assert_eq!(totals.calls_without_tokens, 1);
        assert_eq!(totals.calls_without_estimate, 1);
        assert_eq!(totals.estimated_usd, None);
        assert!(estimate(&record(), None).is_none());
    }
    #[test]
    fn invalid_cache_counts_cannot_create_negative_estimates() {
        let mut source = record();
        source.cached_input_tokens = Some(1001);
        assert!(estimate(&source, Some(&catalog())).is_none());
    }

    #[test]
    fn payment_totals_preserve_currency_and_never_count_failed_or_pending_as_paid() {
        let mut totals = BTreeMap::new();
        for (currency, amount, state) in [
            ("USD", 100, finance::PaymentState::Settled),
            ("USD", 200, finance::PaymentState::Processing),
            ("USD", 300, finance::PaymentState::Failed),
            ("AUD", 400, finance::PaymentState::Settled),
            ("AUD", 500, finance::PaymentState::Rejected),
            ("AUD", 600, finance::PaymentState::Cancelled),
        ] {
            add_payment_total(&mut totals, currency, amount, state);
        }
        assert_eq!(totals.len(), 2);
        assert_eq!(
            (totals["USD"].settled_minor, totals["USD"].pending_minor),
            (100, 200)
        );
        assert_eq!(
            (totals["AUD"].settled_minor, totals["AUD"].pending_minor),
            (400, 0)
        );
    }

    #[test]
    fn external_api_and_legacy_zero_do_not_look_like_confirmed_free_calls() {
        let mut source = record();
        source.model = "another-provider/model".into();
        let row = model_row(source.clone(), None, &HashMap::new(), &none());
        assert_eq!(row.recorded_usd, None);
        assert_eq!(row.cost_basis, "unreported");
        source.native = Some(restless_model_gateway::NativeTurn {
            billing: "native_api_unmetered".into(),
            tokens: Some(40),
            estimated_cost_micro_usd: Some(2300),
        });
        let row = model_row(source, None, &HashMap::new(), &none());
        assert_eq!(row.recorded_usd, None);
        assert_eq!(row.estimated_usd, Some(0.0023));
        assert_eq!(row.cost_basis, "unreported");
    }
}
