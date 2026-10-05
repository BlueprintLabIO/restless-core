//! Owner control of the company computer's desktop: attach tickets and cookies, observed and controlled desktop sessions, the RFB observer filter, window focus and display leases.

use super::*;

/// Owner desktop control is deliberately short-lived. The cockpit renews this
/// only after input reaches the remote desktop; merely leaving a tab open must
/// not strand the Company computer under an absent owner's control.
pub(crate) const CONTROL_TTL_SECONDS: i64 = 60;

pub(crate) const ATTACH_COOKIE: &str = "restless_attach";

pub(crate) const TICKET_TTL: Duration = Duration::from_secs(30);

pub(crate) const ATTACH_TTL: Duration = Duration::from_secs(30 * 60);

pub(crate) const DISPLAY_LEASE_SECONDS: i64 = 30;

pub(crate) const MAX_DESKTOP_WIDTH: u32 = 3840;

pub(crate) const MAX_DESKTOP_HEIGHT: u32 = 2160;

pub(crate) static DESKTOP_DISPLAY_LEASES: std::sync::LazyLock<
    tokio::sync::Mutex<HashMap<String, DesktopDisplayLease>>,
> = std::sync::LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

#[derive(Clone)]
pub(crate) struct AttachTicket {
    pub(crate) company: String,
    pub(crate) generation: String,
    pub(crate) item_id: String,
    pub(crate) client_id: String,
    pub(crate) requesting_actor: Option<String>,
    pub(crate) work_id: Option<Uuid>,
    pub(crate) attempt_id: Option<Uuid>,
    pub(crate) expires_at: SystemTime,
}

