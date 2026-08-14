use std::collections::BTreeSet;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac as _};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::{GatewayError, GatewayResult, SecretBytes};

pub const PURPOSE_TOKEN_VERSION: u16 = 1;
const TOKEN_PREFIX: &str = "company-purpose-v1";
const MAXIMUM_LIFETIME_SECONDS: i64 = 3_600;

/// Per-token, fail-closed HTTP bounds.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PurposeTokenLimits {
    pub maximum_requests: u32,
    pub maximum_request_bytes: u64,
    pub maximum_response_bytes: u64,
}

/// Authority-free routing claims signed by companyd for one execution.
/// Possession grants only the explicitly named provider request envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PurposeTokenClaims {
    pub schema_version: u16,
    pub token_id: Uuid,
    pub company_id: String,
    pub actor_id: String,
    pub execution_id: Uuid,
    pub audience: String,
    pub issued_at: DateTime<Utc>,
    pub not_before: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub allowed_paths: BTreeSet<String>,
    pub allowed_models: BTreeSet<String>,
    pub limits: PurposeTokenLimits,
}

impl PurposeTokenClaims {
    /// Validate structural and temporal token invariants at an exact instant.
    ///
    /// # Errors
    ///
    /// Returns an error for blank identities, invalid lifetimes, broad paths,
    /// unsupported schemas, or zero/unbounded limits.
    pub fn validate_at(&self, now: DateTime<Utc>, audience: &str) -> GatewayResult<()> {
        if self.schema_version != PURPOSE_TOKEN_VERSION
            || self.token_id.is_nil()
            || self.execution_id.is_nil()
            || !valid_opaque_identity(&self.company_id)
            || !valid_opaque_identity(&self.actor_id)
            || self.audience != audience
        {
            return Err(GatewayError::Forbidden);
        }
        if self.not_before < self.issued_at
            || self.expires_at <= self.not_before
            || self.expires_at - self.issued_at > Duration::seconds(MAXIMUM_LIFETIME_SECONDS)
            || now < self.not_before
            || now >= self.expires_at
        {
            return Err(GatewayError::Forbidden);
        }
        if self.allowed_paths.is_empty()
            || self.allowed_paths.len() > 8
            || self
                .allowed_paths
                .iter()
                .any(|path| !matches!(path.as_str(), "/v1/responses" | "/v1/responses/compact"))
            || self.allowed_models.is_empty()
            || self.allowed_models.len() > 16
            || self.allowed_models.iter().any(|model| {
                model.is_empty()
                    || model.len() > 160
                    || !model.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
            })
            || self.limits.maximum_requests == 0
            || self.limits.maximum_requests > 1_024
            || !(1..=16 * 1024 * 1024).contains(&self.limits.maximum_request_bytes)
            || !(1..=1024 * 1024 * 1024).contains(&self.limits.maximum_response_bytes)
        {
            return Err(GatewayError::Forbidden);
        }
        Ok(())
    }
}

fn valid_opaque_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.contains("://")
        && !value.contains(['/', '\\', '\0'])
        && !value.bytes().any(|byte| byte.is_ascii_whitespace())
}

/// HMAC-SHA256 codec for compact execution purpose tokens.
#[derive(Clone, Debug)]
pub struct PurposeTokenCodec {
    signing_key: SecretBytes,
    audience: String,
}

impl PurposeTokenCodec {
    /// Construct a codec from a high-entropy signing key and exact audience.
    ///
    /// # Errors
    ///
    /// Returns an error for keys shorter than 32 bytes or an invalid audience.
    pub fn new(signing_key: SecretBytes, audience: impl Into<String>) -> GatewayResult<Self> {
        let audience = audience.into();
        if signing_key.expose().len() < 32
            || audience.is_empty()
            || audience.len() > 160
            || audience.bytes().any(|byte| byte.is_ascii_whitespace())
        {
            return Err(GatewayError::Configuration(
                "purpose-token key/audience is invalid".into(),
            ));
        }
        Ok(Self {
            signing_key,
            audience,
        })
    }

