//! The company browser as Chrome's own screen feed (S104 F4): sharper and lighter than the whole
//! VNC desktop, which stays one switch away. Frames come from `Page.startScreencast` on the tab the
//! company last used; input reaches the page only while this browser tab holds owner control, the
//! same rule the desktop applies per frame.

use super::*;
use axum::extract::ws::{Message as ClientMessage, WebSocket};
use base64::Engine as _;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::tungstenite::Message as CdpMessage;

/// How often the feed looks for a newer tab and re-reads who controls the computer.
const FOLLOW_INTERVAL: Duration = Duration::from_secs(3);

pub(crate) async fn desktop_screencast(
    State(state): State<OwnerState>,
    AxumPath(company): AxumPath<String>,
    headers: HeaderMap,
    Query(query): Query<desktop_api::DesktopWebsocketMode>,
    session_lease: Option<Extension<SessionLease>>,
    upgrade: WebSocketUpgrade,
) -> impl IntoResponse {
    if state.entry.network().is_some()
        && session_lease
            .as_ref()
            .is_none_or(|Extension(lease)| lease.is_ended())
    {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "no_session",
            "this plane requires a live verified entry session",
        );
    }
    let Some(client_id) = query.client_id.clone() else {
        return api_error(StatusCode::BAD_REQUEST, "client", "client_id is required");
    };
    if desktop_api::valid_attach_for(&state, &company, &headers, &client_id).is_none() {
        return api_error(
            StatusCode::UNAUTHORIZED,
            "attach",
            "desktop attachment is absent or expired",
        );
    }
    let session_lease = session_lease.map(|Extension(lease)| lease);
    upgrade
        .max_message_size(4 * 1024 * 1024)
        .on_upgrade(move |socket| async move {
            if let Err(error) = stream(socket, &company, &client_id, session_lease).await {
                tracing::warn!(company, "browser feed ended: {error:#}");
            }
        })
        .into_response()
}

/// Whether this browser tab holds owner control of the computer right now.
fn holds_control(control: Option<&serde_json::Value>, client_id: &str) -> bool {
    control.is_some_and(|value| {
        value["controller"] == "owner"
            && value["client_id"].as_str() == Some(client_id)
            && value["expires_at"]
                .as_str()
                .and_then(|value| value.parse::<chrono::DateTime<Utc>>().ok())
                .is_some_and(|expires| expires > Utc::now())
    })
}

/// The tab the company used last: Chrome lists pages most recently active first.
async fn front_page(company: &str) -> Result<(String, String)> {
    let output = runtime::docker_observe(&[
        "exec",
        &runtime::container_name(company),
        "curl",
        "--fail",
        "--silent",
        "--max-time",
        "3",
        "http://127.0.0.1:9222/json/list",
    ])
    .await?;
    let pages: Vec<serde_json::Value> =
        serde_json::from_slice(&output.stdout).context("decode the browser's tabs")?;
    let page = pages
        .iter()
        .find(|target| target["type"] == "page")
        .context("the company browser has no open tab")?;
    Ok((
        page["id"].as_str().unwrap_or_default().to_string(),
        page["url"].as_str().unwrap_or_default().to_string(),
    ))
}

async fn browser_socket(company: &str) -> Result<String> {
    let output = runtime::docker_observe(&[
        "exec",
        &runtime::container_name(company),
        "curl",
        "--fail",
        "--silent",
        "--max-time",
        "3",
        "http://127.0.0.1:9222/json/version",
    ])
    .await?;
    let version: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let url = version["webSocketDebuggerUrl"]
        .as_str()
        .context("the company browser gave no DevTools socket")?;
    Ok(url::Url::parse(url)?.path().to_string())
}

/// The viewer's size, in CSS pixels, and its device pixel ratio.
#[derive(Deserialize)]
struct FeedViewport {
    #[serde(rename = "type")]
    kind: String,
    width: f64,
    height: f64,
    #[serde(default)]
    scale: f64,
}

