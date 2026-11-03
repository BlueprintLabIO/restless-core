//! Standing permissions ("mandates") the owner grants once, of any kind.
//!
//! When an agent needs the owner's permission for a kind of action, it proposes a mandate. The
//! owner approves or declines it as one Inbox card, inside the same Undo window as every other
//! approval. Every action of that kind calls [`require`] before it acts. The answer is either
//! "allowed, under this mandate" or "needs a standing permission", and the second answer carries a
//! ready proposal, so an agent raises a card instead of being stuck.
//!
//! Email is the first kind: its one-use permits, reservations and provider outcomes live in
//! `email_mandate.rs` and keep their exact-text binding. Ordering from a vendor is the second:
//! only the mandate and the check, never a payment.
//!
//! Records live in Authority's append-only table: `mandate_proposal`,
//! `mandate_proposal_decision`, `mandate_granted`, `mandate_revoked` and `mandate_used`. A
//! grant's record id is its version. The email-only kinds that came before (`email_mandate_*`)
//! are rewritten in place into these by [`migrate_email_records`], keeping each record's id.

use std::str::FromStr;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

pub use crate::authority::{
    EmailMandate, EmailMandateUsage, EmailPermit, EmailPermitProposal, EmailReservation,
    NewEmailMandate, ProviderOutcome,
};

pub const PROPOSAL_KIND: &str = "mandate_proposal";
pub const DECISION_KIND: &str = "mandate_proposal_decision";
pub const GRANTED_KIND: &str = "mandate_granted";
pub const REVOKED_KIND: &str = "mandate_revoked";
pub const USED_KIND: &str = "mandate_used";

/// The longest a standing permission may run before the owner is asked again.
const MAX_LIFETIME_DAYS: i64 = 366;

/// What a standing permission lets the company do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MandateKind {
    /// Send email to people who fit an approved purpose and audience.
    Email,
    /// Place an order with a vendor, up to a money limit per order.
    VendorOrder,
}

impl MandateKind {
    pub const ALL: [MandateKind; 2] = [MandateKind::Email, MandateKind::VendorOrder];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::VendorOrder => "vendor_order",
        }
    }

    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "email" => Ok(Self::Email),
            "vendor_order" | "order" | "spend" => Ok(Self::VendorOrder),
            other => bail!(
                "unknown standing permission kind {other:?}; the kinds are {}",
                Self::ALL.map(Self::as_str).join(", ")
            ),
        }
    }

    /// What the permission is for, completing "permission to …".
    pub fn purpose(self) -> &'static str {
        match self {
            Self::Email => "send email",
            Self::VendorOrder => "order from a vendor",
        }
    }

    /// One action and several, in plain words.
    pub fn unit(self) -> (&'static str, &'static str) {
        match self {
            Self::Email => ("email", "emails"),
            Self::VendorOrder => ("order", "orders"),
        }
    }

    /// Whether only one permission of this kind may be active at a time.
    fn single_active(self) -> bool {
        matches!(self, Self::Email)
    }

    /// How an agent works under an active permission of this kind.
    pub fn how_to_act(self) -> &'static str {
        match self {
            Self::Email => "Send the first email under it to the owner's own address as a test. Then permit each real email with `restless mandate permit` (it binds sender, recipient and the exact text) and send it with `restless email send`.",
            Self::VendorOrder => "Before each order, run `restless mandate use --kind vendor_order --vendor <vendor> --amount <total> --currency <code> --summary <what>`; it records the order against the limits, or says why not. Place the order only when it says allowed.",
        }
    }
}

impl std::fmt::Display for MandateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An amount of money in minor units (cents), with its ISO 4217 currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "MoneyInput")]
pub struct Money {
    pub minor: i64,
    pub currency: String,
}

/// Money as a proposal or a check may write it: `{"amount": "150.00", "currency": "AUD"}` or
/// `{"minor": 15000, "currency": "AUD"}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MoneyInput {
    #[serde(default)]
    minor: Option<i64>,
    #[serde(default)]
    amount: Option<Value>,
    currency: String,
}

impl TryFrom<MoneyInput> for Money {
    type Error = anyhow::Error;

    fn try_from(input: MoneyInput) -> Result<Self> {
        let currency = normalize_currency(&input.currency)?;
        let minor = match (input.minor, input.amount) {
            (Some(minor), None) => minor,
            (None, Some(Value::String(text))) => parse_minor(&text, &currency)?,
            (None, Some(Value::Number(number))) => parse_minor(&number.to_string(), &currency)?,
            _ => bail!("money needs exactly one of `amount` or `minor`, and a currency"),
        };
        if minor <= 0 {
            bail!("an amount of money must be more than zero");
        }
        Ok(Self { minor, currency })
    }
}

fn normalize_currency(raw: &str) -> Result<String> {
    let code = match raw.trim() {
        "A$" => "AUD".to_string(),
        "US$" => "USD".to_string(),
        "NZ$" => "NZD".to_string(),
        other => other.to_ascii_uppercase(),
    };
    if code.len() != 3 || !code.bytes().all(|b| b.is_ascii_uppercase()) {
        bail!("currency must be a three-letter ISO code such as AUD");
    }
    Ok(code)
}

fn minor_digits(currency: &str) -> u32 {
    match currency {
        "JPY" | "KRW" | "VND" | "CLP" | "ISK" => 0,
        _ => 2,
    }
}

/// "150", "150.5" or "1,150.50" in the currency's minor units.
pub fn parse_minor(text: &str, currency: &str) -> Result<i64> {
    let digits = minor_digits(currency);
    let cleaned: String = text
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit() && c != '.')
        .chars()
        .filter(|c| *c != ',')
        .collect();
    let (whole, fraction) = cleaned.split_once('.').unwrap_or((&cleaned, ""));
    if whole.is_empty() && fraction.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > digits as usize
    {
        bail!("{text:?} is not an amount of {currency}");
    }
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().context("amount is too large")?
    };
    let mut fraction_value: i64 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().context("amount is not valid")?
    };
    for _ in fraction.len()..digits as usize {
        fraction_value *= 10;
    }
    whole
        .checked_mul(10_i64.pow(digits))
        .and_then(|value| value.checked_add(fraction_value))
        .context("amount is too large")
}

impl Money {
    pub fn new(minor: i64, currency: &str) -> Result<Self> {
        Money::try_from(MoneyInput {
            minor: Some(minor),
            amount: None,
            currency: currency.to_string(),
        })
    }

    /// "A$150" or "A$150.50": the way the owner reads it.
    pub fn plain(&self) -> String {
        let symbol = match self.currency.as_str() {
            "AUD" => "A$",
            "USD" => "US$",
            "NZD" => "NZ$",
            "CAD" => "C$",
            "EUR" => "€",
            "GBP" => "£",
            "JPY" => "¥",
            _ => "",
        };
        let digits = minor_digits(&self.currency);
        let scale = 10_i64.pow(digits);
        let whole = self.minor / scale;
        let fraction = self.minor % scale;
        let mut grouped = String::new();
        let text = whole.to_string();
        for (index, ch) in text.chars().enumerate() {
            if index > 0 && (text.len() - index) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(ch);
        }
        let amount = if fraction == 0 {
            grouped
        } else {
            format!("{grouped}.{fraction:0width$}", width = digits as usize)
        };
        if symbol.is_empty() {
            format!("{} {amount}", self.currency)
        } else {
            format!("{symbol}{amount}")
        }
    }
}

/// The limits a standing permission holds the company to. Each is optional; a kind may require
/// some (email needs both counts; a vendor order needs a money cap per order).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MandateLimits {
    /// How many actions in total.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_total: Option<u32>,
    /// How many actions a day (the rate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_per_day: Option<u32>,
    /// Where a day starts and ends, as an IANA timezone; UTC when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// The most one action may cost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_per_action: Option<Money>,
    /// The most all actions together may cost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_spend_total: Option<Money>,
    /// Only these recipients, vendors or domains. Empty means anyone the scope describes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counterparties: Vec<String>,
}

/// What an email permission adds to the shared terms: the purpose and audience Exec judges each
/// recipient against, and the one address it sends from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailTerms {
    pub purpose: String,
    pub audience_guidance: String,
    pub sender: String,
    #[serde(default)]
    pub sender_name: Option<String>,
}

/// A standing permission as it is proposed: everything the owner approves, exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewMandate {
    pub kind: MandateKind,
    /// The permission in a few plain words: "Print fridge magnets with Vistaprint".
    pub title: String,
    /// What it covers, in a sentence or two.
    pub scope: String,
    #[serde(default)]
    pub limits: MandateLimits,
    pub expires_at: DateTime<Utc>,
    /// Whether the owner still approves each action. Without it, the agent acts within the
    /// limits on its own judgement.
    #[serde(default)]
    pub owner_approves_each: bool,
    /// Why the company needs it.
    #[serde(default)]
    pub rationale: String,
    /// What the agent saw that makes the case: links, file paths, quotes.
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<EmailTerms>,
}

/// A granted standing permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mandate {
    pub id: Uuid,
    pub kind: MandateKind,
    pub title: String,
    pub scope: String,
    #[serde(default)]
    pub limits: MandateLimits,
    pub expires_at: DateTime<Utc>,
    #[serde(default)]
    pub owner_approves_each: bool,
    #[serde(default)]
    pub rationale: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<EmailTerms>,
    /// Who asked for it.
    #[serde(default)]
    pub proposed_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A proposal still waiting for the owner.
