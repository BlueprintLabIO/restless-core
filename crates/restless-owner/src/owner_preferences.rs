//! Preferences follow the person (S109 F2): sidebar and pane widths, the Work view, the Inbox
//! filter and folded sidebar sections, the same in every company and on every device.
//!
//! A small file per person beside the plane's sessions, keyed by who asks
//! (`RequestPrincipal::person_key`), never by company: the owner's account session and each of a
//! member's company sessions on this plane name the same person. Only known keys are kept, and
//! the whole set is capped, so this stays a handful of layout choices and never becomes storage.

use super::*;
use std::sync::LazyLock;

pub(super) const PREFERENCES_PATH: &str = "/api/preferences";

/// The whole set, as stored. A few dozen layout choices fit many times over.
const MAX_BYTES: usize = 16 * 1024;

/// The choices the web app keeps; kept in step with `web/src/lib/model/preferences.ts`.
const KNOWN_KEYS: &[&str] = &[
    "pane.sidebar.work",
    "pane.sidebar.apps",
    "pane.sidebar.library",
    "pane.sidebar.company",
    "pane.sidebar.people",
    "pane.exec",
    "pane.work-detail",
    "pane.attention-document",
    "pane.handover",
    "pane.inbox",
    "account-rail",
    "work.view",
    "work.sort",
    "inbox.filter",
    "folds",
    // Voice (S107 D): speech to text and text to speech follow the person, not a company.
    "voice.stt",
    "voice.tts",
];

type Preferences = serde_json::Map<String, serde_json::Value>;

/// One writer at a time, so two tabs' changes to different keys both land.
static WRITES: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreferencesPatch {
    /// The keys to change; `null` forgets a key, so the app's default returns.
    preferences: Preferences,
}

#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    UnknownKey(String),
    TooLarge,
}

fn file(root: &std::path::Path, person: &str) -> PathBuf {
    root.join("preferences").join(format!("{person}.json"))
}

fn read(root: &std::path::Path, person: &str) -> Result<Preferences> {
    match std::fs::read(file(root, person)) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::new()),
        Err(error) => Err(error).context("read preferences"),
    }
}

/// Merge a patch into the person's preferences and keep the result.
fn apply(
    root: &std::path::Path,
    person: &str,
    patch: Preferences,
) -> Result<std::result::Result<Preferences, Refusal>> {
    if let Some(key) = patch.keys().find(|key| !KNOWN_KEYS.contains(&key.as_str())) {
        return Ok(Err(Refusal::UnknownKey(key.clone())));
    }
    let _writing = WRITES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut preferences = read(root, person)?;
    for (key, value) in patch {
        if value.is_null() {
            preferences.remove(&key);
        } else {
            preferences.insert(key, value);
        }
    }
    let body = serde_json::to_vec(&preferences).context("encode preferences")?;
    if body.len() > MAX_BYTES {
        return Ok(Err(Refusal::TooLarge));
    }
    let path = file(root, person);
    let directory = path.parent().expect("preferences directory");
    std::fs::create_dir_all(directory).context("create preferences directory")?;
    let staged = path.with_extension("tmp");
    std::fs::write(&staged, &body).context("write preferences")?;
    std::fs::rename(&staged, &path).context("keep preferences")?;
    Ok(Ok(preferences))
}

fn answer(preferences: Preferences) -> Response<Body> {
    Json(serde_json::json!({ "preferences": preferences })).into_response()
}

fn unavailable(error: anyhow::Error) -> Response<Body> {
    tracing::error!(error = %format!("{error:#}"), "preferences unavailable");
    api_error(
        StatusCode::SERVICE_UNAVAILABLE,
        "preferences",
        "Your layout choices could not be read or kept just now.",
    )
}

fn get_at(root: &std::path::Path, principal: &RequestPrincipal) -> Response<Body> {
    match read(root, principal.person_key()) {
        Ok(preferences) => answer(preferences),
        Err(error) => unavailable(error),
    }
}

fn put_at(
    root: &std::path::Path,
    principal: &RequestPrincipal,
    patch: PreferencesPatch,
) -> Response<Body> {
    match apply(root, principal.person_key(), patch.preferences) {
        Ok(Ok(preferences)) => answer(preferences),
        Ok(Err(Refusal::UnknownKey(key))) => api_error(
            StatusCode::BAD_REQUEST,
            "preferences",
            format!("{key} is not a preference this plane keeps"),
        ),
        Ok(Err(Refusal::TooLarge)) => api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "preferences",
            "These preferences are larger than the plane keeps.",
        ),
        Err(error) => unavailable(error),
    }
}

/// The signed-in person's preferences.
pub(super) async fn get_preferences(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
) -> Response<Body> {
    get_at(&state.daemon.root, &principal)
}