#[derive(Clone)]
pub(crate) struct AttachSession {
    pub(crate) company: String,
    pub(crate) client_id: String,
    pub(crate) requesting_actor: Option<String>,
    pub(crate) work_id: Option<Uuid>,
    pub(crate) attempt_id: Option<Uuid>,
    pub(crate) expires_at: SystemTime,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TicketRequest {
    pub(crate) item_id: String,
    pub(crate) client_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TicketResponse {
    pub(crate) desktop_url: String,
    pub(crate) expires_in_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TicketQuery {
    pub(crate) ticket: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DesktopWindowFocusRequest {
    pub(crate) client_id: String,
    pub(crate) lease_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DesktopDisplayLeaseRequest {
    pub(crate) client_id: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Clone)]
pub(crate) struct DesktopDisplayLease {
    pub(crate) client_id: String,
    pub(crate) expires_at: SystemTime,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct DesktopMode {
    pub(crate) client_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct DesktopWebsocketMode {
    pub(crate) client_id: Option<String>,
}

pub(crate) async fn issue_ticket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<TicketRequest>,
) -> impl IntoResponse {
    if Uuid::parse_str(&input.client_id).is_err() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "client",
            "client_id must be a UUID",
        );
    }
    let current = match runtime::generation(&company).await {
        Ok(Some(generation)) => generation,
        _ => {
            return api_error(
                StatusCode::SERVICE_UNAVAILABLE,
                "runtime",
                "company runtime is unavailable",
            )
        }
    };
    let mut requesting_actor = None;
    let mut attached_work_id = None;
    let mut attached_attempt_id = None;
    if input.item_id != "runtime-rescue" {
        let config = match runtime::CompanyConfig::load(&state.daemon.root, &company) {
            Ok(config) => config,
            Err(error) => return api_error(StatusCode::NOT_FOUND, "company", format!("{error:#}")),
        };
        let org = state.daemon.orgintel.get(&company).await.ok();
        let view = match attention::project(&config, &state.daemon.authority, org.as_ref()).await {
            Ok(view) => view,
            Err(error) => {
                return api_error(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "projection",
                    format!("{error:#}"),
                )
            }
        };
        let Some(item) = view.items.iter().find(|item| item.id == input.item_id) else {
            return api_error(
                StatusCode::NOT_FOUND,
                "attention",
                "attention source is no longer outstanding",
            );
        };
        let Some(reference) = item.runtime_attach.as_ref() else {
            return api_error(
                StatusCode::CONFLICT,
                "runtime",
                "this item has no live runtime attachment",
            );
        };
        if current != reference.generation {
            return api_error(
                StatusCode::CONFLICT,
                "runtime",
                "runtime generation changed; refresh the item",
            );
        }
        requesting_actor.clone_from(&reference.requesting_actor);
        attached_work_id = item.work_id;
        attached_attempt_id = reference.attempt_id;
    }
    let ticket = Uuid::new_v4().simple().to_string();
    state.tickets.lock().expect("ticket registry").insert(
        ticket.clone(),
        AttachTicket {
            company: company.clone(),
            generation: current,
            item_id: input.item_id,
            client_id: input.client_id,
            requesting_actor,
            work_id: attached_work_id,
            attempt_id: attached_attempt_id,
            expires_at: SystemTime::now() + TICKET_TTL,
        },
    );
    Json(TicketResponse {
        desktop_url: format!("/desktop/{company}?ticket={ticket}"),
        expires_in_seconds: TICKET_TTL.as_secs(),
    })
    .into_response()
}

pub(crate) async fn open_desktop(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    Query(query): Query<TicketQuery>,
) -> impl IntoResponse {
    let ticket = state
        .tickets
        .lock()
        .expect("ticket registry")
        .remove(&query.ticket);
    let Some(ticket) = ticket else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "ticket",
            "attach ticket is invalid or already used",
        );
    };
    if ticket.company != company || ticket.expires_at <= SystemTime::now() {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "ticket",
            "attach ticket is expired or belongs to another company",
        );
    }
    let current = runtime::generation(&company).await.ok().flatten();
    if current.as_deref() != Some(ticket.generation.as_str()) {
        return api_error(
            StatusCode::CONFLICT,
            "runtime",
            "attach ticket names a stale runtime generation",
        );
    }
    let attach = Uuid::new_v4().simple().to_string();
    let client_id = ticket.client_id.clone();
    state.attaches.lock().expect("attach registry").insert(
        attach.clone(),
        AttachSession {
            company: company.clone(),
            client_id: client_id.clone(),
            requesting_actor: ticket.requesting_actor,
            work_id: ticket.work_id,
            attempt_id: ticket.attempt_id,
            expires_at: SystemTime::now() + ATTACH_TTL,
        },
    );
    tracing::info!(company, item = %ticket.item_id, "owner desktop attached");
    let target = desktop_client_url(&company, DesktopClientMode::Observe, None, &client_id);
    let mut response = Redirect::to(&target).into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{}={attach}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            attach_cookie_name(&client_id).expect("ticket client id is a UUID"),
            ATTACH_TTL.as_secs()
        ))
        .expect("attach cookie"),
    );
    response
}

#[derive(Clone, Copy)]
pub(crate) enum DesktopClientMode {
    Observe,
    Control,
}

/// Every attached tab uses one persistent, input-capable protocol connection.
/// The server gates input and desktop resizing against live leases per frame.
pub(crate) fn desktop_client_url(
    company: &str,
    mode: DesktopClientMode,
    lease_id: Option<&str>,
    client_id: &str,
) -> String {
    let (resize, view_only) = match mode {
        DesktopClientMode::Observe => ("remote", "1"),
        DesktopClientMode::Control => ("remote", "0"),
    };
    // noVNC reads `path` as one URL query value and uses it to construct its
    // WebSocket URL. All modes share the server-authorized dynamic socket.
    let websocket_path = match mode {
        DesktopClientMode::Observe => format!("desktop/{company}/websockify%3Fclient_id%3D{client_id}"),
        DesktopClientMode::Control => format!(
            "desktop/{company}/websockify%3Fmode%3Dcontrol%26lease_id%3D{}%26client_id%3D{client_id}",
            lease_id.expect("control desktop URL requires a lease id"),
        ),
    };
    format!(
        "/desktop/{company}/vnc.html?autoconnect=1&reconnect=1&reconnect_delay=1000&shared=1&show_dot=1&resize={resize}&view_only={view_only}&path={websocket_path}"
    )
}

