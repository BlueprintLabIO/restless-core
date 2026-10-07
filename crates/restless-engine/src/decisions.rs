//! Structured decisions: a state and typed questions in, probabilities out, from a decision model
//! behind the hosted model relay (`POST {GPT_BASE_URL}/decisions`). Every decision is advisory and
//! recorded with its probabilities. A decision kind starts in shadow: it is logged beside what the
//! company actually did and changes nothing, so it earns its switch-on from the company's own record.
//! Without a relay (a self-hosted appliance), decisions are simply absent and today's behaviour holds.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use restless_orgintel::{NewDecision, OrgIntel};
use serde_json::{Value, json};

/// The relay's public decision id; the relay picks and falls back between upstream models.
const DECISION_MODEL: &str = "decide";
const DECISION_TIMEOUT: Duration = Duration::from_secs(10);
const RELAY_TOKEN_ENV: &str = "RESTLESS_HOSTED_MODEL_RELAY_TOKEN";

/// Decisions run in shadow unless `RESTLESS_DECISIONS=off`. A per-company switch arrives with the
/// first kind whose shadow record earns an active mode.
pub fn enabled() -> bool {
    !matches!(std::env::var("RESTLESS_DECISIONS").as_deref(), Ok("off"))
}

pub struct DecisionClient {
    http: reqwest::Client,
    url: String,
    token: String,
}

pub struct Decision {
    pub model: String,
    pub answers: Value,
    pub latency_ms: i32,
}

impl Decision {
    pub fn noul(&self, question: &str) -> Option<f64> {
        self.answers.get(question)?.get("noul")?.as_f64()
    }
}

impl DecisionClient {
    /// The hosted relay route, when this plane has one.
    pub fn from_env() -> Option<Self> {
        let base = std::env::var("GPT_BASE_URL").ok()?;
        let token = std::env::var(RELAY_TOKEN_ENV)
            .ok()
            .filter(|token| !token.trim().is_empty())?;
        Self::new(&base, token)
    }

    /// A relay at `base` (the OpenAI-compatible base, ending `/v1`) with a plane bearer.
    pub fn new(base: &str, token: String) -> Option<Self> {
        let url = reqwest::Url::parse(base.trim_end_matches('/')).ok()?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return None;
        }
        let http = reqwest::Client::builder()
            .timeout(DECISION_TIMEOUT)
            .build()
            .ok()?;
        Some(Self {
            http,
            url: format!("{}/decisions", url.as_str().trim_end_matches('/')),
            token,
        })
    }

    pub async fn decide(&self, state: Value, questions: Value) -> Result<Decision> {
        let started = Instant::now();
        let response = self
            .http
            .post(&self.url)
            .bearer_auth(&self.token)
            .json(&json!({ "model": DECISION_MODEL, "state": state, "questions": questions }))
            .send()
            .await
            .context("decision relay unreachable")?;
        anyhow::ensure!(
            response.status().is_success(),
            "decision relay answered {}",
            response.status()
        );
        let body: Value = response
            .json()
            .await
            .context("decision relay sent unreadable JSON")?;
        let answers = body
            .get("answers")
            .cloned()
            .context("decision relay sent no answers")?;
        Ok(Decision {
            model: body
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or(DECISION_MODEL)
                .chars()
                .take(160)
                .collect(),
            answers,
            latency_ms: i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX),
        })
    }
}

pub const EXEC_GATE: &str = "exec_gate";
/// The successful-result notice OrgIntel writes for the Exec (attempts.rs).
const EXEC_RESULT_PREFIX: &str = "Completed Work result for Exec delivery consideration:";
/// Skip only when both answers are confidently no. On labelled results this saved 11 of 13 needless
/// wakes and missed none; anything less certain wakes, as today.
const EXEC_GATE_SKIP_BELOW: f64 = 0.3;
const MAX_PER_PASS: usize = 8;

