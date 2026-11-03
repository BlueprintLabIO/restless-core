//! What a new company starts on (S107 B1): the owner's default model, as one of their account
//! connections and a model it serves, and their default decision model. One small file per plane,
//! which is the owner's account. A new company inherits them when it is created; changing a
//! default never changes a company that already exists.

use super::*;

const FILE: &str = "new-company-defaults.json";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DefaultModel {
    /// An owner account connection id (Settings › Accounts).
    pub connection: String,
    /// provider/model, of that connection's provider.
    pub model: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NewCompanyDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<DefaultModel>,
    /// `included`, an OpenRouter provider/model, or none (the included model, watching only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_model: Option<String>,
}

fn file(root: &std::path::Path) -> PathBuf {
    root.join(FILE)
}

pub(super) fn load(root: &std::path::Path) -> Result<NewCompanyDefaults> {
    match std::fs::read(file(root)) {
        Ok(bytes) => serde_json::from_slice(&bytes).context("parse new company defaults"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
        Err(error) => Err(error.into()),
    }
}

fn save(root: &std::path::Path, defaults: &NewCompanyDefaults) -> Result<()> {
    let temporary = root.join(format!(".{FILE}.{}.tmp", Uuid::new_v4()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(defaults)?)?;
    std::fs::rename(&temporary, file(root)).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })?;
    Ok(())
}

/// Normalise and check a choice against the owner's connections: the model must belong to the
/// connection's provider, and the decision model must be one the company setting accepts.
fn validate(
    root: &std::path::Path,
    mut defaults: NewCompanyDefaults,
) -> std::result::Result<NewCompanyDefaults, &'static str> {
    if let Some(model) = defaults.model.as_mut() {
        model.connection = model.connection.trim().to_string();
        model.model = model.model.trim().to_string();
        let registry = load_owner_connections(root).map_err(|_| "Could not read your accounts.")?;
        let connection = registry
            .connections
            .iter()
            .find(|connection| connection.id == model.connection)
            .ok_or("That account is no longer in Settings › Accounts.")?;
        if !model
            .model
            .starts_with(&format!("{}/", connection.provider))
            || model.model.len() <= connection.provider.len() + 1
            || model.model.len() > 200
            || model.model.chars().any(char::is_whitespace)
        {
            return Err("Choose a model this account serves.");
        }
    }
    defaults.decision_model = match defaults.decision_model.as_deref() {
        None => None,
        Some(value) => runtime::decision_model_setting(value)
            .map_err(|_| "Choose the included decision model or an OpenRouter model.")?,
    };
    Ok(defaults)
}

fn owner_only(principal: &RequestPrincipal) -> Option<Response<Body>> {
    (!principal.is_account_owner()).then(|| {
        api_error(
            StatusCode::FORBIDDEN,
            "new_company_defaults",
            "Only the account owner sets what new companies start on.",
        )
    })
}

pub(super) async fn read(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
) -> Response<Body> {
    if let Some(refused) = owner_only(&principal) {
        return refused;
    }
    match load(&state.daemon.root) {
        Ok(defaults) => Json(defaults).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "new_company_defaults",
            logged(error, "Could not read what new companies start on."),
        ),
    }
}

pub(super) async fn update(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    Json(input): Json<NewCompanyDefaults>,
) -> Response<Body> {
    if let Some(refused) = owner_only(&principal) {
        return refused;
    }
    let defaults = match validate(&state.daemon.root, input) {
        Ok(defaults) => defaults,
        Err(message) => return api_error(StatusCode::BAD_REQUEST, "new_company_defaults", message),
    };
    match save(&state.daemon.root, &defaults) {
        Ok(()) => Json(defaults).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "new_company_defaults",
            logged(
                error,
                "What new companies start on was not saved. Try again.",
            ),
        ),
    }
}

/// The decision model a new company starts with.
pub(super) fn apply_decision_model(
    defaults: &NewCompanyDefaults,
    config: &mut runtime::CompanyConfig,
) -> Result<()> {
    match defaults.decision_model.as_deref() {
        Some(value) => config.set_decision_model(value),
        None => Ok(()),
    }
}

/// Give a just-created company the owner's default model, exactly as "Use here" with "make it the
/// default" would. `Some(sentence)` says why it could not, for the owner to see.
pub(super) async fn grant_default_model(
    state: &OwnerState,
    by: &str,
    defaults: &NewCompanyDefaults,
    company: &str,
) -> Option<String> {
    let model = defaults.model.as_ref()?;
    let failed = |detail: &str| {
        Some(format!(
            "The company did not get your default model ({}): {detail} Choose its model in Company › Intelligence.",
            model.model
        ))
    };
    let Ok(config) = runtime::CompanyConfig::load(&state.daemon.root, company) else {
        return failed("its settings could not be read.");
    };
    let revision = company_setup_view(&config)["revision"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let response = grant_connection(
        state,
        by,
        company,
        &model.connection,
        GrantOwnerConnectionInput {
            model: model.model.clone(),
            revision,
            replace_existing: false,
            make_default: true,
        },
    )
    .await;
    if response.status().is_success() {
        return None;
    }
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap_or_default();
    let message = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|body| body["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "the account could not be given to it.".into());
    failed(&message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("new-company-defaults-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn defaults_round_trip_and_start_empty() {
        let root = root();
        assert_eq!(load(&root).unwrap(), NewCompanyDefaults::default());
        let defaults = NewCompanyDefaults {
            model: None,
            decision_model: Some("included".into()),
        };
        save(&root, &defaults).unwrap();
        assert_eq!(load(&root).unwrap(), defaults);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_choice_must_name_an_account_and_a_model_it_serves() {
        let root = root();
        let unknown = NewCompanyDefaults {
            model: Some(DefaultModel {
                connection: "nope".into(),
                model: "anthropic/claude-opus-5-5".into(),
            }),
            decision_model: None,
        };
        assert!(validate(&root, unknown).is_err());
        let decision = validate(
            &root,
            NewCompanyDefaults {
                model: None,
                decision_model: Some(" Included ".into()),
            },
        )
        .unwrap();
        assert_eq!(decision.decision_model.as_deref(), Some("included"));
        assert!(validate(
            &root,
            NewCompanyDefaults {
                model: None,
                decision_model: Some("not a model".into()),
            }
        )
        .is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_new_company_inherits_the_decision_model_and_an_old_one_keeps_its_own() {
        let defaults = NewCompanyDefaults {
            model: None,
            decision_model: Some("included".into()),
        };
        let mut config: runtime::CompanyConfig =
            toml::from_str("name = \"fresh\"\nmodel = \"\"\n").unwrap();
        apply_decision_model(&defaults, &mut config).unwrap();
        assert_eq!(config.decision_model.as_deref(), Some("included"));
        let mut none: runtime::CompanyConfig =
            toml::from_str("name = \"old\"\nmodel = \"\"\n").unwrap();
        apply_decision_model(&NewCompanyDefaults::default(), &mut none).unwrap();
        assert_eq!(none.decision_model, None);
    }
}