#[derive(Debug, Clone, Serialize)]
pub struct MandateProposal {
    pub id: Uuid,
    pub record_id: i64,
    pub proposed_by: Option<String>,
    pub terms: NewMandate,
    pub replaces: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// What a standing permission has been used for so far.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MandateUsage {
    pub usage_day: String,
    pub used_today: u32,
    pub used_total: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spent_total: Option<Money>,
}

/// One action an agent is about to take.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MandateAction {
    /// What it is, in plain words: "Order 200 fridge magnets".
    #[serde(default)]
    pub summary: String,
    /// The recipient, vendor or domain.
    #[serde(default)]
    pub counterparty: Option<String>,
    /// What it costs in total.
    #[serde(default)]
    pub amount: Option<Money>,
    /// A key that names this one action, so recording it twice counts once.
    #[serde(default)]
    pub key: Option<String>,
}

/// The answer to [`require`].
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MandateCheck {
    Allowed {
        kind: MandateKind,
        mandate_id: Uuid,
        mandate_version: i64,
        title: String,
        owner_approves_each: bool,
        /// What is left after this action.
        remaining: String,
        how: String,
    },
    NeedsPermission {
        kind: MandateKind,
        /// Why no standing permission covers it, in plain words.
        reason: String,
        /// A proposal already waiting for the owner; do not propose again.
        #[serde(skip_serializing_if = "Option::is_none")]
        pending_proposal: Option<Uuid>,
        /// A ready proposal for this action, to adjust and raise.
        #[serde(skip_serializing_if = "Option::is_none")]
        proposal: Option<NewMandate>,
        /// The current permission a raised proposal would change, if any.
        #[serde(skip_serializing_if = "Option::is_none")]
        replaces: Option<Uuid>,
        how: String,
    },
}

impl MandateCheck {
    pub fn allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. })
    }
}

// ---------------------------------------------------------------------------------------------
// Email, the first kind.

impl NewMandate {
    /// An email permission from the shape email proposals had before mandates were generic.
    pub fn from_email(proposal: NewEmailMandate, rationale: Option<&str>) -> Self {
        Self {
            kind: MandateKind::Email,
            title: email_title(&proposal.sender, proposal.sender_name.as_deref()),
            scope: proposal.purpose.clone(),
            limits: MandateLimits {
                max_total: Some(proposal.max_total),
                max_per_day: Some(proposal.max_per_day),
                timezone: Some(proposal.timezone.clone()),
                ..MandateLimits::default()
            },
            expires_at: proposal.expires_at,
            owner_approves_each: false,
            rationale: rationale.unwrap_or_default().trim().to_string(),
            evidence: Vec::new(),
            email: Some(EmailTerms {
                purpose: proposal.purpose,
                audience_guidance: proposal.audience_guidance,
                sender: proposal.sender,
                sender_name: proposal.sender_name,
            }),
        }
    }

    /// The email-only view the send path checks against.
    pub fn email_view(&self) -> Option<NewEmailMandate> {
        let email = self.email.as_ref()?;
        Some(NewEmailMandate {
            purpose: email.purpose.clone(),
            audience_guidance: email.audience_guidance.clone(),
            sender: email.sender.clone(),
            sender_name: email.sender_name.clone(),
            max_per_day: self.limits.max_per_day.unwrap_or_default(),
            max_total: self.limits.max_total.unwrap_or_default(),
            timezone: self.limits.timezone.clone().unwrap_or_else(|| "UTC".into()),
            expires_at: self.expires_at,
        })
    }
}

fn email_title(sender: &str, sender_name: Option<&str>) -> String {
    match sender_name {
        Some(name) if !name.trim().is_empty() => {
            format!("Send email as {} <{}>", name.trim(), sender.trim())
        }
        _ => format!("Send email as {}", sender.trim()),
    }
}

impl Mandate {
    /// The email-only view the send path checks against.
    pub fn as_email(&self) -> Option<EmailMandate> {
        if self.kind != MandateKind::Email {
            return None;
        }
        let email = self.email.as_ref()?;
        Some(EmailMandate {
            id: self.id,
            purpose: email.purpose.clone(),
            audience_guidance: email.audience_guidance.clone(),
            sender: email.sender.clone(),
            sender_name: email.sender_name.clone(),
            max_per_day: self.limits.max_per_day.unwrap_or_default(),
            max_total: self.limits.max_total.unwrap_or_default(),
            timezone: self.limits.timezone.clone().unwrap_or_else(|| "UTC".into()),
            expires_at: self.expires_at,
            created_at: self.created_at,
        })
    }

    fn from_email(mandate: EmailMandate) -> Self {
        let terms = NewMandate::from_email(
            NewEmailMandate {
                purpose: mandate.purpose,
                audience_guidance: mandate.audience_guidance,
                sender: mandate.sender,
                sender_name: mandate.sender_name,
                max_per_day: mandate.max_per_day,
                max_total: mandate.max_total,
                timezone: mandate.timezone,
                expires_at: mandate.expires_at,
            },
            None,
        );
        Self::granted(mandate.id, terms, None, mandate.created_at)
    }