pub(crate) async fn open_observed_desktop(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Query(query): Query<DesktopMode>,
) -> impl IntoResponse {
    let Some(client_id) = query.client_id.as_deref() else {
        return api_error(StatusCode::BAD_REQUEST, "client", "client_id is required");
    };
    let Some(attach) = valid_attach_for(&state, &company, &headers, client_id) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    };
    Redirect::to(&desktop_client_url(
        &company,
        DesktopClientMode::Observe,
        None,
        &attach.client_id,
    ))
    .into_response()
}

pub(crate) async fn open_controlled_desktop(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Query(query): Query<DesktopMode>,
) -> impl IntoResponse {
    let Some(client_id) = query.client_id.as_deref() else {
        return api_error(StatusCode::BAD_REQUEST, "client", "client_id is required");
    };
    let Some(attach) = valid_attach_for(&state, &company, &headers, client_id) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    };
    let requested = client_id;
    if requested != attach.client_id {
        return api_error(
            StatusCode::FORBIDDEN,
            "controller",
            "attachment belongs to another browser tab",
        );
    }
    let control = runtime::read_browser_control(&company).await.ok().flatten();
    let allowed = control.as_ref().is_some_and(|value| {
        value["controller"] == "owner"
            && value["client_id"].as_str() == Some(requested)
            && value["lease_id"]
                .as_str()
                .is_some_and(|lease_id| !lease_id.is_empty())
            && value["expires_at"]
                .as_str()
                .and_then(|value| value.parse::<chrono::DateTime<Utc>>().ok())
                .is_some_and(|expires| expires > Utc::now())
    });
    if !allowed {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "this browser tab does not hold control",
        );
    }
    Redirect::to(&desktop_client_url(
        &company,
        DesktopClientMode::Control,
        control
            .as_ref()
            .and_then(|value| value["lease_id"].as_str()),
        &attach.client_id,
    ))
    .into_response()
}

pub(crate) async fn desktop_asset(
    State(state): State<OwnerState>,
    AxumPath((company, asset)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if valid_attach(&state, &company, &headers).is_none() {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    }
    match runtime::desktop_asset(&company, &asset).await {
        Ok(bytes) => {
            let mut response = Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(bytes))
                .expect("response");
            response.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static(desktop_content_type(&asset)),
            );
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            response
        }
        Err(error) => api_error(
            StatusCode::BAD_GATEWAY,
            "desktop",
            format!("desktop asset bridge: {error:#}"),
        ),
    }
}

pub(crate) fn desktop_content_type(asset: &str) -> &'static str {
    match asset.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

pub(crate) async fn desktop_websocket(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Query(query): Query<DesktopWebsocketMode>,
    session_lease: Option<Extension<SessionLease>>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    if state.entry.network().is_some()
        && session_lease
            .as_ref()
            .is_none_or(|Extension(lease)| lease.is_ended())
    {
        // Boundary middleware installs the lease atomically with resolving
        // the session. Refuse a network upgrade if that invariant is ever
        // broken rather than creating an uncancellable desktop channel.
        return api_error(
            StatusCode::UNAUTHORIZED,
            "no_session",
            "this plane requires a live verified entry session",
        );
    }
    let Some(client_id) = query.client_id.as_deref() else {
        return api_error(StatusCode::BAD_REQUEST, "client", "client_id is required");
    };
    let Some(attach) = valid_attach_for(&state, &company, &headers, client_id) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    };
    // One transport stays attached for the lifetime of the viewer. Server-side
    // RFB filtering grants input and resize independently as leases change.
    let access = DesktopWebsocketAccess::Live {
        client_id: attach.client_id,
    };
    let control_guard = runtime::browser_control_guard(&company).await;
    let current_control = runtime::read_browser_control(&company).await.ok().flatten();
    runtime::publish_browser_control(&company, current_control).await;
    let control_watch = Some(runtime::watch_browser_control(&company).await);
    drop(control_guard);
    let session_lease = session_lease.map(|Extension(lease)| lease);
    upgrade
        .on_upgrade(move |socket| async move {
            if let Err(error) =
                proxy_websocket(socket, &company, access, control_watch, session_lease).await
            {
                tracing::warn!(company, "desktop websocket ended: {error:#}");
            }
        })
        .into_response()
}

