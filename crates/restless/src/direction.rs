//! CLI serialization only; Core admits Exec alone, routes each change and records it.
use anyhow::{Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};

const CLASSES: [&str; 4] = ["content", "work", "staffing", "schedule"];

#[derive(Subcommand)]
pub(crate) enum DirectionCommand {
    /// Which path a person's change takes: direct (they own it), act_then_tell (apply it, then
    /// record it with `restless direction applied`) or ask_first (`restless request file`).
    Path {
        /// The person who asked, by Actor id.
        #[arg(long)]
        requester: String,
        #[arg(long, value_parser = CLASSES)]
        class: Option<String>,
        /// The team the change belongs to, by id or name.
        #[arg(long)]
        team: Option<String>,
    },
    /// Record a change you applied on someone's direction, so whoever owns it is told and can undo it.
    #[command(after_long_help = r#"UNDO HANDLES (--undo JSON)
  Team brief:   {"kind":"content","before":{"team_id":ID,"brief":OLD},"after":{"brief":NEW}}
  Document:     {"kind":"content","before":{"document_id":ID,"version_id":OLD},"after":{"version_id":NEW}}
  Agent model:  {"kind":"staffing","actor_id":ID,"before":{"model":OLD},"after":{"model":NEW}}
  New schedule: {"kind":"schedule","schedule_id":ID,"before":null}
  Work owner:   {"kind":"work","work_id":ID,"before":{"owner_id":OLD},"after":{"owner_id":NEW}}
  Work goal:    {"kind":"work","work_id":ID,"before":{"goal_id":OLD},"after":{"goal_id":NEW}}

TARGETS (--target JSON)
  {"kind":"document","id":ID,"title":T}    {"kind":"team_brief","team":{"id":ID,"name":N}}
  {"kind":"work","id":ID,"title":T}        {"kind":"schedule","id":ID,"title":T}
  {"kind":"agent","actor_id":ID,"name":N}  {"kind":"charter"}"#)]
    Applied {
        #[arg(long)]
        requester: String,
        #[arg(long, value_parser = CLASSES)]
        class: String,
        #[arg(long)]
        team: Option<String>,
        /// One line in plain words: what changed.
        #[arg(long)]
        summary: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        undo: String,
        /// The message the person asked in, by id.
        #[arg(long)]
        source_message: Option<String>,
        /// The approved request this carries out.
        #[arg(long)]
        request: Option<String>,
    },
    /// Report an approved request carried out, or why it could not be.
    Done {
        #[arg(long)]
        request: String,
        #[arg(long, conflicts_with = "failure")]
        target: Option<String>,
        #[arg(long)]
        failure: Option<String>,
    },
    /// The standing permissions in force: who may change what without asking.
    Grants,
}

#[derive(Subcommand)]
pub(crate) enum RequestCommand {
    /// Ask whoever owns the direction to approve a change; nothing changes until they answer.
    File {
        #[arg(long)]
        requester: String,
        #[arg(long, value_parser = CLASSES)]
        class: Option<String>,
        #[arg(long)]
        team: Option<String>,
        /// One line: what the person wants.
        #[arg(long)]
        summary: String,
        /// The exact change, so the approver knows what they approve.
        #[arg(long)]
        change: String,
        /// Why, in the person's words.
        #[arg(long, default_value = "")]
        reason: String,
        #[arg(long)]
        source_message: Option<String>,
    },
    /// A request and its answer.
    Show { id: String },
}

