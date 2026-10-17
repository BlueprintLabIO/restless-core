//! Ordinary company bookkeeping. Observations here neither move money nor authorise a payment.
use super::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AccountingInput {
    pub kind: String,
    pub description: String,
    pub counterparty: String,
    pub category: String,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub occurred_on: NaiveDate,
    /// Receipt/invoice locator or a plain source note. Never assumed to prove settlement.
    pub evidence: String,
}

impl AccountingInput {
    pub fn validate(&self) -> Result<()> {
        let invalid = |s: &str| OrgIntelError::InvalidAccounting(s.into());
        if !matches!(self.kind.as_str(), "income" | "expense")
            || !matches!(self.status.as_str(), "settled" | "pending" | "void")
        {
            return Err(invalid(
                "Choose income or expense and a valid payment status.",
            ));
        }
        if self.amount_minor <= 0 || self.amount_minor > 9_007_199_254_740_991 {
            return Err(invalid(
                "Enter a positive amount within the supported range.",
            ));
        }
        if self.currency.len() != 3 || !self.currency.bytes().all(|c| c.is_ascii_uppercase()) {
            return Err(invalid("Use a three-letter uppercase currency code."));
        }
        if self.description.trim().is_empty()
            || self.description.len() > 500
            || self.counterparty.len() > 300
            || self.category.len() > 100
            || self.evidence.len() > 4000
        {
            return Err(invalid(
                "Add a description and keep the record's fields within their length limits.",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct AccountingRecord {
    pub id: Uuid,
    pub kind: String,
    pub description: String,
    pub counterparty: String,
    pub category: String,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub occurred_on: NaiveDate,
    pub evidence: String,
    pub version: i64,
    pub recorded_by: String,
    pub updated_at: DateTime<Utc>,
}

impl OrgIntel {
    pub async fn accounting_records(&self) -> Result<Vec<AccountingRecord>> {
        Ok(
            sqlx::query_as("SELECT * FROM accounting_records ORDER BY occurred_on DESC, id")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    /// Stable client ids make an uncertain create retry harmless. Edits compare versions so a
    /// stale browser cannot overwrite someone else's correction. Voiding retains the evidence.
    pub async fn save_accounting_record(
        &self,
        id: Uuid,
        version: i64,
        input: &AccountingInput,
        actor: &str,
    ) -> Result<AccountingRecord> {
        input.validate()?;
        let row = sqlx::query_as::<_, AccountingRecord>(
            "INSERT INTO accounting_records \
             (id,kind,description,counterparty,category,amount_minor,currency,status,occurred_on,evidence,recorded_by) \
             SELECT $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11 WHERE $12 = 0 \
             ON CONFLICT (id) DO NOTHING RETURNING *")
            .bind(id).bind(&input.kind).bind(&input.description).bind(&input.counterparty)
            .bind(&input.category).bind(input.amount_minor).bind(&input.currency).bind(&input.status)
            .bind(input.occurred_on).bind(&input.evidence).bind(actor).bind(version)
            .fetch_optional(&self.pool).await?;
        if let Some(row) = row {
            return Ok(row);
        }
        // An exact replay returns the current record without incrementing its version.
        let existing =
            sqlx::query_as::<_, AccountingRecord>("SELECT * FROM accounting_records WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        if let Some(row) = existing {
            if row.kind == input.kind
                && row.description == input.description
                && row.counterparty == input.counterparty
                && row.category == input.category
                && row.amount_minor == input.amount_minor
                && row.currency == input.currency
                && row.status == input.status
                && row.occurred_on == input.occurred_on
                && row.evidence == input.evidence
            {
                return Ok(row);
            }
        }
        sqlx::query_as("UPDATE accounting_records SET kind=$2,description=$3,counterparty=$4,category=$5,amount_minor=$6,\
            currency=$7,status=$8,occurred_on=$9,evidence=$10,recorded_by=$11,version=version+1,updated_at=now() \
            WHERE id=$1 AND version=$12 RETURNING *")
            .bind(id).bind(&input.kind).bind(&input.description).bind(&input.counterparty)
            .bind(&input.category).bind(input.amount_minor).bind(&input.currency).bind(&input.status)
            .bind(input.occurred_on).bind(&input.evidence).bind(actor).bind(version)
            .fetch_optional(&self.pool).await?
            .ok_or_else(|| OrgIntelError::InvalidAccounting("This record changed. Refresh before saving your correction.".into()))
    }
}