fn exec_gate_questions() -> Value {
    json!({
        "owner_update": {"type": "noul", "instructions": "Should the owner hear about this result now, rather than in a routine summary?",
            "criteria": {"true": "It is something the owner asked for, is waiting on, must act on, or would be surprised not to be told: a deliverable ready for review, money, a customer or lead event, a risk, or a blocker cleared.",
                         "false": "It is routine progress, maintenance or an internal step the owner did not ask about and need not act on."}},
        "next_step": {"type": "noul", "instructions": "Does this result change what the company should do next, beyond work already under way?",
            "criteria": {"true": "It completes something that unblocks or triggers new work, a decision or an approval, or it reveals a fact that changes priorities.",
                         "false": "Nothing new follows: it is a step inside work that continues on its own, or self-contained routine work."}}
    })
}

/// Decide, in shadow, whether each unread successful-result notice would have needed an Exec wake.
/// The wake still happens; its own quiet-or-update finish becomes the outcome (see
/// [`settle_exec_results`]). Failures only log: a decision never stands between work and the Exec.
pub async fn shadow_exec_results(org: &OrgIntel, mission: &str) {
    if !enabled() {
        return;
    }
    if let Some(client) = DecisionClient::from_env() {
        shadow_exec_results_with(&client, org, mission).await;
    }
}

pub async fn shadow_exec_results_with(client: &DecisionClient, org: &OrgIntel, mission: &str) {
    let notices = match org.inbox(Some("exec")).await {
        Ok(messages) => messages
            .into_iter()
            .filter(|message| {
                message.from_actor == "daemon" && message.body.starts_with(EXEC_RESULT_PREFIX)
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            tracing::warn!("decision pass could not read the Exec inbox: {error:#}");
            return;
        }
    };
    if notices.is_empty() {
        return;
    }
    let subjects: Vec<String> = notices
        .iter()
        .map(|message| message.id.to_string())
        .collect();
    let decided = org
        .decided_subjects(EXEC_GATE, &subjects)
        .await
        .unwrap_or_default();
    for notice in notices
        .iter()
        .filter(|message| !decided.contains(&message.id.to_string()))
        .take(MAX_PER_PASS)
    {
        let result: String = notice.body.chars().take(4000).collect();
        let state = json!({ "company_mission": mission, "result": result });
        match client.decide(state, exec_gate_questions()).await {
            Ok(decision) => {
                let (Some(update), Some(next)) =
                    (decision.noul("owner_update"), decision.noul("next_step"))
                else {
                    tracing::warn!(
                        message = notice.id,
                        "exec gate decision came back without both answers"
                    );
                    continue;
                };
                let choice = if update < EXEC_GATE_SKIP_BELOW && next < EXEC_GATE_SKIP_BELOW {
                    "skip_wake"
                } else {
                    "wake"
                };
                let subject = notice.id.to_string();
                if let Err(error) = org
                    .record_decision(NewDecision {
                        kind: EXEC_GATE,
                        subject: &subject,
                        mode: "shadow",
                        model: &decision.model,
                        answers: &decision.answers,
                        choice,
                        latency_ms: decision.latency_ms,
                    })
                    .await
                {
                    tracing::warn!(
                        message = notice.id,
                        "could not record exec gate decision: {error:#}"
                    );
                }
            }
            Err(error) => tracing::warn!(
                message = notice.id,
                "exec gate decision unavailable: {error:#}"
            ),
        }
    }
}

/// Record what the Exec did with the result notices one background wake consumed: an owner update
/// or the explicit quiet marker. A wake that also answered the owner is labelled apart, since the
/// owner's message forced a reply whatever the results merited.
pub async fn settle_exec_results(
    org: &OrgIntel,
    consumed: &[i64],
    updated_owner: bool,
    owner_wrote: bool,
) {
    if consumed.is_empty() {
        return;
    }
    let subjects: Vec<String> = consumed.iter().map(i64::to_string).collect();
    let outcome = match (owner_wrote, updated_owner) {
        (true, _) => "owner_wrote",
        (false, true) => "update",
        (false, false) => "quiet",
    };
    if let Err(error) = org
        .settle_decision_outcomes(EXEC_GATE, &subjects, outcome)
        .await
    {
        tracing::warn!("could not settle exec gate outcomes: {error:#}");
    }
}