    fn granted(
        id: Uuid,
        terms: NewMandate,
        proposed_by: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            kind: terms.kind,
            title: terms.title,
            scope: terms.scope,
            limits: terms.limits,
            expires_at: terms.expires_at,
            owner_approves_each: terms.owner_approves_each,
            rationale: terms.rationale,
            evidence: terms.evidence,
            email: terms.email,
            proposed_by,
            created_at,
        }
    }

    pub fn terms(&self) -> NewMandate {
        NewMandate {
            kind: self.kind,
            title: self.title.clone(),
            scope: self.scope.clone(),
            limits: self.limits.clone(),
            expires_at: self.expires_at,
            owner_approves_each: self.owner_approves_each,
            rationale: self.rationale.clone(),
            evidence: self.evidence.clone(),
            email: self.email.clone(),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Validation and plain words.

fn bounded(label: &str, value: &str, max: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("a standing permission needs a {label}");
    }
    if value.chars().count() > max {
        bail!("the {label} is longer than {max} characters");
    }
    if value.chars().any(|c| c.is_control() && c != '\n') {
        bail!("the {label} must not contain control characters");
    }
    Ok(value.to_string())
}

/// A recipient, vendor or domain the way it is compared: lower case, without a scheme, `www.`
/// or path. Email addresses stay whole.
pub fn normalize_counterparty(raw: &str) -> String {
    let value = raw.trim().to_ascii_lowercase();
    if value.contains('@') && !value.contains("://") {
        return value;
    }
    let value = value
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    value
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_string()
}

/// Whether `actual` is one of the counterparties a permission names: the same address, vendor or
/// domain, or an address or subdomain under a named domain.
pub fn counterparty_matches(allowed: &str, actual: &str) -> bool {
    let allowed = normalize_counterparty(allowed);
    let actual = normalize_counterparty(actual);
    if allowed.is_empty() || actual.is_empty() {
        return false;
    }
    if allowed == actual {
        return true;
    }
    let host = actual
        .rsplit_once('@')
        .map_or(actual.as_str(), |(_, host)| host);
    host == allowed || host.ends_with(&format!(".{allowed}"))
}

/// Check and tidy a proposal before it reaches the owner.
pub fn canonicalize(mut terms: NewMandate) -> Result<NewMandate> {
    terms.title = bounded("title", &terms.title, 160)?;
    terms.scope = bounded("scope", &terms.scope, 2_000)?;
    terms.rationale = terms.rationale.trim().to_string();
    if terms.rationale.chars().count() > 4_000 {
        bail!("the rationale is longer than 4,000 characters");
    }
    terms.evidence = terms
        .evidence
        .iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect();
    if terms.evidence.len() > 16 || terms.evidence.iter().any(|entry| entry.len() > 1_000) {
        bail!("evidence is at most 16 entries of at most 1,000 characters each");
    }
    let now = Utc::now();
    if terms.expires_at <= now {
        bail!("a standing permission must expire in the future");
    }
    if terms.expires_at > now + chrono::Duration::days(MAX_LIFETIME_DAYS) {
        bail!("a standing permission runs for at most a year; ask again after that");
    }
    let limits = &mut terms.limits;
    if limits.max_total == Some(0) || limits.max_per_day == Some(0) {
        bail!("count limits must be at least one");
    }
    if let (Some(daily), Some(total)) = (limits.max_per_day, limits.max_total) {
        if daily > total {
            bail!("the daily limit cannot be higher than the total limit");
        }
    }
    if let Some(timezone) = limits.timezone.as_deref() {
        Tz::from_str(timezone.trim()).context("timezone must be a valid IANA timezone")?;
        limits.timezone = Some(timezone.trim().to_string());
    }
    for money in [&limits.max_per_action, &limits.max_spend_total]
        .into_iter()
        .flatten()
    {
        if money.minor <= 0 {
            bail!("money limits must be more than zero");
        }
    }
    if let (Some(each), Some(total)) = (&limits.max_per_action, &limits.max_spend_total) {
        if each.currency != total.currency {
            bail!("the per-action and total money limits must be in the same currency");
        }
        if each.minor > total.minor {
            bail!("the per-action money limit cannot be higher than the total");
        }
    }
    let mut counterparties = Vec::new();
    for raw in &limits.counterparties {
        let value = normalize_counterparty(raw);
        if value.is_empty() || value.len() > 320 || value.chars().any(char::is_whitespace) {
            bail!("{raw:?} is not a recipient, vendor or domain");
        }
        if !counterparties.contains(&value) {
            counterparties.push(value);
        }
    }
    if counterparties.len() > 50 {
        bail!("a standing permission names at most 50 recipients, vendors or domains");
    }
    limits.counterparties = counterparties;
    match terms.kind {
        MandateKind::Email => {
            if terms.owner_approves_each {
                bail!("an email permission lets Exec permit each email against its exact text; without one, each email already waits for the owner");
            }
            let view = terms
                .email_view()
                .context("an email permission needs its purpose, audience_guidance and sender")?;
            if terms.limits.max_per_day.is_none() || terms.limits.max_total.is_none() {
                bail!("an email permission needs a daily and a total limit");
            }
            if terms.limits.max_per_action.is_some() || terms.limits.max_spend_total.is_some() {
                bail!("an email permission has no money limits");
            }
            let view = crate::authority::canonicalize_new_email_mandate(view)?;
            terms.limits.timezone = Some(view.timezone.clone());
            terms.email = Some(EmailTerms {
                purpose: view.purpose,
                audience_guidance: view.audience_guidance,
                sender: view.sender,
                sender_name: view.sender_name,
            });
        }
        MandateKind::VendorOrder => {
            if terms.email.is_some() {
                bail!("email terms belong to an email permission");
            }
            if terms.limits.max_per_action.is_none() {
                bail!("a vendor order permission needs a money limit per order (max_per_action)");
            }
        }
    }
    Ok(terms)
}

/// Each limit as one line the owner reads.
pub fn limit_lines(kind: MandateKind, limits: &MandateLimits) -> Vec<String> {
    let (one, many) = kind.unit();
    let count = |n: u32| format!("{n} {}", if n == 1 { one } else { many });
    let mut lines = Vec::new();
    if let Some(money) = &limits.max_per_action {
        lines.push(format!("Up to {} per {one}", money.plain()));
    }
    if let Some(money) = &limits.max_spend_total {
        lines.push(format!("Up to {} in total", money.plain()));
    }
    match (limits.max_per_day, limits.max_total) {
        (Some(daily), Some(total)) => lines.push(format!(
            "At most {} a day, {} in total",
            count(daily),
            count(total)
        )),
        (Some(daily), None) => lines.push(format!("At most {} a day", count(daily))),
        (None, Some(total)) => lines.push(format!("At most {} in total", count(total))),
        (None, None) => {}
    }
    if limits.max_per_day.is_some() {
        if let Some(zone) = limits.timezone.as_deref().filter(|zone| *zone != "UTC") {
            lines.push(format!("Days counted in {zone} time"));
        }
    }
    if !limits.counterparties.is_empty() {
        lines.push(format!("Only with {}", limits.counterparties.join(", ")));
    }
    lines
}

/// What a permission has used so far, in a few words: "1 of 5 today · 3 of 100 emails".
pub fn usage_line(mandate: &Mandate, usage: &MandateUsage) -> String {
    let (one, many) = mandate.kind.unit();
    let noun = |n: u32| if n == 1 { one } else { many };
    let limits = &mandate.limits;
    let mut parts = Vec::new();
    if let Some(daily) = limits.max_per_day {
        parts.push(format!("{} of {daily} today", usage.used_today));
    }
    match limits.max_total {
        Some(total) => parts.push(format!("{} of {total} {}", usage.used_total, noun(total))),
        None => parts.push(format!(
            "{} {} so far",
            usage.used_total,
            noun(usage.used_total)
        )),
    }
    let spent = usage.spent_total.clone();
    match (&limits.max_spend_total, spent) {
        (Some(cap), spent) => parts.push(format!(
            "{} of {} spent",
            spent
                .filter(|spent| spent.currency == cap.currency)
                .map_or_else(
                    || Money {
                        minor: 0,
                        currency: cap.currency.clone()
                    }
                    .plain(),
                    |spent| spent.plain()
                ),
            cap.plain()
        )),
        (None, Some(spent)) => parts.push(format!("{} spent", spent.plain())),
        (None, None) => {}
    }
    parts.join(" · ")
}

/// What the owner approves, line by line, as `Label: value`. The Inbox card shows exactly this.
pub fn plain_terms(terms: &NewMandate) -> Vec<(String, String)> {
    let mut rows = vec![("Allows".to_string(), terms.title.clone())];
    rows.push(("Covers".into(), terms.scope.clone()));
    if let Some(email) = &terms.email {
        if email.purpose.trim() != terms.scope.trim() {
            rows.push(("Purpose".into(), email.purpose.clone()));
        }
        rows.push(("Audience".into(), email.audience_guidance.clone()));
        rows.push((
            "From".into(),
            match email.sender_name.as_deref() {
                Some(name) => format!("{name} <{}>", email.sender),
                None => email.sender.clone(),
            },
        ));
    }
    let limits = limit_lines(terms.kind, &terms.limits);
    rows.push((
        "Limits".into(),
        if limits.is_empty() {
            "None beyond the scope and the date".into()
        } else {
            limits.join("; ")
        },
    ));
    let (one, _) = terms.kind.unit();
    rows.push((
        "Each one".into(),
        match (terms.kind, terms.owner_approves_each) {
            (_, true) => format!("You still approve each {one} before it happens"),
            (MandateKind::Email, false) => "Exec checks each email against its exact text first; the first goes to you as a test".into(),
            (_, false) => format!("The company places each {one} within these limits without asking you again"),
        },
    ));
    rows.push((
        "Until".into(),
        terms.expires_at.format("%-d %b %Y").to_string(),
    ));
    rows
}

// ---------------------------------------------------------------------------------------------
// Records.

/// Serialize every change to standing permissions and every use of one, per company.
pub async fn lock_company(tx: &mut Transaction<'_, Postgres>, company: &str) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(format!("mandate-authority:{company}"))
        .execute(&mut **tx)
        .await
        .context("serialize standing permission operation")?;
    Ok(())
}

async fn is_revoked(
    tx: &mut Transaction<'_, Postgres>,
    company: &str,
    mandate_id: Uuid,
    version: Option<i64>,
) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.records \
         WHERE company=$1 AND kind='mandate_revoked' AND body->>'mandate_id'=$2 \
           AND ($3::bigint IS NULL OR (body->>'mandate_version')::bigint=$3))",
    )
    .bind(company)
    .bind(mandate_id.to_string())
    .bind(version)
    .fetch_one(&mut **tx)
    .await?)
}

/// The current version of one permission, refusing a revoked one.
pub async fn current(
    tx: &mut Transaction<'_, Postgres>,
    company: &str,
    mandate_id: Uuid,
) -> Result<(Mandate, i64)> {
    let row = sqlx::query(
        "SELECT id, body FROM restless_authority.records \
         WHERE company=$1 AND kind='mandate_granted' AND body->>'id'=$2 \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(company)
    .bind(mandate_id.to_string())
    .fetch_optional(&mut **tx)
    .await?
    .context("this standing permission does not exist")?;
    let version: i64 = row.try_get("id")?;
    let mandate: Mandate =
        serde_json::from_value(row.try_get("body")?).context("decode standing permission")?;
    if is_revoked(tx, company, mandate_id, Some(version)).await? {
        bail!("this standing permission has been revoked");
    }
    Ok((mandate, version))
}

/// Every granted permission that has not been revoked, newest first, with its version. Expired
/// ones are included; callers that act check the date.
pub async fn list_versions(pool: &PgPool, company: &str) -> Result<Vec<(Mandate, i64)>> {
    let rows = sqlx::query(
        "SELECT g.id, g.body FROM restless_authority.records g \
         WHERE g.company=$1 AND g.kind='mandate_granted' \
           AND NOT EXISTS (SELECT 1 FROM restless_authority.records r \
             WHERE r.company=g.company AND r.kind='mandate_revoked' AND r.body->>'mandate_id'=g.body->>'id') \
         ORDER BY g.id DESC",
    )
    .bind(company)
    .fetch_all(pool)
    .await?;
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        let mandate: Mandate = serde_json::from_value(row.try_get("body")?)?;
        if seen.insert(mandate.id) {
            result.push((mandate, row.try_get("id")?));
        }
    }
    Ok(result)
}

pub async fn list(pool: &PgPool, company: &str) -> Result<Vec<Mandate>> {
    Ok(list_versions(pool, company)
        .await?
        .into_iter()
        .map(|(mandate, _)| mandate)
        .collect())
}

/// Permissions that can be acted under now.
pub async fn active(pool: &PgPool, company: &str) -> Result<Vec<(Mandate, i64)>> {
    let now = Utc::now();
    Ok(list_versions(pool, company)
        .await?
        .into_iter()
        .filter(|(mandate, _)| mandate.expires_at > now)
        .collect())
}

pub async fn propose(
    pool: &PgPool,
    company: &str,
    proposer: &str,
    terms: NewMandate,
    replaces: Option<Uuid>,
) -> Result<Uuid> {
    if proposer.trim().is_empty() {
        bail!("a standing permission proposal needs the actor who asks");
    }
    let terms = canonicalize(terms)?;
    if let Some(replaces) = replaces {
        let mut tx = pool.begin().await?;
        let (current, _) = current(&mut tx, company, replaces).await?;
        tx.commit().await?;
        if current.kind != terms.kind {
            bail!("a change replaces a permission of the same kind");
        }
    }
    let id = Uuid::new_v4();
    let mut body = serde_json::json!({ "id": id, "kind": terms.kind, "terms": terms });
    if let Some(replaces) = replaces {
        body["replaces"] = serde_json::json!(replaces);
    }
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'mandate_proposal',$2,$3)",
    )
    .bind(company)
    .bind(proposer)
    .bind(body)
    .execute(pool)
    .await?;
    Ok(id)
}

