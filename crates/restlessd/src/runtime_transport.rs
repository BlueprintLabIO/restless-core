//! One Core-side interface to a company computer.
//!
//! Self-hosted Core implements this interface with its local Docker runtime.
//! Hosted Core implements it with the outbound Runtime Agent connection. The
//! product code above this boundary must not know which transport carries the
//! request, and network mode must never fall back to a host process or Docker.

use std::{fmt, pin::Pin, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::io::{AsyncRead, AsyncWrite};
use uuid::Uuid;

pub type RuntimeRead = Pin<Box<dyn AsyncRead + Send + 'static>>;
pub type RuntimeWrite = Pin<Box<dyn AsyncWrite + Send + 'static>>;

pub trait RuntimeIo: AsyncRead + AsyncWrite + Send + Unpin {}

impl<T> RuntimeIo for T where T: AsyncRead + AsyncWrite + Send + Unpin {}

pub type RuntimeDuplex = Box<dyn RuntimeIo + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeTransportError {
    Unavailable,
    Unauthorized,
    InvalidRequest(&'static str),
    NotFound,
    Conflict,
    DeadlineExceeded,
    Remote(String),
    Transport(String),
}

impl fmt::Display for RuntimeTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => formatter.write_str("company Runtime is unavailable"),
            Self::Unauthorized => formatter.write_str("company Runtime request was not authorized"),
            Self::InvalidRequest(reason) => formatter.write_str(reason),
            Self::NotFound => formatter.write_str("company Runtime resource was not found"),
            Self::Conflict => {
                formatter.write_str("company Runtime identity or operation conflicted")
            }
            Self::DeadlineExceeded => {
                formatter.write_str("company Runtime operation deadline elapsed")
            }
            Self::Remote(reason) => {
                write!(formatter, "company Runtime refused the operation: {reason}")
            }
            Self::Transport(reason) => {
                write!(formatter, "company Runtime transport failed: {reason}")
            }
        }
    }
}