/// One input event from the viewer, in the page's own CSS pixels.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum FeedInput {
    Mouse {
        event: String,
        x: f64,
        y: f64,
        #[serde(default)]
        button: String,
        #[serde(default)]
        clicks: u32,
        #[serde(default)]
        modifiers: u32,
    },
    Wheel {
        x: f64,
        y: f64,
        dx: f64,
        dy: f64,
    },
    Key {
        event: String,
        key: String,
        code: String,
        #[serde(default)]
        key_code: u32,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        modifiers: u32,
    },
}

/// The page laid out at the viewer's own size (owner, 23 Oct: "make the browser render at the
/// available dimensions"), so the feed fills the viewer pixel for pixel instead of being scaled.
/// The override belongs to this feed's DevTools session: Chrome drops it when the feed closes, and
/// the browser returns to its own size for the agents. Bounded to sizes a page can sensibly use.
fn viewport_command(width: f64, height: f64, scale: f64) -> Option<serde_json::Value> {
    if !(width.is_finite() && height.is_finite()) || width < 200.0 || height < 150.0 {
        return None;
    }
    let scale = if scale.is_finite() {
        scale.clamp(1.0, 2.0)
    } else {
        1.0
    };
    Some(serde_json::json!({
        "width": width.min(2560.0).round(),
        "height": height.min(1600.0).round(),
        "deviceScaleFactor": scale,
        "mobile": false,
    }))
}

/// The DevTools command one viewer event becomes. `None` refuses anything malformed.
fn input_command(input: FeedInput) -> Option<(&'static str, serde_json::Value)> {
    match input {
        FeedInput::Mouse {
            event,
            x,
            y,
            button,
            clicks,
            modifiers,
        } => {
            let kind = match event.as_str() {
                "down" => "mousePressed",
                "up" => "mouseReleased",
                "move" => "mouseMoved",
                _ => return None,
            };
            let button = match button.as_str() {
                "left" | "middle" | "right" => button.as_str(),
                _ => "none",
            };
            Some((
                "Input.dispatchMouseEvent",
                serde_json::json!({ "type": kind, "x": x, "y": y, "button": button,
                    "clickCount": clicks.min(3), "modifiers": modifiers & 15 }),
            ))
        }
        FeedInput::Wheel { x, y, dx, dy } => Some((
            "Input.dispatchMouseEvent",
            serde_json::json!({ "type": "mouseWheel", "x": x, "y": y,
                "deltaX": dx.clamp(-5000.0, 5000.0), "deltaY": dy.clamp(-5000.0, 5000.0) }),
        )),
        FeedInput::Key {
            event,
            key,
            code,
            key_code,
            text,
            modifiers,
        } => {
            if key.len() > 32 || code.len() > 32 {
                return None;
            }
            let printable = text.as_deref().filter(|text| text.chars().count() == 1);
            let kind = match (event.as_str(), printable) {
                ("down", Some(_)) => "keyDown",
                ("down", None) => "rawKeyDown",
                ("up", _) => "keyUp",
                _ => return None,
            };
            let mut command = serde_json::json!({ "type": kind, "key": key, "code": code,
                "windowsVirtualKeyCode": key_code, "modifiers": modifiers & 15 });
            if kind == "keyDown" {
                command["text"] = serde_json::json!(printable);
            }
            Some(("Input.dispatchKeyEvent", command))
        }
    }
}