fn proposal_from(record: crate::authority::AuthorityRecord) -> Result<MandateProposal> {
    let id = record.body["id"]
        .as_str()
        .and_then(|id| Uuid::parse_str(id).ok())
        .context("a standing permission proposal has no id")?;
    let terms: NewMandate = serde_json::from_value(record.body["terms"].clone())
        .context("decode standing permission proposal")?;
    Ok(MandateProposal {
        id,
        record_id: record.id,
        proposed_by: record.actor_id,
        terms,
        replaces: record.body["replaces"]
            .as_str()
            .and_then(|id| Uuid::parse_str(id).ok()),
        created_at: record.created_at,
    })
}

/// Raise a proposal an agent wrote as one Inbox card. The proposal may say its kind inside or
/// take it from `kind`; the email shape that came before generic permissions (purpose, sender,
/// caps, no kind) still reads as email. A proposal of the same kind already waiting is named
/// instead of a second card, and a change to a single-active kind (email) names the current
/// permission, so approving replaces it.
pub async fn raise(
    pool: &PgPool,
    company: &str,
    actor: &str,
    mut value: Value,
    kind: Option<&str>,
    reason: Option<&str>,
) -> Result<Value> {
    let legacy_email = value.get("kind").is_none() && value.get("sender").is_some();
    let mut terms: NewMandate = if legacy_email {
        let email: NewEmailMandate = serde_json::from_value(value)
            .map_err(|error| anyhow::anyhow!("invalid email permission proposal: {error}"))?;
        NewMandate::from_email(email, reason)
    } else {
        if let (Some(kind), Some(object)) = (kind, value.as_object_mut()) {
            let kind = MandateKind::parse(kind)?;
            object
                .entry("kind")
                .or_insert_with(|| serde_json::json!(kind));
        }
        serde_json::from_value(value)
            .map_err(|error| anyhow::anyhow!("invalid standing permission proposal: {error}"))?
    };
    if let Some(kind) = kind {
        if MandateKind::parse(kind)? != terms.kind {
            bail!(
                "--kind {kind} does not match the proposal's kind {}",
                terms.kind
            );
        }
    }
    if terms.rationale.trim().is_empty() {
        terms.rationale = reason.unwrap_or_default().trim().to_string();
    }
    if terms.rationale.is_empty() {
        bail!("say why the company needs this, in a sentence the owner reads (--reason)");
    }
    if let Some(waiting) = pending(pool, company)
        .await?
        .into_iter()
        .rev()
        .find(|proposal| proposal.terms.kind == terms.kind)
    {
        return Ok(serde_json::json!({
            "proposal_id": waiting.id,
            "status": "waiting_for_owner",
            "title": waiting.terms.title,
            "note": format!("A proposal (“{}”) is already waiting in the owner's Inbox. Do not propose again; you are told when they answer.", waiting.terms.title),
        }));
    }
    let replaces = if terms.kind.single_active() {
        active(pool, company)
            .await?
            .into_iter()
            .find(|(mandate, _)| mandate.kind == terms.kind)
            .map(|(mandate, _)| mandate.id)
    } else {
        None
    };
    let title = terms.title.clone();
    let id = propose(pool, company, actor, terms, replaces).await?;
    Ok(serde_json::json!({
        "proposal_id": id,
        "status": "waiting_for_owner",
        "title": title,
        "replaces": replaces,
        "note": "Nothing is allowed under it yet. The owner approves or declines exactly this in their Inbox; you are told when they answer. Continue other work meanwhile.",
    }))
}

/// Proposals the owner has not answered, oldest first.
pub async fn pending(pool: &PgPool, company: &str) -> Result<Vec<MandateProposal>> {
    let records: Vec<crate::authority::AuthorityRecord> = sqlx::query_as(
        "SELECT p.id,p.actor_id,p.body,p.created_at FROM restless_authority.records p \
         WHERE p.company=$1 AND p.kind='mandate_proposal' AND NOT EXISTS (\
           SELECT 1 FROM restless_authority.records d WHERE d.company=p.company \
           AND d.kind='mandate_proposal_decision' AND d.body->>'proposal_id'=p.body->>'id') \
         ORDER BY p.id",
    )
    .bind(company)
    .fetch_all(pool)
    .await?;
    let mut proposals = Vec::new();
    for record in records {
        match proposal_from(record) {
            Ok(proposal) => proposals.push(proposal),
            Err(error) => {
                tracing::warn!(company, "skipping unreadable mandate proposal: {error:#}")
            }
        }
    }
    Ok(proposals)
}

async fn grant_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    company: &str,
    owner: &str,
    proposer: Option<String>,
    terms: NewMandate,
) -> Result<Mandate> {
    if owner.trim().is_empty() {
        bail!("owner attribution is required to grant a standing permission");
    }
    let terms = canonicalize(terms)?;
    let now = Utc::now();
    if terms.kind.single_active() {
        let grants = sqlx::query(
            "SELECT id, body FROM restless_authority.records \
             WHERE company=$1 AND kind='mandate_granted' AND body->>'kind'=$2 ORDER BY id DESC",
        )
        .bind(company)
        .bind(terms.kind.as_str())
        .fetch_all(&mut **tx)
        .await?;
        for row in grants {
            let existing: Mandate = serde_json::from_value(row.try_get("body")?)?;
            if existing.expires_at > now
                && !is_revoked(tx, company, existing.id, Some(row.try_get("id")?)).await?
            {
                bail!(
                    "the company already has an active permission to {}; approve a change to it or revoke it first",
                    terms.kind.purpose()
                );
            }
        }
    }
    let mandate = Mandate::granted(Uuid::new_v4(), terms, proposer, now);
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'mandate_granted',$2,$3)",
    )
    .bind(company)
    .bind(owner)
    .bind(serde_json::to_value(&mandate)?)
    .execute(&mut **tx)
    .await?;
    Ok(mandate)
}

/// The owner's answer to one proposal. Answering again the same way returns the same result;
/// answering it the other way is refused.
pub async fn decide(
    pool: &PgPool,
    company: &str,
    owner: &str,
    proposal_id: Uuid,
    approve: bool,
    owner_note: Option<&str>,
) -> Result<Option<Mandate>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(format!("mandate-proposal:{company}:{proposal_id}"))
        .execute(&mut *tx)
        .await?;
    let record: Option<crate::authority::AuthorityRecord> = sqlx::query_as(
        "SELECT id,actor_id,body,created_at FROM restless_authority.records \
         WHERE company=$1 AND kind='mandate_proposal' AND body->>'id'=$2 ORDER BY id DESC LIMIT 1",
    )
    .bind(company)
    .bind(proposal_id.to_string())
    .fetch_optional(&mut *tx)
    .await?;
    let proposal = proposal_from(record.context("standing permission proposal not found")?)?;
    let decision_word = if approve { "approve" } else { "decline" };
    if let Some(row) = sqlx::query(
        "SELECT body FROM restless_authority.records \
         WHERE company=$1 AND kind='mandate_proposal_decision' AND body->>'proposal_id'=$2 \
         ORDER BY id LIMIT 1",
    )
    .bind(company)
    .bind(proposal_id.to_string())
    .fetch_optional(&mut *tx)
    .await?
    {
        let old: Value = row.try_get("body")?;
        tx.commit().await?;
        if old["decision"] != decision_word {
            bail!("this standing permission proposal was already answered");
        }
        let Some(mandate_id) = old["mandate_id"]
            .as_str()
            .and_then(|id| Uuid::parse_str(id).ok())
        else {
            return Ok(None);
        };
        let granted: Option<Value> = sqlx::query_scalar(
            "SELECT body FROM restless_authority.records \
             WHERE company=$1 AND kind='mandate_granted' AND body->>'id'=$2 ORDER BY id LIMIT 1",
        )
        .bind(company)
        .bind(mandate_id.to_string())
        .fetch_optional(pool)
        .await?;
        return Ok(granted.and_then(|body| serde_json::from_value(body).ok()));
    }
    let mandate = if approve {
        lock_company(&mut tx, company).await?;
        // An approved change retires the permission it changes in the same transaction, so there
        // is never a moment with both or neither.
        if let Some(replaces) = proposal.replaces {
            revoke_in_transaction(
                &mut tx,
                company,
                owner,
                replaces,
                "Replaced by an approved change.",
            )
            .await?;
        }
        Some(
            grant_in_transaction(
                &mut tx,
                company,
                owner,
                proposal.proposed_by.clone(),
                proposal.terms.clone(),
            )
            .await?,
        )
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'mandate_proposal_decision',$2,$3)",
    )
    .bind(company)
    .bind(owner)
    .bind(serde_json::json!({
        "proposal_id": proposal_id,
        "decision": decision_word,
        "kind": proposal.terms.kind,
        "title": proposal.terms.title,
        "mandate_id": mandate.as_ref().map(|mandate| mandate.id),
        "owner_note": owner_note.map(str::trim).filter(|note| !note.is_empty()),
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(mandate)
}

pub async fn revoke(
    pool: &PgPool,
    company: &str,
    owner: &str,
    mandate_id: Uuid,
    reason: &str,
) -> Result<()> {
    if owner.trim().is_empty() || reason.trim().is_empty() {
        bail!("owner attribution and a reason are required to revoke a standing permission");
    }
    if reason.len() > 2_000 {
        bail!("the revocation reason is longer than 2,000 characters");
    }
    let mut tx = pool.begin().await?;
    lock_company(&mut tx, company).await?;
    revoke_in_transaction(&mut tx, company, owner, mandate_id, reason).await?;
    tx.commit().await?;
    Ok(())
}

/// Revoke the current version of one permission, inside a transaction holding the company lock.
pub async fn revoke_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    company: &str,
    owner: &str,
    mandate_id: Uuid,
    reason: &str,
) -> Result<()> {
    let (mandate, version) = current(tx, company, mandate_id).await?;
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'mandate_revoked',$2,$3)",
    )
    .bind(company)
    .bind(owner)
    .bind(serde_json::json!({
        "mandate_id": mandate_id,
        "mandate_version": version,
        "kind": mandate.kind,
        "reason": reason.trim(),
        "revoked_at": Utc::now(),
    }))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn usage_day(limits: &MandateLimits, at: DateTime<Utc>) -> String {
    let zone = limits
        .timezone
        .as_deref()
        .and_then(|zone| Tz::from_str(zone).ok())
        .unwrap_or(Tz::UTC);
    at.with_timezone(&zone).date_naive().to_string()
}

