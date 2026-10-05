//! The three Telegram decision invariants, against the real Authority store
//! and an in-process fake of the Telegram Bot API.

use super::*;
use axum::extract::{Path as AxumPath, State};
use axum::routing::post;
use axum::{Json, Router};
use std::sync::{Arc, Mutex};

const PAIRED_CHAT: i64 = 111;
const TOKEN_REFERENCE: &str = "env:TELEGRAM_FIXTURE_TOKEN";

#[derive(Clone, Default)]
struct FakeTelegram {
    calls: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    updates: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl FakeTelegram {
    async fn start(&self) -> String {
        async fn handle(
            State(fake): State<FakeTelegram>,
            AxumPath((_bot, method)): AxumPath<(String, String)>,
            Json(body): Json<serde_json::Value>,
        ) -> Json<serde_json::Value> {
            let mut calls = fake.calls.lock().unwrap();
            calls.push((method.clone(), body));
            let result = match method.as_str() {
                "getUpdates" => {
                    serde_json::Value::Array(std::mem::take(&mut *fake.updates.lock().unwrap()))
                }
                "sendMessage" => serde_json::json!({"message_id": calls.len()}),
                _ => serde_json::json!(true),
            };
            Json(serde_json::json!({"ok": true, "result": result}))
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route("/{bot}/{method}", post(handle))
            .with_state(self.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn push(&self, update: serde_json::Value) {
        self.updates.lock().unwrap().push(update);
    }

    fn calls(&self, method: &str) -> Vec<serde_json::Value> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(name, _)| name == method)
            .map(|(_, body)| body.clone())
            .collect()
    }
}

struct LocalOwner;

#[async_trait::async_trait]
impl Companies for LocalOwner {
    async fn authority_owner(&self, _company: &str) -> Result<String> {
        Ok("owner".into())
    }
    async fn orgintel(&self, _company: &str) -> Option<restless_orgintel::OrgIntel> {
        None
    }
}

struct Fixture {
    authority: AuthorityStore,
    root: std::path::PathBuf,
    company: String,
    fake: FakeTelegram,
    bot: Bot,
    offset: i64,
    /// Approve and decline callback data for the first-contact items, by party.
    buttons: Vec<(String, String, String)>,
}

impl Fixture {
    /// A paired chat that has been sent two first-contact approvals.
    async fn new() -> Option<Self> {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Telegram decision scenario");
            return None;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let company = format!("tg_{}_test", &Uuid::new_v4().simple().to_string()[..12]);
        let root = std::env::temp_dir().join(format!("restless-telegram-{company}"));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        std::fs::write(
            root.join("companies").join(format!("{company}.toml")),
            format!("name = \"{company}\"\nmission = \"Telegram fixture\"\n"),
        )
        .unwrap();
        let fake = FakeTelegram::default();
        let bot = Bot::with_base(&fake.start().await, "123456:fixture-token").unwrap();
        let mut fixture = Self {
            authority,
            root,
            company,
            fake,
            bot,
            offset: 0,
            buttons: Vec::new(),
        };

        let link = start_pairing(
            fixture.authority.pool(),
            &fixture.company,
            TOKEN_REFERENCE,
            "fixture_bot",
            "http://127.0.0.1:7788",
            "owner",
        )
        .await
        .unwrap();
        let code = link.pairing_code.unwrap();
        fixture.fake.push(serde_json::json!({
            "update_id": 1,
            "message": {"message_id": 1, "chat": {"id": PAIRED_CHAT, "type": "private"},
                        "from": {"id": PAIRED_CHAT}, "text": format!("/start {code}")},
        }));
        fixture.poll().await;
        let link = super::link(fixture.authority.pool(), &fixture.company)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(link.chat_id, Some(PAIRED_CHAT));

        let mut outgoing = Vec::new();
        for party in ["alice@example.test", "bob@example.test"] {
            let record = fixture
                .authority
                .emit(
                    &fixture.company,
                    "approval_required",
                    Some("exec"),
                    serde_json::json!({"party": party, "effect_class": "email",
                                       "prepared_command": ["send", party]}),
                )
                .await
                .unwrap();
            outgoing.push(Outgoing {
                key: format!("key-{party}"),
                item_id: format!("authority:approval:email:{party}"),
                text: format!("First contact: {party}"),
                binding: Some(Binding::Party {
                    record,
                    party: party.into(),
                }),
            });
        }
        assert_eq!(
            deliver(
                fixture.authority.pool(),
                &fixture.bot,
                &link,
                outgoing.clone()
            )
            .await
            .unwrap(),
            2
        );
        // A second pass is the crash-restart case: nothing goes out twice.
        assert_eq!(
            deliver(fixture.authority.pool(), &fixture.bot, &link, outgoing)
                .await
                .unwrap(),
            0
        );
        for message in fixture.fake.calls("sendMessage") {
            let Some(row) = message["reply_markup"]["inline_keyboard"][0].as_array() else {
                continue;
            };
            let party = message["text"]
                .as_str()
                .unwrap()
                .trim_start_matches("First contact: ");
            fixture.buttons.push((
                party.to_string(),
                row[0]["callback_data"].as_str().unwrap().to_string(),
                row[1]["callback_data"].as_str().unwrap().to_string(),
            ));
        }
        assert_eq!(fixture.buttons.len(), 2);
        Some(fixture)
    }

    fn button(&self, party: &str, approve: bool) -> String {
        let (_, yes, no) = self.buttons.iter().find(|(p, _, _)| p == party).unwrap();
        if approve {
            yes.clone()
        } else {
            no.clone()
        }
    }

    async fn poll(&mut self) {
        poll_once(
            &self.authority,
            &self.root,
            &LocalOwner,
            &self.bot,
            TOKEN_REFERENCE,
            &mut self.offset,
            0,
        )
        .await
        .unwrap();
    }

    /// Press a button as `chat` and return the bot's callback answer.
    async fn press(&mut self, update_id: i64, chat: i64, data: &str) -> String {
        self.fake.push(serde_json::json!({
            "update_id": update_id,
            "callback_query": {"id": format!("cb-{update_id}"), "from": {"id": chat}, "data": data,
                               "message": {"message_id": 9, "chat": {"id": chat}}},
        }));
        self.poll().await;
        let answers = self.fake.calls("answerCallbackQuery");
        answers.last().unwrap()["text"]
            .as_str()
            .unwrap()
            .to_string()
    }

    async fn decisions(&self, kind: &str, party: &str) -> usize {
        self.authority
            .records_of_kind(&self.company, kind)
            .await
            .unwrap()
            .iter()
            .filter(|record| record.body["party"] == party)
            .count()
    }

    async fn cleanup(self) {
        let pool = self.authority.pool();
        sqlx::query("DELETE FROM restless_authority.records WHERE company=$1")
            .bind(&self.company)
            .execute(pool)
            .await
            .unwrap();
        unpair(pool, &self.company).await.unwrap();
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[tokio::test]
async fn only_the_paired_chat_can_decide() {
    let Some(mut fixture) = Fixture::new().await else {
        return;
    };
    let approve = fixture.button("alice@example.test", true);
    let answer = fixture.press(10, 222, &approve).await;
    assert_eq!(answer, Answer::NotPaired.text());
    assert_eq!(
        fixture
            .decisions("approval_granted", "alice@example.test")
            .await,
        0
    );
    // The refused press did not use up the button for the paired chat.
    let answer = fixture.press(11, PAIRED_CHAT, &approve).await;
    assert!(
        answer.starts_with("Alice@example.test approved"),
        "{answer}"
    );
    assert_eq!(
        fixture
            .decisions("approval_granted", "alice@example.test")
            .await,
        1
    );
    fixture.cleanup().await;
}

#[tokio::test]
async fn a_button_decides_only_its_own_item() {
    let Some(mut fixture) = Fixture::new().await else {
        return;
    };
    let approve_alice = fixture.button("alice@example.test", true);
    fixture.press(10, PAIRED_CHAT, &approve_alice).await;
    assert_eq!(
        fixture
            .decisions("approval_granted", "alice@example.test")
            .await,
        1
    );
    assert_eq!(
        fixture
            .decisions("approval_granted", "bob@example.test")
            .await,
        0
    );
    assert_eq!(
        fixture
            .decisions("approval_declined", "bob@example.test")
            .await,
        0
    );
    assert!(
        !crate::approval::approved_parties(&fixture.authority, &fixture.company)
            .await
            .unwrap()
            .contains(&"bob@example.test".to_string())
    );
    fixture.cleanup().await;
}

#[tokio::test]
async fn a_replayed_callback_does_not_decide_twice() {
    let Some(mut fixture) = Fixture::new().await else {
        return;
    };
    // Decline appends a decision every time it runs; the delivery claim (and,
    // for a sequential replay, the open-question check) must stop the second.
    let decline = fixture.button("bob@example.test", false);
    let first = fixture.press(10, PAIRED_CHAT, &decline).await;
    assert!(
        first.starts_with("First contact with bob@example.test declined"),
        "{first}"
    );
    let replay = fixture.press(10, PAIRED_CHAT, &decline).await;
    assert_eq!(replay, Answer::AlreadyDecided.text());
    // The other button on the same message is just as spent.
    let approve = fixture.button("bob@example.test", true);
    let late = fixture.press(11, PAIRED_CHAT, &approve).await;
    assert_eq!(late, Answer::AlreadyDecided.text());
    assert_eq!(
        fixture
            .decisions("approval_declined", "bob@example.test")
            .await,
        1
    );
    assert_eq!(
        fixture
            .decisions("approval_granted", "bob@example.test")
            .await,
        0
    );
    fixture.cleanup().await;
}