async fn stream(
    socket: WebSocket,
    company: &str,
    client_id: &str,
    session_lease: Option<SessionLease>,
) -> Result<()> {
    let (mut to_viewer, mut from_viewer) = socket.split();
    let path = browser_socket(company).await?;
    let upstream = runtime::devtools_stream(company).await?;
    let (cdp, _) = tokio_tungstenite::client_async(format!("ws://127.0.0.1:9222{path}"), upstream)
        .await
        .context("open the company browser's DevTools")?;
    let (mut to_chrome, mut from_chrome) = cdp.split();
    let mut next_id = 1u64;
    let mut attach_id = 0u64;
    let mut session: Option<String> = None;
    let mut target = String::new();
    let mut controls = false;
    let mut checked = std::time::Instant::now();
    let mut size = (0f64, 0f64);
    let mut viewport: Option<serde_json::Value> = None;
    let mut follow = tokio::time::interval(FOLLOW_INTERVAL);
    let lease_ended = async {
        match &session_lease {
            Some(lease) => lease.ended().await,
            None => std::future::pending().await,
        }
    };
    tokio::pin!(lease_ended);
    loop {
        tokio::select! {
            () = &mut lease_ended => break,
            _ = follow.tick() => {
                controls = holds_control(
                    runtime::read_browser_control(company).await.ok().flatten().as_ref(),
                    client_id,
                );
                let Ok((front, url)) = front_page(company).await else { continue };
                if front == target {
                    continue;
                }
                // Follow the tab the company is on: leave the old one and watch the new one.
                if let Some(old) = session.take() {
                    let detach = serde_json::json!({ "id": next_id, "method": "Target.detachFromTarget",
                        "params": { "sessionId": old } });
                    next_id += 1;
                    to_chrome.send(CdpMessage::Text(detach.to_string().into())).await?;
                }
                target = front;
                attach_id = next_id;
                next_id += 1;
                let attach = serde_json::json!({ "id": attach_id, "method": "Target.attachToTarget",
                    "params": { "targetId": target, "flatten": true } });
                to_chrome.send(CdpMessage::Text(attach.to_string().into())).await?;
                let note = serde_json::json!({ "type": "page", "url": url });
                to_viewer.send(ClientMessage::Text(note.to_string().into())).await?;
            }
            message = from_chrome.next() => {
                let Some(Ok(CdpMessage::Text(text))) = message else {
                    if message.is_none() { break }
                    continue;
                };
                let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
                if value["id"].as_u64() == Some(attach_id) {
                    let Some(attached) = value["result"]["sessionId"].as_str() else { continue };
                    session = Some(attached.to_string());
                    // A newly followed tab takes the viewer's size too, before its first frame.
                    if let Some(metrics) = &viewport {
                        let resize = serde_json::json!({ "id": next_id, "sessionId": attached,
                            "method": "Emulation.setDeviceMetricsOverride", "params": metrics });
                        next_id += 1;
                        to_chrome.send(CdpMessage::Text(resize.to_string().into())).await?;
                    }
                    let start = serde_json::json!({ "id": next_id, "sessionId": attached,
                        "method": "Page.startScreencast",
                        "params": { "format": "jpeg", "quality": 70, "maxWidth": 2560, "maxHeight": 1600 } });
                    next_id += 1;
                    to_chrome.send(CdpMessage::Text(start.to_string().into())).await?;
                    continue;
                }
                if value["method"] == "Target.detachedFromTarget"
                    && value["params"]["sessionId"].as_str() == session.as_deref()
                {
                    // The tab closed: the next look attaches to whichever is in front.
                    session = None;
                    target.clear();
                    continue;
                }
                if value["method"] != "Page.screencastFrame"
                    || value["sessionId"].as_str() != session.as_deref()
                {
                    continue;
                }
                let params = &value["params"];
                let ack = serde_json::json!({ "id": next_id, "sessionId": session,
                    "method": "Page.screencastFrameAck", "params": { "sessionId": params["sessionId"] } });
                next_id += 1;
                to_chrome.send(CdpMessage::Text(ack.to_string().into())).await?;
                let frame = (
                    params["metadata"]["deviceWidth"].as_f64().unwrap_or_default(),
                    params["metadata"]["deviceHeight"].as_f64().unwrap_or_default(),
                );
                if frame != size {
                    size = frame;
                    let note = serde_json::json!({ "type": "size", "width": size.0, "height": size.1 });
                    to_viewer.send(ClientMessage::Text(note.to_string().into())).await?;
                }
                let Some(data) = params["data"].as_str() else { continue };
                let Ok(jpeg) = base64::engine::general_purpose::STANDARD.decode(data) else { continue };
                to_viewer.send(ClientMessage::Binary(jpeg.into())).await?;
            }
            message = from_viewer.next() => {
                let Some(Ok(message)) = message else { break };
                let ClientMessage::Text(text) = message else {
                    if matches!(message, ClientMessage::Close(_)) { break }
                    continue;
                };
                // The viewer's size applies while it watches, control or not: it changes only how
                // the page is laid out, never what happens on it.
                if let Ok(FeedViewport { kind, width, height, scale }) = serde_json::from_str::<FeedViewport>(&text) {
                    if kind == "viewport" {
                        viewport = viewport_command(width, height, scale);
                        if let (Some(metrics), Some(current)) = (&viewport, session.as_deref()) {
                            let resize = serde_json::json!({ "id": next_id, "sessionId": current,
                                "method": "Emulation.setDeviceMetricsOverride", "params": metrics });
                            next_id += 1;
                            to_chrome.send(CdpMessage::Text(resize.to_string().into())).await?;
                        }
                        continue;
                    }
                }
                // Input reaches the page only while this tab holds control, as on the desktop.
                // A lease taken a moment ago is read again rather than waiting for the next look.
                if !controls && checked.elapsed() >= Duration::from_secs(1) {
                    checked = std::time::Instant::now();
                    controls = holds_control(
                        runtime::read_browser_control(company).await.ok().flatten().as_ref(),
                        client_id,
                    );
                }
                if !controls {
                    continue;
                }
                let Some(current) = session.as_deref() else { continue };
                let Ok(input) = serde_json::from_str::<FeedInput>(&text) else { continue };
                let Some((method, params)) = input_command(input) else { continue };
                let command = serde_json::json!({ "id": next_id, "sessionId": current,
                    "method": method, "params": params });
                next_id += 1;
                to_chrome.send(CdpMessage::Text(command.to_string().into())).await?;
            }
        }
    }
    let _ = to_viewer.close().await;
    let _ = to_chrome.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_viewer_size_becomes_a_bounded_layout_override() {
        let metrics = viewport_command(1684.4, 1100.0, 1.25).unwrap();
        assert_eq!(metrics["width"], 1684.0);
        assert_eq!(metrics["height"], 1100.0);
        assert_eq!(metrics["deviceScaleFactor"], 1.25);
        assert_eq!(metrics["mobile"], false);
        let huge = viewport_command(9000.0, 9000.0, 4.0).unwrap();
        assert_eq!(
            (huge["width"].clone(), huge["height"].clone()),
            (2560.0.into(), 1600.0.into())
        );
        assert_eq!(huge["deviceScaleFactor"], 2.0);
        assert!(viewport_command(100.0, 900.0, 1.0).is_none());
        assert!(viewport_command(f64::NAN, 900.0, 1.0).is_none());
    }

    #[test]
    fn viewer_events_become_bounded_devtools_input() {
        let (method, params) = input_command(
            serde_json::from_value(serde_json::json!({
                "type": "mouse", "event": "down", "x": 10.5, "y": 20.0, "button": "left", "clicks": 9
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(method, "Input.dispatchMouseEvent");
        assert_eq!(params["type"], "mousePressed");
        assert_eq!(params["clickCount"], 3);

        let (_, typed) = input_command(
            serde_json::from_value(serde_json::json!({
                "type": "key", "event": "down", "key": "a", "code": "KeyA", "key_code": 65, "text": "a"
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(typed["type"], "keyDown");
        assert_eq!(typed["text"], "a");

        let (_, enter) = input_command(
            serde_json::from_value(serde_json::json!({
                "type": "key", "event": "down", "key": "Enter", "code": "Enter", "key_code": 13
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(enter["type"], "rawKeyDown");
        assert!(enter.get("text").is_none());

        assert!(input_command(
            serde_json::from_value(serde_json::json!({
                "type": "mouse", "event": "teleport", "x": 0.0, "y": 0.0
            }))
            .unwrap()
        )
        .is_none());
    }

    #[test]
    fn only_this_tabs_live_owner_lease_controls() {
        let later = (Utc::now() + chrono::Duration::seconds(30)).to_rfc3339();
        let earlier = (Utc::now() - chrono::Duration::seconds(30)).to_rfc3339();
        let lease = |client: &str, expires: &str| serde_json::json!({ "controller": "owner", "client_id": client, "lease_id": "l", "expires_at": expires });
        assert!(holds_control(Some(&lease("tab-a", &later)), "tab-a"));
        assert!(!holds_control(Some(&lease("tab-b", &later)), "tab-a"));
        assert!(!holds_control(Some(&lease("tab-a", &earlier)), "tab-a"));
        assert!(!holds_control(
            Some(&serde_json::json!({ "controller": "agent" })),
            "tab-a"
        ));
        assert!(!holds_control(None, "tab-a"));
    }
}