#[derive(Subcommand)]
pub(crate) enum CharterCommand {
    /// The charter as it stands, its revision, and any draft waiting for the owner.
    Show,
    /// Save the charter because the owner told you what it should say. Refused for anyone else.
    Apply {
        /// The owner, by Actor id.
        #[arg(long)]
        requester: String,
        /// A Markdown file holding the whole new charter.
        #[arg(long)]
        file: std::path::PathBuf,
        /// The owner's message, by id.
        #[arg(long)]
        source_message: Option<String>,
    },
    /// Draft a charter change into Company › Charter for the owner to publish, edit or discard.
    Propose {
        /// Who asked for it, by Actor id (your own id when it is your proposal).
        #[arg(long)]
        requester: String,
        /// A Markdown file holding the whole proposed charter.
        #[arg(long)]
        file: std::path::PathBuf,
        /// Why, in one or two plain sentences; the owner reads it beside the draft.
        #[arg(long, default_value = "")]
        reason: String,
        #[arg(long)]
        source_message: Option<String>,
    },
}

impl CharterCommand {
    pub(crate) fn request(self, company: String) -> Result<Value> {
        let read = |file: &std::path::Path| {
            std::fs::read_to_string(file).with_context(|| format!("read {}", file.display()))
        };
        let operation = match self {
            Self::Show => json!({"operation":"charter_show"}),
            Self::Apply {
                requester,
                file,
                source_message,
            } => json!({
                "operation":"charter_apply","requester":requester,"markdown":read(&file)?,
                "source_message":source_message,
            }),
            Self::Propose {
                requester,
                file,
                reason,
                source_message,
            } => json!({
                "operation":"charter_propose","requester":requester,"markdown":read(&file)?,
                "reason":reason,"source_message":source_message,
            }),
        };
        Ok(envelope(company, operation))
    }
}

fn parse(field: &str, raw: &str) -> Result<Value> {
    serde_json::from_str(raw).with_context(|| format!("--{field} must be JSON"))
}

fn envelope(company: String, operation: Value) -> Value {
    json!({"cmd":"direction-operation","company":company,"direction_operation":operation})
}

impl DirectionCommand {
    pub(crate) fn request(self, company: String) -> Result<Value> {
        let operation = match self {
            Self::Path {
                requester,
                class,
                team,
            } => {
                json!({"operation":"path","requester":requester,"class":class,"team":team})
            }
            Self::Applied {
                requester,
                class,
                team,
                summary,
                target,
                undo,
                source_message,
                request,
            } => json!({
                "operation":"applied","requester":requester,"class":class,"team":team,
                "summary":summary,"target":parse("target", &target)?,"undo":parse("undo", &undo)?,
                "source_message":source_message,"request":request,
            }),
            Self::Done {
                request,
                target,
                failure,
            } => json!({
                "operation":"request_done","id":request,
                "target":target.map(|target| parse("target", &target)).transpose()?,"failure":failure,
            }),
            Self::Grants => json!({"operation":"grants"}),
        };
        Ok(envelope(company, operation))
    }
}

impl RequestCommand {
    pub(crate) fn request(self, company: String) -> Result<Value> {
        let operation = match self {
            Self::File {
                requester,
                class,
                team,
                summary,
                change,
                reason,
                source_message,
            } => json!({
                "operation":"request_file","requester":requester,"class":class,"team":team,
                "summary":summary,"change":change,"reason":reason,"source_message":source_message,
            }),
            Self::Show { id } => json!({"operation":"request_show","id":id}),
        };
        Ok(envelope(company, operation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applied(target: &str) -> DirectionCommand {
        DirectionCommand::Applied {
            requester: "sam".into(),
            class: "content".into(),
            team: Some("Design".into()),
            summary: "Tightened the hero copy".into(),
            target: target.into(),
            undo: r#"{"kind":"content","before":{}}"#.into(),
            source_message: None,
            request: None,
        }
    }

    #[test]
    fn applied_carries_its_target_and_undo_as_json() {
        let value = applied(r#"{"kind":"charter"}"#)
            .request("acme_test".into())
            .unwrap();
        assert_eq!(value["cmd"], "direction-operation");
        assert_eq!(value["direction_operation"]["operation"], "applied");
        assert_eq!(value["direction_operation"]["target"]["kind"], "charter");
        assert!(applied("charter").request("acme_test".into()).is_err());
    }
}