    /// Sign validated claims.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid claims or serialization failure.
    pub fn issue_at(
        &self,
        claims: &PurposeTokenClaims,
        now: DateTime<Utc>,
    ) -> GatewayResult<String> {
        claims.validate_at(now, &self.audience)?;
        let payload = serde_json::to_vec(claims)
            .map_err(|_| GatewayError::Configuration("serialize purpose token".into()))?;
        let encoded = URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{TOKEN_PREFIX}.{encoded}");
        let signature = self.sign(signing_input.as_bytes())?;
        Ok(format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }

    /// Verify signature, schema, audience, and time bounds.
    ///
    /// # Errors
    ///
    /// Returns `Unauthorized` for every malformed or forged token and
    /// `Forbidden` for a validly signed token outside its purpose.
    pub fn verify_at(&self, token: &str, now: DateTime<Utc>) -> GatewayResult<PurposeTokenClaims> {
        let mut segments = token.split('.');
        let prefix = segments.next();
        let payload = segments.next();
        let signature = segments.next();
        if prefix != Some(TOKEN_PREFIX)
            || payload.is_none()
            || signature.is_none()
            || segments.next().is_some()
            || token.len() > 64 * 1024
        {
            return Err(GatewayError::Unauthorized);
        }
        let payload = payload.unwrap_or_default();
        let signature = URL_SAFE_NO_PAD
            .decode(signature.unwrap_or_default())
            .map_err(|_| GatewayError::Unauthorized)?;
        let signing_input = format!("{TOKEN_PREFIX}.{payload}");
        let mut mac = Hmac::<Sha256>::new_from_slice(self.signing_key.expose())
            .map_err(|_| GatewayError::Unauthorized)?;
        mac.update(signing_input.as_bytes());
        mac.verify_slice(&signature)
            .map_err(|_| GatewayError::Unauthorized)?;
        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| GatewayError::Unauthorized)?;
        let claims = serde_json::from_slice::<PurposeTokenClaims>(&payload)
            .map_err(|_| GatewayError::Unauthorized)?;
        claims.validate_at(now, &self.audience)?;
        Ok(claims)
    }

    fn sign(&self, input: &[u8]) -> GatewayResult<Vec<u8>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.signing_key.expose())
            .map_err(|_| GatewayError::Configuration("invalid HMAC key".into()))?;
        mac.update(input);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(now: DateTime<Utc>) -> PurposeTokenClaims {
        PurposeTokenClaims {
            schema_version: PURPOSE_TOKEN_VERSION,
            token_id: Uuid::new_v4(),
            company_id: "company-1".into(),
            actor_id: "actor-1".into(),
            execution_id: Uuid::new_v4(),
            audience: "model-gateway".into(),
            issued_at: now,
            not_before: now,
            expires_at: now + Duration::minutes(5),
            allowed_paths: BTreeSet::from(["/v1/responses".into()]),
            allowed_models: BTreeSet::from(["gpt-5.1-codex".into()]),
            limits: PurposeTokenLimits {
                maximum_requests: 3,
                maximum_request_bytes: 1_024,
                maximum_response_bytes: 4_096,
            },
        }
    }

    #[test]
    fn token_round_trip_rejects_tampering_and_expiry() {
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![7; 32]).unwrap(), "model-gateway")
            .unwrap();
        let token = codec.issue_at(&claims(now), now).unwrap();
        let verified = codec.verify_at(&token, now).unwrap();
        assert_eq!(verified.company_id, "company-1");

        let mut forged = token.into_bytes();
        let last = forged.len() - 1;
        forged[last] = if forged[last] == b'A' { b'B' } else { b'A' };
        assert!(matches!(
            codec.verify_at(std::str::from_utf8(&forged).unwrap(), now),
            Err(GatewayError::Unauthorized)
        ));

        let valid = codec.issue_at(&claims(now), now).unwrap();
        assert!(matches!(
            codec.verify_at(&valid, now + Duration::minutes(6)),
            Err(GatewayError::Forbidden)
        ));
    }

    #[test]
    fn token_scope_rejects_unbounded_or_path_like_company_identities() {
        let now = Utc::now();
        let codec = PurposeTokenCodec::new(SecretBytes::new(vec![7; 32]).unwrap(), "model-gateway")
            .unwrap();
        for invalid in [
            String::new(),
            "company/other".into(),
            "https://identity.example".into(),
            "company\nother".into(),
            "x".repeat(257),
        ] {
            let mut invalid_claims = claims(now);
            invalid_claims.company_id = invalid;
            assert!(matches!(
                codec.issue_at(&invalid_claims, now),
                Err(GatewayError::Forbidden)
            ));
        }
    }
}
