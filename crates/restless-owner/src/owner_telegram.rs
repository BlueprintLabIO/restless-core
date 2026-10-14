//! Owner API and plane loops for the Telegram Attention channel (ADR 0015).
//! The engine's `telegram` module owns the rules; this module resolves the
//! owner-plane facts it needs (the Authority owner) and drives the bot.

use super::*;
use restless_engine::telegram;

const DELIVERY_TICK: Duration = Duration::from_secs(15);
const LONG_POLL_SECS: u64 = 25;
const TOKEN_CACHE: Duration = Duration::from_secs(600);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PairInput {
    /// An existing secret reference, such as `infisical:/...` or `env:...`.
    #[serde(default)]
    token_reference: Option<String>,
    /// The bot token itself, as BotFather gives it. It is probed, kept in this company's
    /// Vault, and only its reference is stored with the pairing.
    #[serde(default)]
    token: Option<String>,
    cockpit_origin: String,
}

#[derive(Serialize)]
struct ChannelView {
    configured: bool,
    paired: bool,
    /// The stored cockpit origin is a local address, so a phone cannot open
    /// the links Telegram sends.
    local_links: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    link: Option<telegram::Link>,
    /// One tap opens the bot with the code prefilled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pair_url: Option<String>,
}

fn view(link: Option<telegram::Link>) -> ChannelView {
    let live_code = link.as_ref().and_then(|link| {
        let code = link.pairing_code.as_ref()?;
        (link.code_expires_at? > Utc::now())
            .then(|| format!("https://t.me/{}?start={code}", link.bot_username))
    });
    ChannelView {
        configured: link.is_some(),
        paired: link.as_ref().is_some_and(|link| link.paired_at.is_some()),
        local_links: link
            .as_ref()
            .is_some_and(|link| telegram::loopback_origin(&link.cockpit_origin)),
        pair_url: live_code,
        link,
    }
}

async fn gate(
    state: &OwnerState,
    company: &str,
    principal: &RequestPrincipal,
) -> Result<(), Response<Body>> {
    if let Err(error) = runtime::CompanyConfig::load(&state.daemon.root, company) {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            "company",
            format!("{error:#}"),
        ));
    }
    require_authority_owner(state, company, principal).await
}

pub(super) async fn status(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = gate(&state, &company, &principal).await {
        return refusal;
    }
    match telegram::link(state.daemon.authority.pool(), &company).await {
        Ok(link) => Json(view(link)).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "telegram",
            format!("{error:#}"),
        ),
    }
}

/// Store the bot token reference, live-probe the bot, and issue a fresh
/// pairing code. Re-running it replaces the previous pairing.
pub(super) async fn start(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PairInput>,
) -> Response<Body> {
    if let Err(refusal) = gate(&state, &company, &principal).await {
        return refusal;
    }
    let pasted = input
        .token
        .as_deref()
        .map(str::trim)
        .filter(|token| !token.is_empty());
    let reference = match (pasted, input.token_reference.as_deref().map(str::trim)) {
        (Some(_), _) => format!("infisical:/companies/{company}/telegram-bot"),
        (None, Some(reference)) if !reference.is_empty() => reference.to_string(),
        _ => {
            return api_error(
                StatusCode::BAD_REQUEST,
                "telegram",
                "Paste the bot token BotFather gave you.",
            )
        }
    };
    let probed = async {
        telegram::validate_origin(&input.cockpit_origin)?;
        let token = match pasted {
            Some(token) => token.to_string(),
            None => credential::resolve_reference(&reference)
                .await
                .context("the token reference did not resolve")?,
        };
        telegram::Bot::new(&token)?
            .username()
            .await
            .context("Telegram did not accept this bot token")
    }
    .await;
    let username = match probed {
        Ok(username) => username,
        Err(error) => return api_error(StatusCode::BAD_REQUEST, "telegram", format!("{error:#}")),
    };
    // A pasted token goes to the company Vault only after Telegram accepted it, so a typo
    // never overwrites a working token.
    if let Some(token) = pasted {
        if let Err(error) = credential::store_reference(&reference, token).await {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "telegram",
                format!("The Vault could not keep the bot token: {error:#}"),
            );
        }
    }
    let reference = reference.as_str();
    match telegram::start_pairing(
        state.daemon.authority.pool(),
        &company,
        reference,
        &username,
        &input.cockpit_origin,
        principal.actor_id(),
    )
    .await
    {
        Ok(link) => Json(view(Some(link))).into_response(),
        Err(error) => api_error(StatusCode::BAD_REQUEST, "telegram", format!("{error:#}")),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreferencesInput {
    decisions_only: bool,
}

/// What the paired chat receives: every Attention item, or only what it can decide.
pub(super) async fn preferences(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<PreferencesInput>,
) -> Response<Body> {
    if let Err(refusal) = gate(&state, &company, &principal).await {
        return refusal;
    }
    match telegram::set_decisions_only(state.daemon.authority.pool(), &company, input.decisions_only)
        .await
    {
        Ok(Some(link)) => Json(view(Some(link))).into_response(),
        Ok(None) => api_error(
            StatusCode::NOT_FOUND,
            "telegram",
            "Telegram is not paired for this company.",
        ),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "telegram",
            format!("{error:#}"),
        ),
    }
}