async fn recorded_usage<'e, E>(
    executor: E,
    company: &str,
    mandate: &Mandate,
) -> Result<MandateUsage>
where
    E: sqlx::PgExecutor<'e>,
{
    let today = usage_day(&mandate.limits, Utc::now());
    let rows = sqlx::query(
        "SELECT body FROM restless_authority.records \
         WHERE company=$1 AND kind='mandate_used' AND body->>'mandate_id'=$2",
    )
    .bind(company)
    .bind(mandate.id.to_string())
    .fetch_all(executor)
    .await?;
    let mut usage = MandateUsage {
        usage_day: today.clone(),
        used_today: 0,
        used_total: 0,
        spent_total: None,
    };
    for row in rows {
        let body: Value = row.try_get("body")?;
        usage.used_total += 1;
        if body["usage_day"].as_str() == Some(today.as_str()) {
            usage.used_today += 1;
        }
        if let Ok(amount) = serde_json::from_value::<Money>(body["amount"].clone()) {
            let spent = usage.spent_total.get_or_insert(Money {
                minor: 0,
                currency: amount.currency.clone(),
            });
            if spent.currency == amount.currency {
                spent.minor = spent.minor.saturating_add(amount.minor);
            }
        }
    }
    Ok(usage)
}

/// What one permission has been used for so far. Email counts its send reservations, as the
/// send path always has; other kinds count the uses recorded with [`record_use`].
pub async fn usage(pool: &PgPool, company: &str, mandate: &Mandate) -> Result<MandateUsage> {
    if mandate.kind == MandateKind::Email {
        let email = crate::authority::email_mandate_usage_in(pool, company, mandate.id).await?;
        return Ok(MandateUsage {
            usage_day: email.usage_day,
            used_today: email.used_today,
            used_total: email.used_total,
            spent_total: None,
        });
    }
    recorded_usage(pool, company, mandate).await
}

/// Why `mandate` does not cover `action`, or `None` when it does.
fn refusal(mandate: &Mandate, usage: &MandateUsage, action: &MandateAction) -> Option<String> {
    let (one, many) = mandate.kind.unit();
    let title = &mandate.title;
    let limits = &mandate.limits;
    if !limits.counterparties.is_empty() {
        match action
            .counterparty
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            None => {
                return Some(format!(
                    "“{title}” is only for {}; say who this {one} is with",
                    limits.counterparties.join(", ")
                ))
            }
            Some(actual)
                if !limits
                    .counterparties
                    .iter()
                    .any(|allowed| counterparty_matches(allowed, actual)) =>
            {
                return Some(format!(
                    "“{title}” is only for {}, not {}",
                    limits.counterparties.join(", "),
                    actual.trim()
                ))
            }
            Some(_) => {}
        }
    }
    if let Some(cap) = &limits.max_per_action {
        let Some(amount) = &action.amount else {
            return Some(format!(
                "“{title}” allows up to {} per {one}; say what this {one} costs",
                cap.plain()
            ));
        };
        if amount.currency != cap.currency {
            return Some(format!(
                "“{title}” is in {}, and this {one} is in {}",
                cap.currency, amount.currency
            ));
        }
        if amount.minor > cap.minor {
            return Some(format!(
                "this {one} is {}, over the {} per {one} limit of “{title}”",
                amount.plain(),
                cap.plain()
            ));
        }
    }
    if let Some(cap) = &limits.max_spend_total {
        let spent = usage.spent_total.as_ref().map_or(0, |money| money.minor);
        let amount = action.amount.as_ref().map_or(0, |money| money.minor);
        if spent.saturating_add(amount) > cap.minor {
            let left = Money {
                minor: (cap.minor - spent).max(0),
                currency: cap.currency.clone(),
            };
            return Some(format!(
                "“{title}” has {} of its {} left",
                if left.minor == 0 {
                    "nothing".into()
                } else {
                    left.plain()
                },
                cap.plain()
            ));
        }
    }
    if let Some(total) = limits.max_total {
        if usage.used_total >= total {
            return Some(format!(
                "“{title}” has used all {total} {}",
                if total == 1 { one } else { many }
            ));
        }
    }
    if let Some(daily) = limits.max_per_day {
        if usage.used_today >= daily {
            return Some(format!(
                "“{title}” has used today's {daily} {}; it allows more tomorrow",
                if daily == 1 { one } else { many }
            ));
        }
    }
    None
}

fn remaining(mandate: &Mandate, usage: &MandateUsage, action: &MandateAction) -> String {
    let (_, many) = mandate.kind.unit();
    let mut parts = Vec::new();
    if let Some(total) = mandate.limits.max_total {
        parts.push(format!(
            "{} {many} left in total",
            total.saturating_sub(usage.used_total + 1)
        ));
    }
    if let Some(daily) = mandate.limits.max_per_day {
        parts.push(format!(
            "{} left today",
            daily.saturating_sub(usage.used_today + 1)
        ));
    }
    if let Some(cap) = &mandate.limits.max_spend_total {
        let spent = usage.spent_total.as_ref().map_or(0, |money| money.minor)
            + action.amount.as_ref().map_or(0, |money| money.minor);
        parts.push(format!(
            "{} of {} left to spend",
            Money {
                minor: (cap.minor - spent).max(0),
                currency: cap.currency.clone()
            }
            .plain(),
            cap.plain()
        ));
    }
    parts.push(format!("until {}", mandate.expires_at.format("%-d %b %Y")));
    parts.join(", ")
}

/// A ready proposal for an action no permission covers, to adjust and raise.
pub fn draft_for(kind: MandateKind, action: &MandateAction) -> NewMandate {
    let summary = action.summary.trim();
    let counterparty = action
        .counterparty
        .as_deref()
        .map(normalize_counterparty)
        .filter(|value| !value.is_empty());
    let (title, scope) = match (kind, counterparty.as_deref()) {
        (MandateKind::VendorOrder, Some(vendor)) => (
            format!("Order from {vendor}"),
            if summary.is_empty() {
                format!("Orders from {vendor}")
            } else {
                summary.to_string()
            },
        ),
        (MandateKind::VendorOrder, None) => (
            "Order from a vendor".to_string(),
            if summary.is_empty() {
                "Orders from vendors".into()
            } else {
                summary.to_string()
            },
        ),
        (MandateKind::Email, _) => (
            "Send email".to_string(),
            if summary.is_empty() {
                "Email people who fit the purpose".into()
            } else {
                summary.to_string()
            },
        ),
    };
    let limits = match kind {
        MandateKind::VendorOrder => MandateLimits {
            max_per_action: action.amount.clone(),
            counterparties: counterparty.into_iter().collect(),
            ..MandateLimits::default()
        },
        MandateKind::Email => MandateLimits {
            max_per_day: Some(5),
            max_total: Some(50),
            timezone: Some("UTC".into()),
            ..MandateLimits::default()
        },
    };
    NewMandate {
        kind,
        title,
        scope,
        limits,
        expires_at: Utc::now() + chrono::Duration::days(30),
        owner_approves_each: false,
        rationale: String::new(),
        evidence: Vec::new(),
        email: (kind == MandateKind::Email).then(|| EmailTerms {
            purpose: if summary.is_empty() {
                "What these emails achieve".into()
            } else {
                summary.to_string()
            },
            audience_guidance: "Who should receive them".into(),
            sender: "you@your-domain".into(),
            sender_name: None,
        }),
    }
}

fn propose_how(kind: MandateKind) -> String {
    format!(
        "Raise it as one card in the owner's Inbox: write `proposal` (adjusted: a plain title, the real scope and limits, your rationale and evidence) to a file and run `restless mandate propose --kind {kind} --proposal-file <file> --reason <why>`, or call the `restless_propose_mandate` tool. Do not ask for it in chat. You are told when the owner answers; continue other work meanwhile."
    )
}