pub(crate) enum DesktopWebsocketAccess {
    Live { client_id: String },
}

/// Server-enforced view-only filtering for the RFB client-to-server stream.
/// noVNC still needs display-negotiation and framebuffer-request messages to
/// render an observation, while key, pointer and clipboard messages must
/// never reach the company desktop.
pub(crate) struct RfbObserverFilter {
    pub(crate) pending: Vec<u8>,
    pub(crate) handshake_remaining: usize,
    pub(crate) handshake_step: u8,
}

impl Default for RfbObserverFilter {
    fn default() -> Self {
        Self {
            pending: Vec::new(),
            handshake_remaining: 12,
            handshake_step: 0,
        }
    }
}

impl RfbObserverFilter {
    pub(crate) fn filter(
        &mut self,
        bytes: &[u8],
        allow_input: bool,
        allow_resize: bool,
    ) -> Result<Vec<tungstenite::Message>> {
        const MAX_PENDING: usize = 1024 * 1024;
        self.pending.extend_from_slice(bytes);
        if self.pending.len() > MAX_PENDING {
            bail!("view-only desktop protocol message is too large");
        }
        let mut forwarded = Vec::new();
        // RFB requires three turn-taking client handshake messages. Forward
        // each only after validating it; buffering all three would deadlock
        // while the client waits for the server's security response.
        if self.handshake_remaining > 0 {
            if self.pending.len() < self.handshake_remaining {
                return Ok(forwarded);
            }
            let handshake: Vec<u8> = self.pending.drain(..self.handshake_remaining).collect();
            match self.handshake_step {
                0 if handshake.as_slice() == b"RFB 003.008\n" => {
                    self.handshake_step = 1;
                    self.handshake_remaining = 1;
                }
                1 if handshake.as_slice() == [1] => {
                    self.handshake_step = 2;
                    self.handshake_remaining = 1;
                }
                // Shared ClientInit prevents an observation session from
                // displacing an active controller at the VNC server.
                2 if handshake.as_slice() == [1] => {
                    self.handshake_step = 3;
                    self.handshake_remaining = 0;
                }
                _ => bail!("view-only desktop requires the shared RFB 3.8 handshake"),
            }
            forwarded.push(tungstenite::Message::Binary(handshake.into()));
            return Ok(forwarded);
        }
        loop {
            let Some((&kind, rest)) = self.pending.split_first() else {
                break;
            };
            let length = match kind {
                // SetPixelFormat, SetEncodings and FramebufferUpdateRequest
                // are required for a viewer to negotiate and request pixels.
                0 => 20,
                2 => {
                    if rest.len() < 3 {
                        break;
                    }
                    4 + 4 * u16::from_be_bytes([rest[1], rest[2]]) as usize
                }
                3 => 10,
                // KeyEvent, PointerEvent and ClientCutText are consumed but
                // deliberately not forwarded.
                4 => 8,
                5 => 6,
                6 => {
                    if rest.len() < 7 {
                        break;
                    }
                    // Extended clipboard frames encode their payload length
                    // as a negative signed integer. Treating that as u32
                    // leaves every later input or resize frame queued behind
                    // an impossible multi-gigabyte message.
                    8 + i32::from_be_bytes([rest[3], rest[4], rest[5], rest[6]]).unsigned_abs()
                        as usize
                }
                251 => {
                    if rest.len() < 7 {
                        break;
                    }
                    8 + 16 * rest[5] as usize
                }
                // Fence is coordination for the viewer, not desktop input:
                // type + padding + flags + one-byte payload length.
                248 => {
                    if rest.len() < 8 {
                        break;
                    }
                    9 + rest[7] as usize
                }
                // EnableContinuousUpdates asks for pixels; it does not alter
                // the remote desktop.
                150 => 10,
                _ => bail!("unsupported view-only desktop protocol message {kind}"),
            };
            if self.pending.len() < length {
                break;
            }
            let message: Vec<u8> = self.pending.drain(..length).collect();
            if matches!(kind, 0 | 2 | 3 | 150 | 248)
                || (allow_input && matches!(kind, 4 | 5 | 6))
                || (allow_resize && kind == 251)
            {
                forwarded.push(tungstenite::Message::Binary(message.into()));
            }
        }
        Ok(forwarded)
    }
}

