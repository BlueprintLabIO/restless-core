//! Host-side, mandate-gated Resend email effect.
//!
//! The caller resolves declared artifact references inside the host boundary,
//! asks Authority to reserve the matching permit, and only then dispatches the
//! prepared payload. This module never looks up or returns the provider key.

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

const RESEND_EMAILS_URL: &str = "https://api.resend.com/emails";
const MAX_SUBJECT_CHARS: usize = 998;
const MAX_BODY_BYTES: usize = 1_000_000;
const MAX_ATTACHMENTS: usize = 10;
const MAX_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;
const MAX_TOTAL_ATTACHMENT_BYTES: usize = 28 * 1024 * 1024;
const COMPANY_ATTACHMENT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);
const COMPANY_ATTACHMENT_RESOLVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);
const COMPANY_ATTACHMENT_READ_SCRIPT: &str = r#"
import os, stat, sys
limit = 25 * 1024 * 1024
fd = os.open(sys.argv[1], os.O_RDONLY | getattr(os, "O_CLOEXEC", 0))
try:
    info = os.fstat(fd)
    if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
        raise SystemExit(24)
    target = os.path.realpath("/proc/self/fd/" + str(fd))
    roots = ("/company/outputs", "/company/repos", "/company/workspaces",
             "/company/projects", "/company/knowledge", "/company/decisions",
             "/company/goals", "/company/documents", "/company/releases")
    if not any(target.startswith(root + "/") for root in roots):
        raise SystemExit(23)
    data = bytearray()
    while len(data) <= limit:
        chunk = os.read(fd, min(65536, limit + 1 - len(data)))
        if not chunk:
            break
        data.extend(chunk)
    if len(data) > limit:
        raise SystemExit(24)
    sys.stdout.buffer.write(data)
finally:
    os.close(fd)
"#;

/// A file the agent has explicitly declared it wants attached. `reference` is
/// an opaque artifact reference; the host resolves it and supplies the bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmailAttachmentRef {
    pub reference: String,
    pub filename: String,
    pub content_type: String,
}

/// A complete proposed email. There is intentionally exactly one recipient;
/// cc, bcc and reply-to are not accepted until their authority semantics exist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailSendRequest {
    /// Sender mailbox. The optional display name is a separate mandate-bound field.
    pub from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,
    pub to: String,
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(default)]
    pub attachments: Vec<EmailAttachmentRef>,
    pub effect_key: String,
    #[serde(default)]
    pub permit_id: Uuid,
}