impl std::error::Error for RuntimeTransportError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeReadiness {
    pub runtime_id: String,
    pub runtime_generation: i64,
    pub runtime_image: String,
    pub source_revision: String,
    pub volume_name: String,
    pub observed_at: DateTime<Utc>,
    pub components: Vec<RuntimeComponentCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeComponentCheck {
    pub name: String,
    pub status: RuntimeComponentStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeComponentStatus {
    Ready,
    Degraded,
}

/// The account plane may launch productive work only with its durable
/// Authority identities attached. Small infrastructure probes are a distinct
/// purpose so they cannot be mistaken for unattributed company work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeProcessAuthority {
    Attempt {
        company: String,
        actor: String,
        responsibility: String,
        work_id: Uuid,
        attempt_id: Uuid,
        session_id: String,
    },
    /// Productive work that is durably rooted in an OrgIntel event rather
    /// than a claimed Work Attempt (for example an Exec portfolio wake or a
    /// two-person conversation turn). The event is created before process
    /// launch; an arbitrary daemon-generated UUID is not equivalent evidence.
    AuthorityEvent {
        company: String,
        actor: String,
        responsibility: String,
        event_id: i64,
        session_id: String,
    },
    /// One phase of an Authority-governed external effect. The referenced
    /// Authority record is created before any Runtime process starts; the
    /// phase then selects the least-privileged Runtime identity required for
    /// preparation, isolated execution, cleanup, or restart recovery.
    GovernedEffect {
        company: String,
        actor: String,
        effect_class: String,
        authority_id: i64,
        idempotency_key: String,
        execution_no: i32,
        staging_id: Uuid,
        phase: RuntimeEffectPhase,
    },
    InfrastructureProbe {
        company: String,
        probe: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEffectPhase {
    WorkspaceAccess,
    ArtifactStage,
    Execute,
    ArtifactCleanup,
    RecoverCompanyProcess,
    RecoverEffectProcess,
}

impl RuntimeEffectPhase {
    /// External-effect commands and their private staging lifecycle run as
    /// the dedicated effect UID. Workspace permission migration and recovery
    /// of a company-UID preparation child must instead run as the company UID.
    pub fn uses_effect_identity(self) -> bool {
        !matches!(self, Self::WorkspaceAccess | Self::RecoverCompanyProcess)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorkspaceAccess => "workspace_access",
            Self::ArtifactStage => "artifact_stage",
            Self::Execute => "execute",
            Self::ArtifactCleanup => "artifact_cleanup",
            Self::RecoverCompanyProcess => "recover_company_process",
            Self::RecoverEffectProcess => "recover_effect_process",
        }
    }
}

impl RuntimeProcessAuthority {
    pub fn validate(&self) -> Result<(), RuntimeTransportError> {
        let company = match self {
            Self::Attempt {
                company,
                actor,
                responsibility,
                work_id,
                attempt_id,
                session_id,
            } => {
                if work_id.is_nil() || attempt_id.is_nil() {
                    return Err(RuntimeTransportError::InvalidRequest(
                        "productive Runtime processes require non-nil Work and Attempt identities",
                    ));
                }
                validate_authority_labels([
                    actor.as_str(),
                    responsibility.as_str(),
                    session_id.as_str(),
                ])?;
                company
            }
            Self::AuthorityEvent {
                company,
                actor,
                responsibility,
                event_id,
                session_id,
            } => {
                if *event_id < 1 {
                    return Err(RuntimeTransportError::InvalidRequest(
                        "productive Runtime event authority requires a positive durable event ID",
                    ));
                }
                validate_authority_labels([
                    actor.as_str(),
                    responsibility.as_str(),
                    session_id.as_str(),
                ])?;
                company
            }
            Self::GovernedEffect {
                company,
                actor,
                effect_class,
                authority_id,
                idempotency_key,
                execution_no,
                staging_id,
                ..
            } => {
                if *authority_id < 1 || *execution_no < 1 || staging_id.is_nil() {
                    return Err(RuntimeTransportError::InvalidRequest(
                        "governed effect authority requires an exact durable record, execution, and staging identity",
                    ));
                }
                validate_authority_labels([
                    actor.as_str(),
                    effect_class.as_str(),
                    idempotency_key.as_str(),
                ])?;
                company
            }
            Self::InfrastructureProbe { company, probe } => {
                validate_authority_labels([probe.as_str()])?;
                company
            }
        };
        validate_company(company)?;
        Ok(())
    }

    pub fn company(&self) -> &str {
        match self {
            Self::Attempt { company, .. }
            | Self::AuthorityEvent { company, .. }
            | Self::GovernedEffect { company, .. }
            | Self::InfrastructureProbe { company, .. } => company,
        }
    }
}

fn validate_authority_labels<'a>(
    labels: impl IntoIterator<Item = &'a str>,
) -> Result<(), RuntimeTransportError> {
    if labels
        .into_iter()
        .any(|value| value.is_empty() || value.len() > 256 || value.contains(['\0', '\r', '\n']))
    {
        return Err(RuntimeTransportError::InvalidRequest(
            "Runtime process authority labels must be bounded single-line values",
        ));
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub struct RuntimeEnvironment {
    pub name: String,
    pub value: String,
    pub sensitive: bool,
}

impl RuntimeEnvironment {
    pub fn public(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            sensitive: false,
        }
    }

    pub fn secret(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            sensitive: true,
        }
    }

    fn validate(&self) -> Result<(), RuntimeTransportError> {
        if self.name.is_empty()
            || self.name.len() > 128
            || !self.name.bytes().enumerate().all(|(index, byte)| {
                byte == b'_' || byte.is_ascii_uppercase() || (index > 0 && byte.is_ascii_digit())
            })
            || self.value.len() > 32 * 1024
            || self.value.contains('\0')
        {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime process environment is invalid or too large",
            ));
        }
        Ok(())
    }
}

impl fmt::Debug for RuntimeEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut value = formatter.debug_struct("RuntimeEnvironment");
        value.field("name", &self.name);
        if self.sensitive {
            value.field("value", &"[REDACTED]");
        } else {
            value.field("value", &self.value);
        }
        value.field("sensitive", &self.sensitive).finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProcessSpec {
    pub operation_id: Uuid,
    pub authority: RuntimeProcessAuthority,
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: CompanyPath,
    pub environment: Vec<RuntimeEnvironment>,
    pub deadline: DateTime<Utc>,
}