pub(crate) async fn desktop_windows(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if valid_attach(&state, &company, &headers).is_none() {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "open this company computer before listing its windows",
        );
    }
    match runtime::desktop_windows(&company).await {
        Ok(windows) => Json(windows).into_response(),
        Err(error) => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "desktop",
            format!("{error:#}"),
        ),
    }
}

pub(crate) async fn focus_desktop_window(
    State(state): State<OwnerState>,
    AxumPath((company, window_id)): AxumPath<(String, String)>,
    headers: HeaderMap,
    Json(input): Json<DesktopWindowFocusRequest>,
) -> impl IntoResponse {
    let Some(_attach) = valid_attach_for(&state, &company, &headers, &input.client_id) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "open this company computer before focusing a window",
        );
    };
    if input.client_id.len() > 128 || input.lease_id.len() > 128 {
        return api_error(
            StatusCode::FORBIDDEN,
            "controller",
            "window focus must come from the attached controlling tab",
        );
    }
    // Hold the same lock as take/return/heartbeat until the broker completes
    // activation, so hand-back cannot race a focus request.
    let _control_guard = runtime::browser_control_guard(&company).await;
    let Some(control) = runtime::read_browser_control(&company).await.ok().flatten() else {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "take control of the company computer before focusing a window",
        );
    };
    if control_expiry(Some(&control), &input.client_id, &input.lease_id).is_none() {
        return api_error(
            StatusCode::CONFLICT,
            "controller",
            "this tab does not hold the live company computer lease",
        );
    }
    match runtime::focus_desktop_window(&company, &window_id, &input.client_id, &input.lease_id)
        .await
    {
        Ok(()) => Json(serde_json::json!({ "focused": true })).into_response(),
        Err(error) => api_error(StatusCode::CONFLICT, "desktop", format!("{error:#}")),
    }
}

pub(crate) async fn claim_desktop_display_lease(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Json(input): Json<DesktopDisplayLeaseRequest>,
) -> impl IntoResponse {
    let Some(attach) = valid_attach_for(&state, &company, &headers, &input.client_id) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    };
    // Bound the exact CSS geometry noVNC will send in its SetDesktopSize
    // message, so the lease metadata and the wire request stay in agreement.
    if !(1..=MAX_DESKTOP_WIDTH).contains(&input.width)
        || !(1..=MAX_DESKTOP_HEIGHT).contains(&input.height)
    {
        return api_error(
            StatusCode::BAD_REQUEST,
            "display",
            "viewport dimensions are outside supported bounds",
        );
    }
    let width = input.width;
    let height = input.height;
    let _control_guard = runtime::browser_control_guard(&company).await;
    let active_control = runtime::read_browser_control(&company).await.ok().flatten();
    let controlling_here = active_control.as_ref().is_some_and(|control| {
        control["controller"] == "owner"
            && control["client_id"].as_str() == Some(&attach.client_id)
            && lease_is_live(control)
    });
    let other_controller = active_control
        .as_ref()
        .is_some_and(|control| control["controller"] == "owner" && lease_is_live(control))
        && !controlling_here;
    let mut leases = DESKTOP_DISPLAY_LEASES.lock().await;
    let now = SystemTime::now();
    leases.retain(|_, lease| lease.expires_at > now);
    let current = leases
        .get(&company)
        .cloned()
        .filter(|lease| lease.expires_at > now);
    let can_resize = if controlling_here {
        true
    } else if other_controller {
        false
    } else {
        match current {
            Some(ref lease) if lease.client_id == attach.client_id => true,
            Some(_) => false,
            None => {
                leases.insert(
                    company.clone(),
                    DesktopDisplayLease {
                        client_id: attach.client_id.clone(),
                        expires_at: now + Duration::from_secs(DISPLAY_LEASE_SECONDS as u64),
                    },
                );
                true
            }
        }
    };
    if can_resize {
        // An active controller owns the display lease too, so this tab remains
        // the primary viewer briefly after hand-back instead of being displaced
        // by a background observer's older claim.
        leases.insert(
            company.clone(),
            DesktopDisplayLease {
                client_id: attach.client_id.clone(),
                expires_at: now + Duration::from_secs(DISPLAY_LEASE_SECONDS as u64),
            },
        );
    }
    drop(leases);
    let Some(attach_id) = refresh_attach(&state, &company, &headers, &attach.client_id) else {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment expired during refresh",
        );
    };
    let mut response = Json(serde_json::json!({
        "can_resize": can_resize,
        "width": width,
        "height": height,
        "expires_in_seconds": if can_resize { DISPLAY_LEASE_SECONDS } else { 0 },
    }))
    .into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{}={attach_id}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            attach_cookie_name(&attach.client_id).expect("attached client id is a UUID"),
            ATTACH_TTL.as_secs()
        ))
        .expect("attach refresh cookie"),
    );
    response
}