/// Change some of the signed-in person's preferences; answers with all of them.
pub(super) async fn put_preferences(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    Json(patch): Json<PreferencesPatch>,
) -> Response<Body> {
    let root = state.daemon.root.clone();
    tokio::task::spawn_blocking(move || put_at(&root, &principal, patch))
        .await
        .unwrap_or_else(|error| unavailable(anyhow::anyhow!("preferences write failed: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("preferences-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn member(company: &str, actor: &str) -> RequestPrincipal {
        RequestPrincipal::from_verified(&VerifiedIdentity {
            user: "user-1".into(),
            issuer: Some("https://cloud.restless.test".into()),
            owner: "owner-1".into(),
            scope: CompanyScope::Company {
                company: company.into(),
            },
            role: "member".into(),
            actor: Some(actor.into()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some(format!("membership-{company}")),
            membership_version: Some(1),
            display_name: None,
        })
        .unwrap()
    }

    fn patch(value: serde_json::Value) -> PreferencesPatch {
        serde_json::from_value(serde_json::json!({ "preferences": value })).unwrap()
    }

    async fn body(response: Response<Body>) -> (StatusCode, serde_json::Value) {
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn a_member_keeps_one_set_of_preferences_across_companies() {
        let root = root();
        let (status, saved) = body(put_at(
            &root,
            &member("aris", "human-1"),
            patch(serde_json::json!({ "pane.sidebar.work": 312, "work.view": "list" })),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(saved["preferences"]["pane.sidebar.work"], 312);

        // Another company, another actor and membership: the same person.
        let (status, read) = body(get_at(&root, &member("reno", "human-7"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            read["preferences"],
            serde_json::json!({ "pane.sidebar.work": 312, "work.view": "list" })
        );

        // The owner is someone else and starts from nothing.
        let (_, owner) = body(get_at(&root, &RequestPrincipal::local_owner())).await;
        assert_eq!(owner["preferences"], serde_json::json!({}));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn a_patch_merges_and_null_forgets() {
        let root = root();
        let owner = RequestPrincipal::local_owner();
        put_at(
            &root,
            &owner,
            patch(serde_json::json!({ "pane.exec": 420, "folds": { "Teams": true } })),
        );
        let (_, kept) = body(put_at(
            &root,
            &owner,
            patch(serde_json::json!({ "pane.exec": null, "inbox.filter": "all" })),
        ))
        .await;
        assert_eq!(
            kept["preferences"],
            serde_json::json!({ "folds": { "Teams": true }, "inbox.filter": "all" })
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn unknown_keys_and_oversized_sets_are_refused_and_nothing_changes() {
        let root = root();
        let owner = RequestPrincipal::local_owner();
        put_at(
            &root,
            &owner,
            patch(serde_json::json!({ "work.view": "board" })),
        );

        let (status, _) = body(put_at(
            &root,
            &owner,
            patch(serde_json::json!({ "work.view": "list", "anything": 1 })),
        ))
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let folds: serde_json::Map<_, _> = (0..2000)
            .map(|index| (format!("Section {index}"), serde_json::json!(true)))
            .collect();
        let (status, _) = body(put_at(
            &root,
            &owner,
            patch(serde_json::json!({ "folds": folds })),
        ))
        .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);

        let (_, read) = body(get_at(&root, &owner)).await;
        assert_eq!(
            read["preferences"],
            serde_json::json!({ "work.view": "board" })
        );
        assert!(
            serde_json::from_value::<PreferencesPatch>(serde_json::json!({ "other": {} })).is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// S107 D3: voice is the person's own. The owner turning on read-aloud leaves a member on the
    /// browser's defaults.
    #[tokio::test]
    async fn voice_choices_are_kept_per_person() {
        let root = root();
        let owner = RequestPrincipal::local_owner();
        let (status, kept) = body(put_at(
            &root,
            &owner,
            patch(serde_json::json!({ "voice.tts": "browser", "voice.stt": "browser" })),
        ))
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(kept["preferences"]["voice.tts"], "browser");
        let (_, member) = body(get_at(&root, &member("aris", "human-1"))).await;
        assert_eq!(member["preferences"], serde_json::json!({}));
        let (_, off) = body(put_at(
            &root,
            &owner,
            patch(serde_json::json!({ "voice.tts": null })),
        ))
        .await;
        assert_eq!(
            off["preferences"],
            serde_json::json!({ "voice.stt": "browser" })
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn every_person_may_keep_their_own_preferences() {
        let member = member("aris", "human-1");
        for method in [Method::GET, Method::PUT] {
            assert!(membership_boundary_violation(&method, PREFERENCES_PATH, &member).is_none());
        }
    }
}