impl RuntimeProcessSpec {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<(), RuntimeTransportError> {
        if self.operation_id.is_nil() {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime operation ID must be non-nil",
            ));
        }
        self.authority.validate()?;
        if self.executable.is_empty()
            || self.executable.len() > 1024
            || self.executable.contains(['\0', '\r', '\n'])
            || self.arguments.len() > 256
            || self
                .arguments
                .iter()
                .any(|argument| argument.len() > 16 * 1024 || argument.contains('\0'))
            || self.arguments.iter().map(String::len).sum::<usize>() > 128 * 1024
            || self.environment.len() > 128
        {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime process command is invalid or too large",
            ));
        }
        for variable in &self.environment {
            variable.validate()?;
        }
        if self.deadline <= now || self.deadline > now + chrono::Duration::hours(24) {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime process deadline must be in the next 24 hours",
            ));
        }
        Ok(())
    }
}

pub struct RuntimeProcess {
    pub process_id: Uuid,
    pub pid: u32,
    pub stdin: RuntimeWrite,
    pub stdout: RuntimeRead,
    pub stderr: RuntimeRead,
    pub control: Box<dyn RuntimeProcessControl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSignal {
    Interrupt,
    Terminate,
    Kill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProcessExit {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub finished_at: DateTime<Utc>,
}

#[async_trait]
pub trait RuntimeProcessControl: Send + Sync {
    async fn signal(&self, signal: RuntimeSignal) -> Result<(), RuntimeTransportError>;
    async fn wait(&self) -> Result<RuntimeProcessExit, RuntimeTransportError>;
}

/// A lexical company path. The Runtime Agent performs the authoritative
/// descriptor/canonical-path check on every filesystem operation; this type
/// prevents accidental host paths and traversal from entering the protocol.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompanyPath(String);

impl CompanyPath {
    pub fn parse(value: impl Into<String>) -> Result<Self, RuntimeTransportError> {
        let value = value.into();
        if value.len() > 4096
            || value.contains(['\0', '\r', '\n'])
            || (value != "/company" && !value.starts_with("/company/"))
            || value[1..].contains("//")
            || value.split('/').any(|part| matches!(part, "." | ".."))
        {
            return Err(RuntimeTransportError::InvalidRequest(
                "Runtime file path must remain lexically beneath /company",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFileKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFileMetadata {
    pub kind: RuntimeFileKind,
    pub size: u64,
    pub modified_at: DateTime<Utc>,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDirectoryEntry {
    pub name: String,
    pub metadata: RuntimeFileMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeService {
    Desktop,
    BrowserControl,
    ReleaseHealth,
    Published(u16),
}

impl RuntimeService {
    pub fn loopback_port(self) -> Result<u16, RuntimeTransportError> {
        match self {
            Self::Desktop => Ok(6080),
            Self::BrowserControl => Ok(9223),
            Self::ReleaseHealth => Ok(7789),
            Self::Published(port)
                if (1024..=65_535).contains(&port)
                    && !matches!(port, 5901 | 6080 | 7789 | 9222 | 9223) =>
            {
                Ok(port)
            }
            Self::Published(_) => Err(RuntimeTransportError::InvalidRequest(
                "published Runtime service port is reserved or invalid",
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeActivity {
    pub observed_at: DateTime<Utc>,
    pub active_processes: Vec<RuntimeActiveProcess>,
    pub open_service_streams: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeActiveProcess {
    pub process_id: Uuid,
    pub pid: u32,
    pub authority: RuntimeProcessAuthority,
    pub started_at: DateTime<Utc>,
}

#[async_trait]
pub trait RuntimeTransport: Send + Sync {
    async fn readiness(&self, company: &str) -> Result<RuntimeReadiness, RuntimeTransportError>;

    async fn start_process(
        &self,
        specification: RuntimeProcessSpec,
    ) -> Result<RuntimeProcess, RuntimeTransportError>;

    async fn stat(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<RuntimeFileMetadata, RuntimeTransportError>;

    async fn list(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<Vec<RuntimeDirectoryEntry>, RuntimeTransportError>;

    async fn read(
        &self,
        company: &str,
        path: &CompanyPath,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, RuntimeTransportError>;

    async fn atomic_write(
        &self,
        company: &str,
        operation_id: Uuid,
        path: &CompanyPath,
        contents: &[u8],
        mode: u32,
    ) -> Result<(), RuntimeTransportError>;

    async fn rename(
        &self,
        company: &str,
        operation_id: Uuid,
        source: &CompanyPath,
        destination: &CompanyPath,
    ) -> Result<(), RuntimeTransportError>;

    async fn digest(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<[u8; 32], RuntimeTransportError>;

    async fn open_service(
        &self,
        company: &str,
        operation_id: Uuid,
        service: RuntimeService,
        idle_timeout: Duration,
    ) -> Result<RuntimeDuplex, RuntimeTransportError>;

    async fn activity(&self, company: &str) -> Result<RuntimeActivity, RuntimeTransportError>;
}

/// Composition-time transport slot used to break the hosted startup cycle:
/// the daemon is needed to construct durable hosted authority, while product
/// work must receive one transport before any scheduler task can start. The
/// slot is installed exactly once before listeners or scheduling are exposed
/// and otherwise fails closed.
#[derive(Clone, Default)]
pub struct RuntimeTransportSlot {
    inner: Arc<std::sync::OnceLock<Arc<dyn RuntimeTransport>>>,
}

impl RuntimeTransportSlot {
    pub fn install(
        &self,
        transport: Arc<dyn RuntimeTransport>,
    ) -> Result<(), RuntimeTransportError> {
        self.inner
            .set(transport)
            .map_err(|_| RuntimeTransportError::Conflict)
    }

    fn transport(&self) -> Result<Arc<dyn RuntimeTransport>, RuntimeTransportError> {
        self.inner
            .get()
            .cloned()
            .ok_or(RuntimeTransportError::Unavailable)
    }
}

#[async_trait]
impl RuntimeTransport for RuntimeTransportSlot {
    async fn readiness(&self, company: &str) -> Result<RuntimeReadiness, RuntimeTransportError> {
        self.transport()?.readiness(company).await
    }

    async fn start_process(
        &self,
        specification: RuntimeProcessSpec,
    ) -> Result<RuntimeProcess, RuntimeTransportError> {
        self.transport()?.start_process(specification).await
    }

    async fn stat(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<RuntimeFileMetadata, RuntimeTransportError> {
        self.transport()?.stat(company, path).await
    }

    async fn list(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<Vec<RuntimeDirectoryEntry>, RuntimeTransportError> {
        self.transport()?.list(company, path).await
    }

    async fn read(
        &self,
        company: &str,
        path: &CompanyPath,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, RuntimeTransportError> {
        self.transport()?.read(company, path, maximum_bytes).await
    }

    async fn atomic_write(
        &self,
        company: &str,
        operation_id: Uuid,
        path: &CompanyPath,
        contents: &[u8],
        mode: u32,
    ) -> Result<(), RuntimeTransportError> {
        self.transport()?
            .atomic_write(company, operation_id, path, contents, mode)
            .await
    }

    async fn rename(
        &self,
        company: &str,
        operation_id: Uuid,
        source: &CompanyPath,
        destination: &CompanyPath,
    ) -> Result<(), RuntimeTransportError> {
        self.transport()?
            .rename(company, operation_id, source, destination)
            .await
    }

    async fn digest(
        &self,
        company: &str,
        path: &CompanyPath,
    ) -> Result<[u8; 32], RuntimeTransportError> {
        self.transport()?.digest(company, path).await
    }

    async fn open_service(
        &self,
        company: &str,
        operation_id: Uuid,
        service: RuntimeService,
        idle_timeout: Duration,
    ) -> Result<RuntimeDuplex, RuntimeTransportError> {
        self.transport()?
            .open_service(company, operation_id, service, idle_timeout)
            .await
    }

    async fn activity(&self, company: &str) -> Result<RuntimeActivity, RuntimeTransportError> {
        self.transport()?.activity(company).await
    }
}

pub fn validate_company(company: &str) -> Result<(), RuntimeTransportError> {
    if company.is_empty()
        || company.len() > 48
        || !company
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        || !company
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(RuntimeTransportError::InvalidRequest(
            "Runtime company identity is invalid",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn company_paths_cannot_name_the_host_or_traverse() {
        assert!(CompanyPath::parse("/company/projects/site").is_ok());
        assert!(CompanyPath::parse("/company").is_ok());
        for value in [
            "/etc/passwd",
            "/company/../etc/passwd",
            "/company/projects/./site",
            "/company/projects//site",
            "company/projects/site",
            "/company\n/etc/passwd",
        ] {
            assert!(CompanyPath::parse(value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn sensitive_environment_never_debug_prints_its_value() {
        let variable = RuntimeEnvironment::secret("RESTLESS_MODEL_TOKEN", "do-not-print");
        let debug = format!("{variable:?}");
        assert!(debug.contains("RESTLESS_MODEL_TOKEN"));
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("do-not-print"));
    }

    #[test]
    fn process_requires_exact_authority_and_a_bounded_future_deadline() {
        let now = Utc::now();
        let valid = RuntimeProcessSpec {
            operation_id: Uuid::new_v4(),
            authority: RuntimeProcessAuthority::Attempt {
                company: "c0123456789abcdef0123456789abcdef".into(),
                actor: "exec".into(),
                responsibility: "owner-direction".into(),
                work_id: Uuid::new_v4(),
                attempt_id: Uuid::new_v4(),
                session_id: "session-1".into(),
            },
            executable: "omp".into(),
            arguments: vec!["acp".into()],
            working_directory: CompanyPath::parse("/company").unwrap(),
            environment: vec![RuntimeEnvironment::secret("RESTLESS_MODEL_TOKEN", "secret")],
            deadline: now + chrono::Duration::hours(1),
        };
        assert_eq!(valid.validate(now), Ok(()));

        let mut invalid = valid.clone();
        if let RuntimeProcessAuthority::Attempt { attempt_id, .. } = &mut invalid.authority {
            *attempt_id = Uuid::nil();
        }
        assert!(invalid.validate(now).is_err());
        invalid = valid;
        invalid.deadline = now + chrono::Duration::hours(25);
        assert!(invalid.validate(now).is_err());

        let event = RuntimeProcessAuthority::AuthorityEvent {
            company: "c0123456789abcdef0123456789abcdef".into(),
            actor: "exec".into(),
            responsibility: "portfolio".into(),
            event_id: 42,
            session_id: "session-2".into(),
        };
        assert_eq!(event.validate(), Ok(()));
        let mut invalid_event = event;
        if let RuntimeProcessAuthority::AuthorityEvent { event_id, .. } = &mut invalid_event {
            *event_id = 0;
        }
        assert!(invalid_event.validate().is_err());

        let effect = RuntimeProcessAuthority::GovernedEffect {
            company: "c0123456789abcdef0123456789abcdef".into(),
            actor: "exec".into(),
            effect_class: "customer-contact.email".into(),
            authority_id: 43,
            idempotency_key: "welcome-1".into(),
            execution_no: 1,
            staging_id: Uuid::new_v4(),
            phase: RuntimeEffectPhase::Execute,
        };
        assert_eq!(effect.validate(), Ok(()));
        let mut invalid_effect = effect;
        if let RuntimeProcessAuthority::GovernedEffect { authority_id, .. } = &mut invalid_effect {
            *authority_id = 0;
        }
        assert!(invalid_effect.validate().is_err());
    }

    #[test]
    fn service_ports_are_closed_except_named_or_explicit_published_services() {
        assert_eq!(RuntimeService::Desktop.loopback_port().unwrap(), 6080);
        assert_eq!(
            RuntimeService::BrowserControl.loopback_port().unwrap(),
            9223
        );
        assert_eq!(
            RuntimeService::Published(4173).loopback_port().unwrap(),
            4173
        );
        for port in [0, 80, 5901, 6080, 7789, 9222, 9223] {
            assert!(RuntimeService::Published(port).loopback_port().is_err());
        }
    }
}
