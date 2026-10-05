//! Personal access for an outside MCP client (Claude Code, Claude Desktop,
//! Codex). A token stands in for the person who issued it, never for more:
//! the owner gateway rebuilds that person's principal on every call and
//! re-checks their membership, so a removed member's token stops working at
//! once. Only a SHA-256 of the token is stored.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::entry::VerifiedIdentity;

const TOKEN_PREFIX: &str = "rsl_mcp_";

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.mcp_access_tokens (\
           id UUID PRIMARY KEY, token_sha256 TEXT NOT NULL UNIQUE, \
           holder TEXT NOT NULL, label TEXT NOT NULL, identity JSONB, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), last_used_at TIMESTAMPTZ, \
           revoked_at TIMESTAMPTZ\
         )",
    )
    .execute(pool)
    .await
    .context("create mcp access tokens")?;
    Ok(())
}

/// Who a token speaks for. `identity` is absent for the local owner.
#[derive(Debug, Clone)]
pub struct Access {
    pub id: Uuid,
    pub identity: Option<VerifiedIdentity>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccessToken {
    pub id: Uuid,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// The stable key of one person: the owner, or one member of one company.
pub fn holder(identity: Option<&VerifiedIdentity>) -> String {
    match identity {
        None => "owner".into(),
        Some(identity) => match &identity.scope {
            crate::entry::CompanyScope::Owner => "owner".into(),
            crate::entry::CompanyScope::Company { company } => format!(
                "{company}/{}",
                identity.actor.as_deref().unwrap_or(&identity.user)
            ),
        },
    }
}

fn digest(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

/// Issue a token; the plaintext is returned once and never stored.
pub async fn issue(
    pool: &PgPool,
    identity: Option<&VerifiedIdentity>,
    label: &str,
) -> Result<(AccessToken, String)> {
    let label = label.trim();
    anyhow::ensure!(
        !label.is_empty() && label.chars().count() <= 80,
        "name the token in 1 to 80 characters"
    );
    let token = format!(
        "{TOKEN_PREFIX}{}{}",
        Uuid::new_v4().simple(),
        Uuid::new_v4().simple()
    );
    let id = Uuid::new_v4();
    let identity_json = identity.map(serde_json::to_value).transpose()?;
    let created_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO restless_authority.mcp_access_tokens \
           (id, token_sha256, holder, label, identity) VALUES ($1, $2, $3, $4, $5) \
         RETURNING created_at",
    )
    .bind(id)
    .bind(digest(&token))
    .bind(holder(identity))
    .bind(label)
    .bind(identity_json)
    .fetch_one(pool)
    .await
    .context("issue mcp access token")?;
    Ok((
        AccessToken {
            id,
            label: label.into(),
            created_at,
            last_used_at: None,
        },
        token,
    ))
}

pub async fn list(pool: &PgPool, holder: &str) -> Result<Vec<AccessToken>> {
    let rows: Vec<(Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, label, created_at, last_used_at FROM restless_authority.mcp_access_tokens \
         WHERE holder = $1 AND revoked_at IS NULL ORDER BY created_at DESC",
    )
    .bind(holder)
    .fetch_all(pool)
    .await
    .context("list mcp access tokens")?;
    Ok(rows
        .into_iter()
        .map(|(id, label, created_at, last_used_at)| AccessToken {
            id,
            label,
            created_at,
            last_used_at,
        })
        .collect())
}

/// Revoke one of the holder's own tokens. False when it is not theirs.
pub async fn revoke(pool: &PgPool, holder: &str, id: Uuid) -> Result<bool> {
    let revoked = sqlx::query(
        "UPDATE restless_authority.mcp_access_tokens SET revoked_at = now() \
         WHERE id = $1 AND holder = $2 AND revoked_at IS NULL",
    )
    .bind(id)
    .bind(holder)
    .execute(pool)
    .await
    .context("revoke mcp access token")?;
    Ok(revoked.rows_affected() == 1)
}

/// The person a presented token speaks for, if it is live.
pub async fn resolve(pool: &PgPool, token: &str) -> Result<Option<Access>> {
    if !token.starts_with(TOKEN_PREFIX) {
        return Ok(None);
    }
    let row: Option<(Uuid, Option<serde_json::Value>)> = sqlx::query_as(
        "UPDATE restless_authority.mcp_access_tokens SET last_used_at = now() \
         WHERE token_sha256 = $1 AND revoked_at IS NULL RETURNING id, identity",
    )
    .bind(digest(token))
    .fetch_optional(pool)
    .await
    .context("resolve mcp access token")?;
    let Some((id, identity)) = row else {
        return Ok(None);
    };
    Ok(Some(Access {
        id,
        identity: identity.map(serde_json::from_value).transpose()?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> Option<PgPool> {
        let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
        let pool = PgPool::connect(&url).await.ok()?;
        sqlx::query("CREATE SCHEMA IF NOT EXISTS restless_authority")
            .execute(&pool)
            .await
            .ok()?;
        ensure_schema(&pool).await.ok()?;
        Some(pool)
    }

    fn member(company: &str, actor: &str) -> VerifiedIdentity {
        VerifiedIdentity {
            user: format!("user-{actor}"),
            issuer: None,
            owner: "acct".into(),
            scope: crate::entry::CompanyScope::Company {
                company: company.into(),
            },
            role: "member".into(),
            actor: Some(actor.into()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some("m1".into()),
            membership_version: Some(3),
        }
    }

    #[tokio::test]
    async fn a_token_speaks_only_for_its_holder_until_revoked() {
        let Some(pool) = pool().await else {
            eprintln!("skipped: RESTLESS_TEST_DATABASE_URL is not set");
            return;
        };
        let company = format!("acc_{}_test", Uuid::new_v4().simple());
        let ann = member(&company, "ann");
        let (issued, token) = issue(&pool, Some(&ann), "laptop").await.unwrap();

        let stored: String = sqlx::query_scalar(
            "SELECT token_sha256 FROM restless_authority.mcp_access_tokens WHERE id = $1",
        )
        .bind(issued.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_ne!(stored, token, "only the digest is stored");

        let access = resolve(&pool, &token).await.unwrap().expect("live token");
        assert_eq!(access.identity.as_ref(), Some(&ann));
        assert!(resolve(&pool, &format!("{token}x"))
            .await
            .unwrap()
            .is_none());

        let bob = holder(Some(&member(&company, "bob")));
        assert!(
            !revoke(&pool, &bob, issued.id).await.unwrap(),
            "not bob's token"
        );
        assert!(list(&pool, &bob).await.unwrap().is_empty());

        let ann_holder = holder(Some(&ann));
        assert_eq!(list(&pool, &ann_holder).await.unwrap().len(), 1);
        assert!(revoke(&pool, &ann_holder, issued.id).await.unwrap());
        assert!(resolve(&pool, &token).await.unwrap().is_none());
    }
}