pub(super) async fn unpair(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
) -> Response<Body> {
    if let Err(refusal) = gate(&state, &company, &principal).await {
        return refusal;
    }
    match telegram::unpair(state.daemon.authority.pool(), &company).await {
        Ok(_) => Json(view(None)).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "telegram",
            format!("{error:#}"),
        ),
    }
}

#[async_trait::async_trait]
impl telegram::Companies for OwnerState {
    async fn authority_owner(&self, company: &str) -> Result<String> {
        let org = self.daemon.orgintel.get(company).await.ok();
        Ok(effective_authority_owner(self, company, org.as_ref())
            .await?
            .actor_id)
    }

    async fn orgintel(&self, company: &str) -> Option<restless_orgintel::OrgIntel> {
        self.daemon.orgintel.get(company).await.ok()
    }
}

/// Resolve each distinct token reference at most every ten minutes.
#[derive(Default)]
struct Bots(HashMap<String, (telegram::Bot, std::time::Instant)>);

impl Bots {
    async fn get(&mut self, reference: &str) -> Result<telegram::Bot> {
        if let Some((bot, at)) = self.0.get(reference) {
            if at.elapsed() < TOKEN_CACHE {
                return Ok(bot.clone());
            }
        }
        let token = credential::resolve_reference(reference).await?;
        let bot = telegram::Bot::new(&token)?;
        self.0.insert(
            reference.to_string(),
            (bot.clone(), std::time::Instant::now()),
        );
        Ok(bot)
    }
}

/// Start both loops. They do nothing until a company stores a token.
pub(super) fn spawn(state: OwnerState) {
    tokio::spawn(poll_loop(state.clone()));
    tokio::spawn(delivery_loop(state));
}

/// Long-poll every bot that has a link: pairing codes and button presses.
async fn poll_loop(state: OwnerState) {
    let mut bots = Bots::default();
    let mut offsets: HashMap<String, i64> = HashMap::new();
    loop {
        let references = match telegram::links(state.daemon.authority.pool()).await {
            Ok(links) => links
                .into_iter()
                .map(|link| link.token_reference)
                .collect::<std::collections::BTreeSet<_>>(),
            Err(error) => {
                tracing::warn!("Telegram links unavailable: {error:#}");
                Default::default()
            }
        };
        if references.is_empty() {
            tokio::time::sleep(DELIVERY_TICK).await;
            continue;
        }
        let timeout = (LONG_POLL_SECS / references.len() as u64).max(1);
        for reference in references {
            let outcome = async {
                let bot = bots.get(&reference).await?;
                let offset = offsets.entry(reference.clone()).or_insert(0);
                telegram::poll_once(
                    &state.daemon.authority,
                    &state.daemon.root,
                    &state,
                    &bot,
                    &reference,
                    offset,
                    timeout,
                )
                .await
            }
            .await;
            if let Err(error) = outcome {
                tracing::warn!(%reference, "Telegram poll failed: {error:#}");
                bots.0.remove(&reference);
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

/// Send each paired chat the Attention items it has not been sent.
async fn delivery_loop(state: OwnerState) {
    let mut bots = Bots::default();
    loop {
        tokio::time::sleep(DELIVERY_TICK).await;
        let links = match telegram::links(state.daemon.authority.pool()).await {
            Ok(links) => links,
            Err(error) => {
                tracing::warn!("Telegram links unavailable: {error:#}");
                continue;
            }
        };
        for link in links.into_iter().filter(|link| link.chat_id.is_some()) {
            let outcome = async {
                let bot = bots.get(&link.token_reference).await?;
                let config = runtime::CompanyConfig::load(&state.daemon.root, &link.company)?;
                let org = state.daemon.orgintel.get(&link.company).await?;
                let attention =
                    attention::project_with_spend(
                        &config,
                        &state.daemon.authority,
                        Some(&org),
                        &state.daemon.spend,
                    )
                    .await?;
                let display = config
                    .display_name
                    .clone()
                    .unwrap_or_else(|| config.name.clone());
                let mut items = attention.items;
                // While the owner is away, only what blocks the company reaches their phone.
                if restless_engine::away::current(&state.daemon.authority, &link.company)
                    .await
                    .ok()
                    .flatten()
                    .is_some()
                {
                    items.retain(|item| item.category == "blocker");
                }
                items.sort_by_key(|item| item.created_at);
                let mut outgoing = Vec::new();
                for item in &items {
                    if let Some(message) = telegram::outgoing(
                        &state.daemon.authority,
                        &link.company,
                        &display,
                        &link.cockpit_origin,
                        item,
                    )
                    .await?
                    {
                        if !link.decisions_only || message.binding.is_some() {
                            outgoing.push(message);
                        }
                    }
                }
                telegram::deliver(state.daemon.authority.pool(), &bot, &link, outgoing).await
            }
            .await;
            if let Err(error) = outcome {
                tracing::warn!(company = %link.company, "Telegram delivery deferred: {error:#}");
            }
        }
    }
}
