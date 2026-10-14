//! The owner's company-wide pause of outside actions.
//!
//! One switch, two existing mechanisms. Pausing freezes every tool connection
//! and payment allowance that is not already frozen, so Apps and Limits show
//! the same truth, and records which ones it froze. While paused, the effect
//! gate refuses every new consequential effect (governed commands, tool acts,
//! mandated email and payment reservations) before any intent is claimed.
//! Resuming lifts the gate and unfreezes exactly what the pause froze; a
//! connection or allowance the owner froze separately stays frozen.

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::authority::AuthorityStore;

pub const PAUSED: &str = "outside_actions_paused";
pub const RESUMED: &str = "outside_actions_resumed";

#[derive(Debug, Clone, Serialize)]
pub struct Pause {
    pub paused_at: DateTime<Utc>,
    pub paused_by: Option<String>,
    pub frozen_connections: Vec<String>,
    pub frozen_currencies: Vec<String>,
}

/// The pause in force, if the latest pause/resume record is a pause.
pub async fn current(authority: &AuthorityStore, company: &str) -> Result<Option<Pause>> {
    let (paused, resumed) = tokio::try_join!(
        authority.recent_records_of_kind(company, PAUSED, 1),
        authority.recent_records_of_kind(company, RESUMED, 1),
    )?;
    let Some(pause) = paused.into_iter().next() else {
        return Ok(None);
    };
    if resumed.first().is_some_and(|resume| resume.id > pause.id) {
        return Ok(None);
    }
    let names = |field: &str| -> Vec<String> {
        pause.body[field]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| value.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    Ok(Some(Pause {
        paused_at: pause.created_at,
        paused_by: pause.actor_id.clone(),
        frozen_connections: names("frozen_connections"),
        frozen_currencies: names("frozen_currencies"),
    }))
}

/// The effect gate: refuse a new outside action while the owner has paused them.
pub async fn ensure_running(authority: &AuthorityStore, company: &str) -> Result<()> {
    if current(authority, company).await?.is_some() {
        bail!(
            "the owner paused outside actions for this company; nothing reaches outside parties until they resume. Prepare the work and wait."
        );
    }
    Ok(())
}

pub async fn pause(authority: &AuthorityStore, company: &str, by: &str) -> Result<Pause> {
    if let Some(pause) = current(authority, company).await? {
        return Ok(pause);
    }
    // The gate goes up first, so nothing slips through while freezes apply.
    authority
        .emit(
            company,
            PAUSED,
            Some(by),
            serde_json::json!({ "frozen_connections": [], "frozen_currencies": [] }),
        )
        .await?;
    let pool = authority.pool();
    let mut frozen_connections = Vec::new();
    for connection in crate::connections::list(pool, company).await? {
        if !connection.frozen {
            crate::connections::set_frozen(pool, authority, company, &connection.name, true, by)
                .await?;
            frozen_connections.push(connection.name);
        }
    }
    let mut frozen_currencies = Vec::new();
    for envelope in crate::finance::envelopes(authority, company).await? {
        if !envelope.limits.frozen {
            crate::finance::set_frozen(authority, company, &envelope.limits.currency, true, by)
                .await?;
            frozen_currencies.push(envelope.limits.currency);
        }
    }
    // Record what this pause froze so resume can undo exactly that.
    authority
        .emit(
            company,
            PAUSED,
            Some(by),
            serde_json::json!({
                "frozen_connections": frozen_connections,
                "frozen_currencies": frozen_currencies,
            }),
        )
        .await?;
    Ok(current(authority, company)
        .await?
        .expect("a pause was just recorded"))
}

pub async fn resume(authority: &AuthorityStore, company: &str, by: &str) -> Result<()> {
    let Some(pause) = current(authority, company).await? else {
        return Ok(());
    };
    let pool = authority.pool();
    for name in &pause.frozen_connections {
        if let Err(error) =
            crate::connections::set_frozen(pool, authority, company, name, false, by).await
        {
            tracing::warn!(company, connection = %name, %error, "could not unfreeze after resume");
        }
    }
    for currency in &pause.frozen_currencies {
        if let Err(error) =
            crate::finance::set_frozen(authority, company, currency, false, by).await
        {
            tracing::warn!(company, %currency, %error, "could not unfreeze allowance after resume");
        }
    }
    authority
        .emit(company, RESUMED, Some(by), serde_json::json!({}))
        .await?;
    Ok(())
}