async fn check_in<'c>(
    tx: &mut Transaction<'c, Postgres>,
    pool: &PgPool,
    company: &str,
    kind: MandateKind,
    action: &MandateAction,
) -> Result<(MandateCheck, Option<(Mandate, i64, MandateUsage)>)> {
    let now = Utc::now();
    let rows = sqlx::query(
        "SELECT g.id, g.body FROM restless_authority.records g \
         WHERE g.company=$1 AND g.kind='mandate_granted' AND g.body->>'kind'=$2 \
           AND NOT EXISTS (SELECT 1 FROM restless_authority.records r \
             WHERE r.company=g.company AND r.kind='mandate_revoked' AND r.body->>'mandate_id'=g.body->>'id') \
         ORDER BY g.id DESC",
    )
    .bind(company)
    .bind(kind.as_str())
    .fetch_all(&mut **tx)
    .await?;
    let mut seen = std::collections::HashSet::new();
    let mut reasons = Vec::new();
    let mut expired = None;
    for row in rows {
        let mandate: Mandate = serde_json::from_value(row.try_get("body")?)?;
        if !seen.insert(mandate.id) {
            continue;
        }
        let version: i64 = row.try_get("id")?;
        if mandate.expires_at <= now {
            expired.get_or_insert_with(|| {
                format!(
                    "“{}” expired on {}",
                    mandate.title,
                    mandate.expires_at.format("%-d %b %Y")
                )
            });
            continue;
        }
        let usage = if kind == MandateKind::Email {
            usage(pool, company, &mandate).await?
        } else {
            recorded_usage(&mut **tx, company, &mandate).await?
        };
        match refusal(&mandate, &usage, action) {
            None => {
                let check = MandateCheck::Allowed {
                    kind,
                    mandate_id: mandate.id,
                    mandate_version: version,
                    title: mandate.title.clone(),
                    owner_approves_each: mandate.owner_approves_each,
                    remaining: remaining(&mandate, &usage, action),
                    how: if mandate.owner_approves_each {
                        format!("This permission still needs the owner's approval for each {}: put this exact one to them in their Inbox before acting.", kind.unit().0)
                    } else {
                        kind.how_to_act().to_string()
                    },
                };
                return Ok((check, Some((mandate, version, usage))));
            }
            Some(reason) => reasons.push((mandate.id, reason)),
        }
    }
    let reason = match (reasons.first(), expired) {
        (Some((_, reason)), _) => format!("No standing permission covers this: {reason}."),
        (None, Some(expired)) => format!("No standing permission covers this: {expired}."),
        (None, None) => format!("There is no standing permission to {}.", kind.purpose()),
    };
    let waiting = pending(pool, company)
        .await?
        .into_iter()
        .rev()
        .find(|proposal| proposal.terms.kind == kind);
    // A single-active kind is changed, not added to: the draft replaces the current one.
    let replaces = kind
        .single_active()
        .then(|| reasons.first().map(|(id, _)| *id))
        .flatten();
    let check = match waiting {
        Some(waiting) => MandateCheck::NeedsPermission {
            kind,
            reason,
            pending_proposal: Some(waiting.id),
            proposal: None,
            replaces: None,
            how: format!("A proposal (“{}”) is already waiting in the owner's Inbox. Do not propose again; you are told when they answer. Continue other work meanwhile.", waiting.terms.title),
        },
        None => MandateCheck::NeedsPermission {
            kind,
            reason,
            pending_proposal: None,
            proposal: Some(draft_for(kind, action)),
            replaces,
            how: propose_how(kind),
        },
    };
    Ok((check, None))
}

/// The one check an action calls before it acts: allowed under a standing permission, or the
/// standing permission it needs, with a ready proposal for it. Records nothing.
pub async fn require(
    pool: &PgPool,
    company: &str,
    kind: MandateKind,
    action: &MandateAction,
) -> Result<MandateCheck> {
    let mut tx = pool.begin().await?;
    let (check, _) = check_in(&mut tx, pool, company, kind, action).await?;
    tx.commit().await?;
    Ok(check)
}

/// Check and, when allowed, record the action against the permission's limits in one step, so
/// two orders cannot both take the last of a limit. Recording the same `key` again returns the
/// first answer. Email records its uses on the send path instead.
pub async fn record_use(
    pool: &PgPool,
    company: &str,
    actor: &str,
    kind: MandateKind,
    action: &MandateAction,
) -> Result<MandateCheck> {
    if kind == MandateKind::Email {
        bail!("an email is recorded when it is sent: permit it with `restless mandate permit`, then `restless email send`");
    }
    if action.summary.trim().is_empty() || action.summary.len() > 1_000 {
        bail!("say what this {} is in a short summary", kind.unit().0);
    }
    let key = action
        .key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string);
    if key.as_deref().is_some_and(|key| key.len() > 200) {
        bail!("the action key is longer than 200 characters");
    }
    let mut tx = pool.begin().await?;
    lock_company(&mut tx, company).await?;
    if let Some(key) = key.as_deref() {
        if let Some(body) = sqlx::query_scalar::<_, Value>(
            "SELECT body FROM restless_authority.records \
             WHERE company=$1 AND kind='mandate_used' AND body->>'action_key'=$2 LIMIT 1",
        )
        .bind(company)
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok(MandateCheck::Allowed {
                kind,
                mandate_id: serde_json::from_value(body["mandate_id"].clone())?,
                mandate_version: body["mandate_version"].as_i64().unwrap_or_default(),
                title: body["title"].as_str().unwrap_or_default().to_string(),
                owner_approves_each: body["owner_approves_each"].as_bool().unwrap_or(false),
                remaining: "This action was already recorded; it counts once.".into(),
                how: kind.how_to_act().to_string(),
            });
        }
    }
    let (check, matched) = check_in(&mut tx, pool, company, kind, action).await?;
    if let Some((mandate, version, _)) = matched {
        sqlx::query(
            "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
             VALUES ($1,'mandate_used',$2,$3)",
        )
        .bind(company)
        .bind(actor)
        .bind(serde_json::json!({
            "mandate_id": mandate.id,
            "mandate_version": version,
            "kind": kind,
            "title": mandate.title,
            "owner_approves_each": mandate.owner_approves_each,
            "summary": action.summary.trim(),
            "counterparty": action.counterparty.as_deref().map(normalize_counterparty),
            "amount": action.amount,
            "action_key": key,
            "usage_day": usage_day(&mandate.limits, Utc::now()),
            "used_at": Utc::now(),
        }))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(check)
}

// ---------------------------------------------------------------------------------------------
// Moving email-only records into the generic ones.

/// Rewrite the email-only records that came before generic mandates (`email_mandate_granted`,
/// `email_mandate_proposal`, `email_mandate_proposal_decision`, `email_mandate_revoked`) into
/// the generic kinds, in place. Each record keeps its id, actor and time: a grant's id is its
/// version, so permits issued under it still match, and announcements keyed on record ids are
/// not repeated. Each rewritten body names what it was migrated from. Idempotent; a record that
/// cannot be read is left untouched and logged, never dropped.
pub async fn migrate_email_records(pool: &PgPool) -> Result<usize> {
    let rows = sqlx::query(
        "SELECT id, company, kind, body FROM restless_authority.records \
         WHERE kind IN ('email_mandate_granted','email_mandate_proposal',\
                        'email_mandate_proposal_decision','email_mandate_revoked') ORDER BY id",
    )
    .fetch_all(pool)
    .await
    .context("read email mandate records to migrate")?;
    let mut migrated = 0;
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let company: String = row.try_get("company")?;
        let kind: String = row.try_get("kind")?;
        let body: Value = row.try_get("body")?;
        let rewritten = match migrate_body(&kind, body) {
            Ok(rewritten) => rewritten,
            Err(error) => {
                tracing::warn!(
                    company,
                    record = id,
                    kind,
                    "email mandate record left as it was: {error:#}"
                );
                continue;
            }
        };
        let updated = sqlx::query(
            "UPDATE restless_authority.records SET kind=$2, body=$3 WHERE id=$1 AND kind=$4",
        )
        .bind(id)
        .bind(rewritten.0)
        .bind(rewritten.1)
        .bind(&kind)
        .execute(pool)
        .await
        .with_context(|| format!("migrate email mandate record {id}"))?;
        migrated += updated.rows_affected() as usize;
    }
    Ok(migrated)
}