pub(crate) fn attach_cookie_name(client_id: &str) -> Option<String> {
    Uuid::parse_str(client_id)
        .ok()
        .map(|client_id| format!("{ATTACH_COOKIE}_{}", client_id.simple()))
}

pub(crate) fn valid_attach_for(
    state: &OwnerState,
    company: &str,
    headers: &HeaderMap,
    client_id: &str,
) -> Option<AttachSession> {
    let id = cookie(headers, &attach_cookie_name(client_id)?)?;
    let mut attaches = state.attaches.lock().expect("attach registry");
    attaches.retain(|_, attach| attach.expires_at > SystemTime::now());
    attaches
        .get(&id)
        .filter(|attach| {
            attach.company == company
                && Uuid::parse_str(&attach.client_id).ok() == Uuid::parse_str(client_id).ok()
        })
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_viewers_share_the_dynamic_desktop_socket() {
        let client_id = "00000000-0000-0000-0000-000000000001";
        let observer =
            desktop_client_url("company_test", DesktopClientMode::Observe, None, client_id);
        assert!(observer.contains("resize=remote"));
        assert!(observer.contains("view_only=1"));
        assert!(observer.contains("show_dot=1"));
        assert!(observer.contains("client_id%3D00000000-0000-0000-0000-000000000001"));

        let controller = desktop_client_url(
            "company_test",
            DesktopClientMode::Control,
            Some("lease-test"),
            client_id,
        );
        assert!(controller.contains("resize=remote"));
        assert!(controller.contains("view_only=0"));
        assert!(controller.contains("show_dot=1"));
        assert!(controller.contains("reconnect=1"));
        assert!(controller.contains("lease_id%3Dlease-test"));
    }

    #[test]
    fn observed_desktop_gates_input_and_resize_per_live_lease() {
        let mut filter = RfbObserverFilter::default();
        assert!(filter.filter(b"RFB 003.008\n", false, false).unwrap().len() == 1);
        assert!(filter.filter(&[1], false, false).unwrap().len() == 1);
        assert!(filter.filter(&[1], false, false).unwrap().len() == 1);

        let framebuffer_request = [3, 0, 0, 0, 0, 0, 0, 4, 0, 4];
        let key_event = [4, 0, 0, 0, 0, 0, 0, 65];
        let mut frames = framebuffer_request.to_vec();
        frames.extend(key_event);
        let forwarded = filter.filter(&frames, false, false).unwrap();
        assert_eq!(forwarded.len(), 1);
        match &forwarded[0] {
            tungstenite::Message::Binary(bytes) => assert_eq!(bytes.as_ref(), framebuffer_request),
            _ => panic!("framebuffer request should remain binary"),
        }
    }
}
