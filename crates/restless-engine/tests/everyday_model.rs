//! The everyday model (S102, S107) as the engine routes a turn: an Opus company's Staff turn starts
//! on its Sonnet sibling with Opus behind it, unless the owner chose otherwise, switched it off,
//! gave the agent its own model on People, or the sibling is cooling down. Needs
//! `RESTLESS_TEST_DATABASE_URL` for the Authority store's cooldowns; skipped otherwise.

use restless_engine::authority::AuthorityStore;
use restless_engine::model_gateway::with_everyday_model;
use restless_engine::runtime::{AgentHarness, AgentIntelligence, CompanyConfig};

fn company(name: &str, extra: &str) -> CompanyConfig {
    toml::from_str(&format!(
        "name = \"{name}\"\nmodel = \"anthropic/claude-opus-5-5\"\n{extra}"
    ))
    .expect("company config parses")
}

fn primary() -> Vec<String> {
    vec!["anthropic/claude-opus-5-5".to_string()]
}

#[tokio::test]
async fn staff_turns_start_on_the_everyday_model_and_owner_choices_win() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("database unset; skipping everyday model routing");
        return;
    };
    let authority = AuthorityStore::connect(&url).await.unwrap();
    let harness = AgentHarness::RestlessManaged;
    let name = format!("every{}", uuid::Uuid::new_v4().simple());

    // Unset: the Sonnet sibling leads, Opus stays behind it as failover.
    let automatic = company(&name, "");
    assert_eq!(
        with_everyday_model(&automatic, "maya", harness, primary(), &authority)
            .await
            .unwrap(),
        ["anthropic/claude-sonnet-5-5", "anthropic/claude-opus-5-5"]
    );

    // The owner's own choice.
    let chosen = company(&name, "everyday_model = \"anthropic/claude-haiku-5-5\"\n");
    assert_eq!(
        with_everyday_model(&chosen, "maya", harness, primary(), &authority)
            .await
            .unwrap()[0],
        "anthropic/claude-haiku-5-5"
    );

    // "Same as main": every turn on Opus.
    let off = company(&name, "everyday_model = \"off\"\n");
    assert_eq!(
        with_everyday_model(&off, "maya", harness, primary(), &authority)
            .await
            .unwrap(),
        primary()
    );

    // An agent the owner gave its own model on People keeps exactly that model.
    let mut overridden = company(&name, "");
    overridden.agent_intelligence.insert(
        "maya".into(),
        AgentIntelligence {
            connection: "direct:anthropic".into(),
            model: "claude-opus-5-5".into(),
        },
    );
    let maya = overridden.for_agent("maya");
    assert_eq!(
        with_everyday_model(&maya, "maya", harness, primary(), &authority)
            .await
            .unwrap(),
        primary()
    );
    // Robin has no override and still gets the everyday model in the same company.
    let robin = overridden.for_agent("robin");
    assert_eq!(
        with_everyday_model(&robin, "robin", harness, primary(), &authority)
            .await
            .unwrap()[0],
        "anthropic/claude-sonnet-5-5"
    );

    // A sibling the provider refused cools down, and the primary takes the turn.
    authority
        .set_model_cooldown(
            &name,
            "anthropic/claude-sonnet-5-5",
            "refused",
            "model not found",
            chrono::Utc::now() + chrono::Duration::minutes(10),
        )
        .await
        .unwrap();
    assert_eq!(
        with_everyday_model(&automatic, "maya", harness, primary(), &authority)
            .await
            .unwrap(),
        primary()
    );
    authority
        .clear_model_cooldown(&name, "anthropic/claude-sonnet-5-5")
        .await
        .unwrap();

    // A primary from another provider than the turn's first candidate keeps its order: the
    // automatic sibling never jumps connections.
    assert_eq!(
        with_everyday_model(
            &automatic,
            "maya",
            harness,
            vec!["openai-codex/gpt-6-sol".to_string()],
            &authority
        )
        .await
        .unwrap(),
        ["openai-codex/gpt-6-sol"]
    );

    // Claude on the owner's own sign-in (a native harness): the everyday model runs on that same
    // connection, with the connection's model behind it. It used to be skipped outright.
    let native = company(
        &name,
        "worker_harness = \"claude-agent\"\n[native_harnesses.claude-agent]\nmode = \"oauth\"\nmodel = \"claude-opus-5-5\"\n",
    );
    let claude = AgentHarness::ClaudeAgent;
    let native_primary = vec!["native-claude-oauth/claude-opus-5-5".to_string()];
    assert_eq!(
        with_everyday_model(&native, "maya", claude, native_primary.clone(), &authority)
            .await
            .unwrap(),
        [
            "native-claude-oauth/claude-sonnet-5-5",
            "native-claude-oauth/claude-opus-5-5"
        ]
    );
}

#[test]
fn each_provider_has_its_cheaper_sibling() {
    use restless_engine::runtime::everyday_sibling;
    assert_eq!(
        everyday_sibling("anthropic/claude-opus-5-5").as_deref(),
        Some("anthropic/claude-sonnet-5-5")
    );
    assert_eq!(
        everyday_sibling("openai-codex/gpt-6-sol").as_deref(),
        Some("openai-codex/gpt-6-terra")
    );
    assert_eq!(
        everyday_sibling("google/gemini-3-pro").as_deref(),
        Some("google/gemini-3-flash")
    );
    for none in [
        "anthropic/claude-sonnet-5-5",
        "openai-codex/gpt-6-nano",
        "deepseek/deepseek-v4.1-flash",
        "native-claude-oauth/opus",
    ] {
        assert_eq!(everyday_sibling(none), None, "{none}");
    }
}
