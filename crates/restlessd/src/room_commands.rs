//! The installed harness uses the same actor-scoped Room operations as the UI.
use restless_orgintel::{NewRoomMessageMention, OrgIntel, RoomKind};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RoomOperation {
    List {
        #[serde(default)]
        before_created_at: Option<chrono::DateTime<chrono::Utc>>,
        #[serde(default)]
        before_room_id: Option<Uuid>,
    },
    Create {
        title: String,
        kind: RoomKind,
        participants: Vec<String>,
        key: String,
    },
    Members {
        room: Uuid,
    },
    Add {
        room: Uuid,
        member: String,
    },
    Remove {
        room: Uuid,
        member: String,
    },
    Read {
        room: Uuid,
        #[serde(default)]
        before: Option<i64>,
    },
    Thread {
        room: Uuid,
        message: i64,
        #[serde(default)]
        before: Option<i64>,
    },
    Send {
        room: Uuid,
        body: String,
        #[serde(default)]
        parent: Option<i64>,
        key: String,
        #[serde(default)]
        mentions: Vec<NewRoomMessageMention>,
    },
}

pub(crate) async fn execute(
    org: &OrgIntel,
    actor: &str,
    operation: RoomOperation,
) -> anyhow::Result<Value> {
    Ok(match operation {
        RoomOperation::List {
            before_created_at,
            before_room_id,
        } => {
            let before = match (before_created_at, before_room_id) {
                (None, None) => None,
                (Some(time), Some(id)) => Some((time, id)),
                _ => anyhow::bail!(
                    "Room pagination requires both before_created_at and before_room_id"
                ),
            };
            json!(org.room_page_for_actor(actor, before, 50).await?)
        }
        RoomOperation::Create {
            title,
            kind,
            participants,
            key,
        } => {
            if kind == RoomKind::Company && !matches!(actor, "exec" | "owner") {
                anyhow::bail!("Only the owner or Exec may establish the company Room");
            }
            let people: Vec<_> = participants.iter().map(String::as_str).collect();
            json!(org.create_room(actor, kind, &title, &people, &key).await?)
        }
        RoomOperation::Members { room } => {
            json!({"participants":org.room_participants(actor, room).await?})
        }
        RoomOperation::Add { room, member } => {
            json!(org.add_room_participant(actor, room, &member).await?)
        }
        RoomOperation::Remove { room, member } => {
            json!(org.remove_room_participant(actor, room, &member).await?)
        }
        RoomOperation::Read { room, before } => {
            json!(org.room_messages_before(actor, room, before, 50).await?)
        }
        RoomOperation::Thread {
            room,
            message,
            before,
        } => json!(
            org.room_thread_before(actor, room, message, before, 50)
                .await?
        ),
        RoomOperation::Send {
            room,
            body,
            parent,
            key,
            mentions,
        } => json!(
            org.send_room_message_with_mentions(
                room, actor, &body, parent, &key, None, &mentions, None
            )
            .await?
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn room_commands_preserve_membership_threads_and_retry_identity() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let schema = format!("roomtools{}_test", Uuid::new_v4().simple());
        let org = OrgIntel::ensure(&url, &schema).await.unwrap();
        let result = async {
            org.ensure_actor("exec", "exec", "exec", "Exec").await?;
            org.ensure_actor("owner", "owner", "owner", "Owner").await?;
            org.ensure_actor("evidence-writer", "staff", "writer", "Writer")
                .await?;
            let create = || RoomOperation::Create {
                title: "Shared planning".into(),
                kind: RoomKind::Group,
                participants: vec!["owner".into()],
                key: "create-room".into(),
            };
            let first = execute(&org, "exec", create()).await?;
            let second = execute(&org, "exec", create()).await?;
            anyhow::ensure!(first["id"] == second["id"], "retry created another Room");
            let room = Uuid::parse_str(first["id"].as_str().unwrap())?;
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-writer",
                    RoomOperation::Read { room, before: None }
                )
                .await
                .is_err(),
                "nonmember read Room"
            );
            execute(
                &org,
                "exec",
                RoomOperation::Add {
                    room,
                    member: "evidence-writer".into(),
                },
            )
            .await?;
            let send = |parent| RoomOperation::Send {
                room,
                body: "A bounded contribution".into(),
                parent,
                key: if parent.is_some() { "reply" } else { "root" }.into(),
                mentions: vec![],
            };
            let root = execute(&org, "evidence-writer", send(None)).await?;
            let retry = execute(&org, "evidence-writer", send(None)).await?;
            anyhow::ensure!(
                root["message"]["id"] == retry["message"]["id"],
                "retry duplicated Message"
            );
            let message = root["message"]["id"].as_i64().unwrap();
            execute(&org, "owner", send(Some(message))).await?;
            let thread = execute(
                &org,
                "exec",
                RoomOperation::Thread {
                    room,
                    message,
                    before: None,
                },
            )
            .await?;
            anyhow::ensure!(
                thread["messages"].as_array().unwrap().len() == 2,
                "reply left its Thread"
            );
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-writer",
                    RoomOperation::Remove {
                        room,
                        member: "owner".into()
                    }
                )
                .await
                .is_err(),
                "member managed membership"
            );
            execute(
                &org,
                "exec",
                RoomOperation::Remove {
                    room,
                    member: "evidence-writer".into(),
                },
            )
            .await?;
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-writer",
                    RoomOperation::Send {
                        room,
                        body: "No access".into(),
                        parent: None,
                        key: "after-revoke".into(),
                        mentions: vec![]
                    }
                )
                .await
                .is_err(),
                "removed member retained access"
            );
            Ok::<(), anyhow::Error>(())
        }
        .await;
        org.drop_schema().await.unwrap();
        result.unwrap();
    }

    #[test]
    fn actor_identity_cannot_be_smuggled_inside_an_operation() {
        assert!(serde_json::from_value::<RoomOperation>(
            json!({"operation":"members","room":Uuid::new_v4(),"actor":"owner"})
        )
        .is_err());
    }
}