/// The generic kind and body for one email-only record.
pub fn migrate_body(kind: &str, mut body: Value) -> Result<(&'static str, Value)> {
    match kind {
        "email_mandate_granted" => {
            let email: EmailMandate =
                serde_json::from_value(body).context("decode email mandate grant")?;
            let mut value = serde_json::to_value(Mandate::from_email(email))?;
            value["migrated_from"] = "email_mandate_granted".into();
            Ok((GRANTED_KIND, value))
        }
        "email_mandate_proposal" => {
            let proposal: NewEmailMandate = serde_json::from_value(body["proposal"].clone())
                .context("decode email mandate proposal")?;
            let note = body["judgement_note"].as_str().map(str::to_owned);
            let terms = NewMandate::from_email(proposal, note.as_deref());
            let mut value = serde_json::json!({
                "id": body["id"],
                "kind": MandateKind::Email,
                "terms": terms,
                "migrated_from": "email_mandate_proposal",
            });
            if body["id"].as_str().is_none() {
                bail!("email mandate proposal has no id");
            }
            if !body["replaces"].is_null() {
                value["replaces"] = body["replaces"].clone();
            }
            if let Some(note) = note {
                value["judgement_note"] = note.into();
            }
            Ok((PROPOSAL_KIND, value))
        }
        "email_mandate_proposal_decision" => {
            let object = body.as_object_mut().context("decision is not an object")?;
            object.insert("kind".into(), "email".into());
            object.insert(
                "migrated_from".into(),
                "email_mandate_proposal_decision".into(),
            );
            Ok((DECISION_KIND, body))
        }
        "email_mandate_revoked" => {
            let object = body
                .as_object_mut()
                .context("revocation is not an object")?;
            object.insert("kind".into(), "email".into());
            object.insert("migrated_from".into(), "email_mandate_revoked".into());
            Ok((REVOKED_KIND, body))
        }
        other => bail!("{other} is not an email mandate record"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(amount: &str) -> MandateAction {
        MandateAction {
            summary: "200 fridge magnets".into(),
            counterparty: Some("https://www.vistaprint.com.au/magnets".into()),
            amount: Some(Money::new(parse_minor(amount, "AUD").unwrap(), "AUD").unwrap()),
            key: None,
        }
    }

    fn magnets() -> NewMandate {
        NewMandate {
            kind: MandateKind::VendorOrder,
            title: "Print fridge magnets with Vistaprint".into(),
            scope: "Reorder the open-day fridge magnets when stock runs low".into(),
            limits: MandateLimits {
                max_per_action: Some(Money::new(15_000, "AUD").unwrap()),
                max_spend_total: Some(Money::new(40_000, "AUD").unwrap()),
                max_total: Some(3),
                counterparties: vec!["https://www.Vistaprint.com.au/".into()],
                ..MandateLimits::default()
            },
            expires_at: Utc::now() + chrono::Duration::days(30),
            owner_approves_each: false,
            rationale: "We hand them out at every open day.".into(),
            evidence: vec!["Library/marketing/open-day.md".into()],
            email: None,
        }
    }

    #[test]
    fn money_reads_and_writes_the_way_people_do() {
        assert_eq!(parse_minor("150", "AUD").unwrap(), 15_000);
        assert_eq!(parse_minor("A$1,150.5", "AUD").unwrap(), 115_050);
        assert_eq!(parse_minor("1200", "JPY").unwrap(), 1_200);
        assert!(parse_minor("1.234", "AUD").is_err());
        assert!(parse_minor("ten", "AUD").is_err());
        assert_eq!(Money::new(15_000, "AUD").unwrap().plain(), "A$150");
        assert_eq!(Money::new(115_050, "aud").unwrap().plain(), "A$1,150.50");
        assert_eq!(Money::new(500, "CHF").unwrap().plain(), "CHF 5");
        let parsed: Money =
            serde_json::from_value(serde_json::json!({"amount": "12.50", "currency": "A$"}))
                .unwrap();
        assert_eq!(parsed, Money::new(1_250, "AUD").unwrap());
        assert!(serde_json::from_value::<Money>(
            serde_json::json!({"amount": "-1", "currency": "AUD"})
        )
        .is_err());
    }

    #[test]
    fn counterparties_match_addresses_vendors_and_domains() {
        assert!(counterparty_matches(
            "vistaprint.com.au",
            "https://www.vistaprint.com.au/x"
        ));
        assert!(counterparty_matches("example.com", "Jo@Example.com"));
        assert!(counterparty_matches("example.com", "mail.example.com"));
        assert!(counterparty_matches("jo@example.com", "JO@example.com"));
        assert!(!counterparty_matches("example.com", "example.com.evil.io"));
        assert!(!counterparty_matches("example.com", "notexample.com"));
        assert!(!counterparty_matches("jo@example.com", "amy@example.com"));
    }

    #[test]
    fn a_vendor_order_needs_a_money_cap_and_reads_plainly() {
        let terms = canonicalize(magnets()).unwrap();
        assert_eq!(terms.limits.counterparties, vec!["vistaprint.com.au"]);
        let rows = plain_terms(&terms);
        let limits = &rows.iter().find(|(label, _)| label == "Limits").unwrap().1;
        assert_eq!(
            limits,
            "Up to A$150 per order; Up to A$400 in total; At most 3 orders in total; Only with vistaprint.com.au"
        );
        let mut uncapped = magnets();
        uncapped.limits.max_per_action = None;
        assert!(canonicalize(uncapped).is_err());
        let mut mixed = magnets();
        mixed.limits.max_spend_total = Some(Money::new(40_000, "USD").unwrap());
        assert!(canonicalize(mixed).is_err());
        let mut late = magnets();
        late.expires_at = Utc::now() + chrono::Duration::days(400);
        assert!(canonicalize(late).is_err());
    }

    #[test]
    fn email_terms_round_trip_through_the_generic_shape() {
        let legacy = NewEmailMandate {
            purpose: "Invite past clients to the open day".into(),
            audience_guidance: "People who bought in the last two years".into(),
            sender: "Hello@Example.com".into(),
            sender_name: Some("Reno".into()),
            max_per_day: 5,
            max_total: 100,
            timezone: "Australia/Sydney".into(),
            expires_at: Utc::now() + chrono::Duration::days(30),
        };
        let terms = canonicalize(NewMandate::from_email(legacy.clone(), Some("Open day"))).unwrap();
        assert_eq!(terms.title, "Send email as Reno <Hello@Example.com>");
        assert_eq!(terms.email.as_ref().unwrap().sender, "hello@example.com");
        assert_eq!(terms.rationale, "Open day");
        let view = terms.email_view().unwrap();
        assert_eq!(view.max_per_day, 5);
        assert_eq!(view.timezone, "Australia/Sydney");
        let mut asks_each = terms.clone();
        asks_each.owner_approves_each = true;
        assert!(canonicalize(asks_each).is_err());
        let mut no_email = terms;
        no_email.email = None;
        assert!(canonicalize(no_email).is_err());
    }

    #[test]
    fn email_records_migrate_into_the_generic_kinds() {
        let id = Uuid::new_v4();
        let expires = Utc::now() + chrono::Duration::days(3);
        let grant = serde_json::json!({
            "id": id, "purpose": "Open day", "audience_guidance": "Past clients",
            "sender": "hello@example.com", "sender_name": null, "max_per_day": 5, "max_total": 50,
            "timezone": "UTC", "expires_at": expires, "created_at": Utc::now(),
        });
        let (kind, body) = migrate_body("email_mandate_granted", grant).unwrap();
        assert_eq!(kind, GRANTED_KIND);
        let mandate: Mandate = serde_json::from_value(body.clone()).unwrap();
        assert_eq!(mandate.id, id);
        assert_eq!(body["migrated_from"], "email_mandate_granted");
        let email = mandate.as_email().unwrap();
        assert_eq!((email.max_per_day, email.max_total), (5, 50));

        let proposal = serde_json::json!({"id": Uuid::new_v4(), "judgement_note": "Because", "replaces": id, "proposal": {
            "purpose": "Open day", "audience_guidance": "Past clients", "sender": "hello@example.com",
            "max_per_day": 6, "max_total": 50, "timezone": "UTC", "expires_at": expires}});
        let (kind, body) = migrate_body("email_mandate_proposal", proposal).unwrap();
        assert_eq!(kind, PROPOSAL_KIND);
        let terms: NewMandate = serde_json::from_value(body["terms"].clone()).unwrap();
        assert_eq!(terms.limits.max_per_day, Some(6));
        assert_eq!(terms.rationale, "Because");
        assert_eq!(body["replaces"], serde_json::json!(id));

        let (kind, body) = migrate_body(
            "email_mandate_proposal_decision",
            serde_json::json!({"proposal_id": "p", "decision": "approve", "mandate_id": id}),
        )
        .unwrap();
        assert_eq!(
            (kind, body["kind"].as_str()),
            (DECISION_KIND, Some("email"))
        );
        let (kind, _) = migrate_body(
            "email_mandate_revoked",
            serde_json::json!({"mandate_id": id, "mandate_version": 7}),
        )
        .unwrap();
        assert_eq!(kind, REVOKED_KIND);
        assert!(migrate_body("email_mandate_granted", serde_json::json!({"id": 1})).is_err());
    }

    fn database() -> Option<String> {
        std::env::var("RESTLESS_TEST_DATABASE_URL").ok()
    }

    /// Propose, approve after the owner decides, use within limits, refuse past them, revoke.
    #[tokio::test]
    async fn a_vendor_order_permission_is_proposed_granted_used_and_revoked() {
        let Some(url) = database() else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping vendor order mandate scenario");
            return;
        };
        let authority = crate::authority::AuthorityStore::connect(&url)
            .await
            .unwrap();
        let pool = authority.pool();
        let company = format!("mandate_vendor_{}", Uuid::new_v4().simple());
        // Nothing yet: the check says what is needed and offers a ready proposal.
        let check = require(pool, &company, MandateKind::VendorOrder, &order("120"))
            .await
            .unwrap();
        let MandateCheck::NeedsPermission {
            proposal: Some(draft),
            pending_proposal: None,
            ..
        } = check
        else {
            panic!("expected a ready proposal, got {check:?}")
        };
        assert_eq!(draft.limits.counterparties, vec!["vistaprint.com.au"]);
        assert_eq!(
            draft.limits.max_per_action,
            Some(Money::new(12_000, "AUD").unwrap())
        );

        let proposal = propose(pool, &company, "exec", magnets(), None)
            .await
            .unwrap();
        // Waiting: the check names the proposal and asks for no second one.
        let MandateCheck::NeedsPermission {
            pending_proposal,
            proposal: None,
            ..
        } = require(pool, &company, MandateKind::VendorOrder, &order("120"))
            .await
            .unwrap()
        else {
            panic!("expected the waiting proposal")
        };
        assert_eq!(pending_proposal, Some(proposal));
        assert_eq!(pending(pool, &company).await.unwrap().len(), 1);

        // A decline is final for that proposal; the same answer again is idempotent.
        let declined = propose(pool, &company, "exec", magnets(), None)
            .await
            .unwrap();
        assert!(
            decide(pool, &company, "owner", declined, false, Some("Not yet"))
                .await
                .unwrap()
                .is_none()
        );
        assert!(decide(pool, &company, "owner", declined, false, None)
            .await
            .unwrap()
            .is_none());
        assert!(decide(pool, &company, "owner", declined, true, None)
            .await
            .is_err());

        let granted = decide(pool, &company, "owner", proposal, true, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            decide(pool, &company, "owner", proposal, true, None)
                .await
                .unwrap()
                .unwrap()
                .id,
            granted.id
        );
        assert_eq!(granted.proposed_by.as_deref(), Some("exec"));
        assert!(pending(pool, &company).await.unwrap().is_empty());

        // Within limits: allowed, and only the vendor it names.
        assert!(
            require(pool, &company, MandateKind::VendorOrder, &order("120"))
                .await
                .unwrap()
                .allowed()
        );
        let mut elsewhere = order("20");
        elsewhere.counterparty = Some("moo.com".into());
        assert!(
            !require(pool, &company, MandateKind::VendorOrder, &elsewhere)
                .await
                .unwrap()
                .allowed()
        );
        let over = require(pool, &company, MandateKind::VendorOrder, &order("150.01"))
            .await
            .unwrap();
        let MandateCheck::NeedsPermission { reason, .. } = &over else {
            panic!()
        };
        assert!(
            reason.contains("over the A$150 per order limit"),
            "{reason}"
        );
        // Email is another kind: this permission does not cover it.
        assert!(!require(
            pool,
            &company,
            MandateKind::Email,
            &MandateAction::default()
        )
        .await
        .unwrap()
        .allowed());

        // Uses count against the total: A$400 holds two A$150 orders and one of A$100.
        let mut first = order("150");
        first.key = Some("po-1".into());
        assert!(
            record_use(pool, &company, "exec", MandateKind::VendorOrder, &first)
                .await
                .unwrap()
                .allowed()
        );
        assert!(
            record_use(pool, &company, "exec", MandateKind::VendorOrder, &first)
                .await
                .unwrap()
                .allowed()
        );
        assert!(record_use(
            pool,
            &company,
            "exec",
            MandateKind::VendorOrder,
            &order("150")
        )
        .await
        .unwrap()
        .allowed());
        let MandateCheck::NeedsPermission { reason, .. } = record_use(
            pool,
            &company,
            "exec",
            MandateKind::VendorOrder,
            &order("120"),
        )
        .await
        .unwrap() else {
            panic!("the total money limit holds")
        };
        assert!(reason.contains("A$100 of its A$400 left"), "{reason}");
        assert!(record_use(
            pool,
            &company,
            "exec",
            MandateKind::VendorOrder,
            &order("100")
        )
        .await
        .unwrap()
        .allowed());
        let used = usage(pool, &company, &granted).await.unwrap();
        assert_eq!(used.used_total, 3, "the repeated key counted once");
        assert_eq!(used.spent_total, Some(Money::new(40_000, "AUD").unwrap()));
        let MandateCheck::NeedsPermission { reason, .. } =
            require(pool, &company, MandateKind::VendorOrder, &order("1"))
                .await
                .unwrap()
        else {
            panic!()
        };
        assert!(
            reason.contains("nothing of its A$400 left") || reason.contains("used all 3"),
            "{reason}"
        );

        // Revoked: nothing is allowed, and it leaves the list.
        revoke(pool, &company, "owner", granted.id, "Stock is fine")
            .await
            .unwrap();
        assert!(list(pool, &company).await.unwrap().is_empty());
        assert!(revoke(pool, &company, "owner", granted.id, "again")
            .await
            .is_err());
    }

    /// Expired permissions allow nothing and say so; a daily limit resets by the day.
    #[tokio::test]
    async fn an_expired_permission_allows_nothing() {
        let Some(url) = database() else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mandate expiry scenario");
            return;
        };
        let authority = crate::authority::AuthorityStore::connect(&url)
            .await
            .unwrap();
        let pool = authority.pool();
        let company = format!("mandate_expiry_{}", Uuid::new_v4().simple());
        let mut daily = magnets();
        daily.limits.max_per_day = Some(1);
        daily.limits.max_total = Some(3);
        let proposal = propose(pool, &company, "exec", daily, None).await.unwrap();
        let granted = decide(pool, &company, "owner", proposal, true, None)
            .await
            .unwrap()
            .unwrap();
        assert!(record_use(
            pool,
            &company,
            "exec",
            MandateKind::VendorOrder,
            &order("10")
        )
        .await
        .unwrap()
        .allowed());
        let MandateCheck::NeedsPermission { reason, .. } =
            require(pool, &company, MandateKind::VendorOrder, &order("10"))
                .await
                .unwrap()
        else {
            panic!("one a day")
        };
        assert!(reason.contains("today's 1 order"), "{reason}");
        // Move the grant's expiry into the past, as time would.
        sqlx::query(
            "UPDATE restless_authority.records SET body = jsonb_set(body, '{expires_at}', to_jsonb($2::text)) \
             WHERE company=$1 AND kind='mandate_granted'",
        )
        .bind(&company)
        .bind((Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
        .execute(pool)
        .await
        .unwrap();
        let MandateCheck::NeedsPermission {
            reason, proposal, ..
        } = require(pool, &company, MandateKind::VendorOrder, &order("10"))
            .await
            .unwrap()
        else {
            panic!("expired")
        };
        assert!(reason.contains("expired on"), "{reason}");
        assert!(
            proposal.is_some(),
            "an expired permission is asked for again"
        );
        assert!(active(pool, &company).await.unwrap().is_empty());
        assert_eq!(list(pool, &company).await.unwrap()[0].id, granted.id);
    }

    /// Email records written before mandates were generic keep working after the migration.
    #[tokio::test]
    async fn email_records_written_before_the_migration_keep_working() {
        let Some(url) = database() else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping email mandate migration scenario"
            );
            return;
        };
        let authority = crate::authority::AuthorityStore::connect(&url)
            .await
            .unwrap();
        let pool = authority.pool();
        let company = format!("mandate_migrate_{}", Uuid::new_v4().simple());
        let expires = Utc::now() + chrono::Duration::days(10);
        let insert = |kind: &'static str, actor: &'static str, body: Value| {
            let company = company.clone();
            async move {
                sqlx::query_scalar::<_, i64>(
                    "INSERT INTO restless_authority.records (company,kind,actor_id,body) VALUES ($1,$2,$3,$4) RETURNING id",
                )
                .bind(company)
                .bind(kind)
                .bind(actor)
                .bind(body)
                .fetch_one(pool)
                .await
                .unwrap()
            }
        };
        let legacy = |per_day: u32| {
            serde_json::json!({
            "purpose": "Open day", "audience_guidance": "Past clients", "sender": "hello@example.com",
            "max_per_day": per_day, "max_total": 50, "timezone": "UTC", "expires_at": expires})
        };
        // One proposal approved into a grant, one declined, one still waiting, and an older
        // grant that was revoked.
        let approved = Uuid::new_v4();
        let declined = Uuid::new_v4();
        let waiting = Uuid::new_v4();
        let old = Uuid::new_v4();
        let current = Uuid::new_v4();
        insert("email_mandate_proposal", "exec", serde_json::json!({"id": approved, "proposal": legacy(5), "judgement_note": "Open day"})).await;
        let mut old_grant = legacy(3);
        old_grant["id"] = serde_json::json!(old);
        old_grant["created_at"] = serde_json::json!(Utc::now());
        let old_version = insert("email_mandate_granted", "owner", old_grant).await;
        insert("email_mandate_revoked", "owner", serde_json::json!({"mandate_id": old, "mandate_version": old_version, "reason": "Old", "revoked_at": Utc::now()})).await;
        let mut grant = legacy(5);
        grant["id"] = serde_json::json!(current);
        grant["created_at"] = serde_json::json!(Utc::now());
        let version = insert("email_mandate_granted", "owner", grant).await;
        insert("email_mandate_proposal_decision", "owner", serde_json::json!({"proposal_id": approved, "decision": "approve", "mandate_id": current, "owner_note": null})).await;
        insert(
            "email_mandate_proposal",
            "exec",
            serde_json::json!({"id": declined, "proposal": legacy(9), "judgement_note": null}),
        )
        .await;
        insert("email_mandate_proposal_decision", "owner", serde_json::json!({"proposal_id": declined, "decision": "decline", "mandate_id": null, "owner_note": "No"})).await;
        insert("email_mandate_proposal", "exec", serde_json::json!({"id": waiting, "proposal": legacy(7), "judgement_note": "More", "replaces": current})).await;

        assert!(migrate_email_records(pool).await.unwrap() >= 8);
        assert_eq!(migrate_email_records(pool).await.unwrap(), 0, "idempotent");
        let left: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM restless_authority.records WHERE company=$1 AND kind LIKE 'email_mandate%'",
        )
        .bind(&company)
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(left, 0);
        let all: i64 =
            sqlx::query_scalar("SELECT count(*) FROM restless_authority.records WHERE company=$1")
                .bind(&company)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(all, 8, "nothing lost, nothing added");

        // The current grant keeps its version; the revoked one stays revoked.
        let listed = list_versions(pool, &company).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!((listed[0].0.id, listed[0].1), (current, version));
        assert_eq!(
            authority.list_email_mandates(&company).await.unwrap()[0].max_per_day,
            5
        );
        // The waiting proposal is still the owner's to answer, and approving replaces the grant.
        let open = pending(pool, &company).await.unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!((open[0].id, open[0].replaces), (waiting, Some(current)));
        assert_eq!(open[0].terms.rationale, "More");
        assert!(
            decide(pool, &company, "owner", declined, true, None)
                .await
                .is_err(),
            "the decline stands"
        );
        let replaced = decide(pool, &company, "owner", waiting, true, None)
            .await
            .unwrap()
            .unwrap();
        let listed = authority.list_email_mandates(&company).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!((listed[0].id, listed[0].max_per_day), (replaced.id, 7));
    }
}