/// Bytes resolved by the host for a declared attachment reference.
#[derive(Debug, Clone)]
pub struct ResolvedEmailAttachment {
    pub reference: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderAttachment {
    filename: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderPayload {
    from: String,
    reply_to: String,
    to: Vec<String>,
    subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<ProviderAttachment>,
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalAttachment<'a> {
    reference: &'a str,
    filename: &'a str,
    content_type: &'a str,
    sha256: String,
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalPayload<'a> {
    from: &'a str,
    from_name: &'a Option<String>,
    reply_to: &'a str,
    to: &'a str,
    subject: &'a str,
    text: &'a Option<String>,
    html: &'a Option<String>,
    attachments: Vec<CanonicalAttachment<'a>>,
}

/// Prepared, validated email whose digest identifies its complete actual
/// contents, including hashes of the attachment bytes.
#[derive(Debug, Clone)]
pub struct PreparedEmail {
    from: String,
    from_name: Option<String>,
    to: String,
    effect_key: String,
    permit_id: Uuid,
    payload_sha256: String,
    provider_payload: ProviderPayload,
}

impl PreparedEmail {
    pub fn sender(&self) -> &str {
        &self.from
    }

    pub fn sender_name(&self) -> Option<&str> {
        self.from_name.as_deref()
    }

    pub fn recipient(&self) -> &str {
        &self.to
    }

    pub fn payload_sha256(&self) -> &str {
        &self.payload_sha256
    }

    pub fn effect_key(&self) -> &str {
        &self.effect_key
    }

    pub fn permit_id(&self) -> Uuid {
        self.permit_id
    }

    /// Dispatch once, after the caller has reserved the permit. Any transport
    /// failure after attempting the request is reported as unknown; callers
    /// must reconcile and must not blindly retry.
    pub async fn send(&self, resend_production_key: &str) -> Result<EmailSendOutcome> {
        if resend_production_key.trim().is_empty()
            || resend_production_key
                .chars()
                .any(|c| c == '\r' || c == '\n')
        {
            bail!("Resend production credential is missing or malformed");
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("create Resend client")?;
        let response = match client
            .post(RESEND_EMAILS_URL)
            .bearer_auth(resend_production_key)
            .header("Idempotency-Key", &self.effect_key)
            .json(&self.provider_payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(_) => {
                return Ok(EmailSendOutcome::Unknown {
                    reason: "Resend request outcome could not be observed".to_owned(),
                })
            }
        };

        let status = response.status();
        if !status.is_success() {
            // Provider 4xx responses are explicit rejection. A server error
            // may follow an accepted send, so it remains ambiguous.
            return if status.is_server_error() {
                Ok(EmailSendOutcome::Unknown {
                    reason: format!("Resend returned HTTP {}", status.as_u16()),
                })
            } else {
                Ok(EmailSendOutcome::Rejected {
                    status: status.as_u16(),
                })
            };
        }
        let body: serde_json::Value =
            match response.json().await {
                Ok(body) => body,
                Err(_) => return Ok(EmailSendOutcome::Unknown {
                    reason:
                        "Resend accepted or processed the request but its receipt was unreadable"
                            .to_owned(),
                }),
            };
        match body.get("id").and_then(serde_json::Value::as_str) {
            Some(id) if !id.trim().is_empty() => Ok(EmailSendOutcome::Accepted {
                provider_id: id.to_owned(),
            }),
            _ => Ok(EmailSendOutcome::Unknown {
                reason: "Resend response did not contain a provider message ID".to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum EmailSendOutcome {
    Accepted { provider_id: String },
    Rejected { status: u16 },
    Unknown { reason: String },
}

/// Resolve each declared attachment from a productive directory in the
/// named company's running Runtime as the unprivileged `company` user, then
/// validate and prepare the exact email payload. The opened file descriptor's
/// real target is checked, so an in-tree symlink cannot escape these folders.
pub async fn prepare_from_company(
    company: &str,
    request: EmailSendRequest,
) -> Result<PreparedEmail> {
    crate::runtime::validate_company_name(company)?;
    for attachment in &request.attachments {
        validate_company_attachment_path(&attachment.reference)?;
    }
    let container = crate::runtime::container_name(company);
    let resolution = async {
        let mut attachments = Vec::with_capacity(request.attachments.len());
        let mut total_size = 0usize;
        for declared in &request.attachments {
            let output = crate::runtime::docker_bounded(
                &[
                    "exec",
                    "-u",
                    "company",
                    &container,
                    "python3",
                    "-c",
                    COMPANY_ATTACHMENT_READ_SCRIPT,
                    &declared.reference,
                ],
                COMPANY_ATTACHMENT_READ_TIMEOUT,
            )
            .await
            .context("read declared attachment from company Runtime")?;
            if !output.status.success() {
                bail!("declared email attachment is unavailable or outside the productive company folders");
            }
            if output.stdout.len() > MAX_ATTACHMENT_BYTES {
                bail!("an email attachment exceeds the 25 MB limit");
            }
            total_size = total_size
                .checked_add(output.stdout.len())
                .context("email attachment size overflow")?;
            if total_size > MAX_TOTAL_ATTACHMENT_BYTES {
                bail!("email attachments exceed the 28 MiB combined raw size limit");
            }
            attachments.push(ResolvedEmailAttachment {
                reference: declared.reference.clone(),
                bytes: output.stdout,
            });
        }
        Ok::<_, anyhow::Error>(attachments)
    };
    let resolved = tokio::time::timeout(COMPANY_ATTACHMENT_RESOLVE_TIMEOUT, resolution)
        .await
        .context("company attachment resolution exceeded its time limit")??;
    prepare_email(request, resolved)
}

fn validate_company_attachment_path(reference: &str) -> Result<()> {
    let Some(relative) = reference.strip_prefix("/company/") else {
        bail!("email attachment must name a file under a productive /company folder");
    };
    if relative.contains('\\') || relative.contains('\0') {
        bail!("email attachment path is malformed");
    }
    let mut parts = relative.split('/');
    let root = parts.next().unwrap_or_default();
    if !matches!(
        root,
        "outputs"
            | "repos"
            | "workspaces"
            | "projects"
            | "knowledge"
            | "decisions"
            | "goals"
            | "documents"
            | "releases"
    ) || parts.clone().next().is_none()
        || parts.any(|part| part.is_empty() || part == "." || part == "..")
    {
        bail!("email attachment must name a file under a productive /company folder");
    }
    Ok(())
}

/// Validate and canonicalize a request after the host has resolved every
/// declared reference. The exact resolved attachment set must match the
/// declaration set, preventing silent omission or injection of files.
pub fn prepare_email(
    request: EmailSendRequest,
    resolved_attachments: Vec<ResolvedEmailAttachment>,
) -> Result<PreparedEmail> {
    let from = canonical_address(&request.from, "from")?;
    let from_name = canonical_sender_name(request.from_name)?;
    let to = canonical_address(&request.to, "to")?;
    let subject = request.subject.trim().to_owned();
    if subject.is_empty()
        || subject.chars().count() > MAX_SUBJECT_CHARS
        || subject.chars().any(char::is_control)
    {
        bail!("email subject must be non-empty, at most 998 characters, and contain no control characters");
    }
    if request.text.as_deref().unwrap_or_default().is_empty()
        && request.html.as_deref().unwrap_or_default().is_empty()
    {
        bail!("email requires non-empty text or html content");
    }
    if request
        .text
        .as_ref()
        .is_some_and(|v| v.len() > MAX_BODY_BYTES)
        || request
            .html
            .as_ref()
            .is_some_and(|v| v.len() > MAX_BODY_BYTES)
    {
        bail!("email body exceeds the 1 MB limit");
    }
    if request.effect_key.trim().is_empty()
        || request.effect_key.len() > 200
        || request.effect_key.chars().any(char::is_control)
    {
        bail!("email effect key must be non-empty, at most 200 bytes, and contain no control characters");
    }
    if request.attachments.len() > MAX_ATTACHMENTS {
        bail!("email may include at most 10 attachments");
    }

    let mut declarations = request.attachments.clone();
    declarations.sort_by(|a, b| a.reference.cmp(&b.reference));
    if declarations.iter().any(|a| {
        a.reference.trim().is_empty()
            || a.filename.trim().is_empty()
            || a.filename.len() > 255
            || a.filename.chars().any(char::is_control)
            || a.content_type.trim().is_empty()
            || a.content_type.len() > 255
            || a.content_type.chars().any(char::is_control)
    }) {
        bail!("email attachment reference, filename, or content type is invalid");
    }
    if declarations
        .windows(2)
        .any(|pair| pair[0].reference == pair[1].reference)
    {
        bail!("email attachment references must be unique");
    }

    let mut resolved = resolved_attachments;
    resolved.sort_by(|a, b| a.reference.cmp(&b.reference));
    if resolved.len() != declarations.len()
        || resolved
            .iter()
            .zip(&declarations)
            .any(|(r, d)| r.reference != d.reference)
    {
        bail!("resolved email attachments do not exactly match the declared references");
    }
    let total_size = resolved.iter().try_fold(0usize, |sum, item| {
        if item.bytes.len() > MAX_ATTACHMENT_BYTES {
            bail!("an email attachment exceeds the 25 MB limit");
        }
        sum.checked_add(item.bytes.len())
            .context("email attachment size overflow")
    })?;
    if total_size > MAX_TOTAL_ATTACHMENT_BYTES {
        bail!("email attachments exceed the 28 MiB combined raw size limit");
    }

    let mut provider_attachments = Vec::with_capacity(declarations.len());
    let mut canonical_attachments = Vec::with_capacity(declarations.len());
    for (declared, actual) in declarations.iter().zip(&resolved) {
        canonical_attachments.push(CanonicalAttachment {
            reference: &declared.reference,
            filename: &declared.filename,
            content_type: &declared.content_type,
            sha256: hex_sha256(&actual.bytes),
        });
        provider_attachments.push(ProviderAttachment {
            filename: declared.filename.clone(),
            content: base64::engine::general_purpose::STANDARD.encode(&actual.bytes),
        });
    }
    let text = request.text;
    let html = request.html;
    let canonical = CanonicalPayload {
        from: &from,
        from_name: &from_name,
        reply_to: &from,
        to: &to,
        subject: &subject,
        text: &text,
        html: &html,
        attachments: canonical_attachments,
    };
    let payload_sha256 =
        hex_sha256(&serde_json::to_vec(&canonical).context("serialize canonical email payload")?);
    let provider_from = match from_name.as_deref() {
        Some(name) => format!(
            "\"{}\" <{}>",
            name.replace('\\', "\\\\").replace('\"', "\\\""),
            from
        ),
        None => from.clone(),
    };
    let provider_payload = ProviderPayload {
        from: provider_from,
        reply_to: from.clone(),
        to: vec![to.clone()],
        subject,
        text,
        html,
        attachments: provider_attachments,
    };
    Ok(PreparedEmail {
        from,
        from_name,
        to,
        effect_key: request.effect_key.trim().to_owned(),
        permit_id: request.permit_id,
        payload_sha256,
        provider_payload,
    })
}

fn canonical_sender_name(value: Option<String>) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty()
        || value.chars().count() > 120
        || value.chars().any(char::is_control)
        || value.chars().any(|c| c == '<' || c == '>')
    {
        bail!("sender display name must be non-empty, at most 120 characters, and contain no control or address-delimiter characters");
    }
    Ok(Some(value.to_owned()))
}

fn canonical_address(value: &str, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 320
        || !value.is_ascii()
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        bail!("email {field} address is malformed");
    }
    let mut parts = value.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    if local.is_empty()
        || domain.is_empty()
        || parts.next().is_some()
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
    {
        bail!("email {field} address is malformed");
    }
    Ok(format!(
        "{}@{}",
        local.to_ascii_lowercase(),
        domain.to_ascii_lowercase()
    ))
}

fn hex_sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
