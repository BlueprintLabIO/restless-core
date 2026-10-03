//! Prepare self-hosted entry without changing the running plane or exporting secrets.
//! The existing identity installer owns accounts and its private configuration.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SetupInput {
    account_origin: String,
    core_origin: String,
    owner_email: String,
    access: String,
}

fn setup_origin(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value)?;
    let local = url
        .host_str()
        .is_some_and(|host| host == "localhost" || host.ends_with(".localhost"));
    if url.origin().ascii_serialization() != value
        || !url.username().is_empty()
        || url.password().is_some()
        || !(url.scheme() == "https" || (local && url.scheme() == "http"))
        || !url
            .host_str()
            .is_some_and(|host| host.contains('.') && host.parse::<std::net::IpAddr>().is_err())
    {
        bail!("Use an HTTPS address with a DNS hostname and no path.");
    }
    Ok(url)
}

pub(super) async fn prepare_setup(
    State(state): State<OwnerState>,
    Extension(principal): Extension<RequestPrincipal>,
    AxumPath(company): AxumPath<String>,
    Json(input): Json<SetupInput>,
) -> Response<Body> {
    if state.entry.network().is_some() || !principal.is_account_owner() {
        return api_error(StatusCode::FORBIDDEN, "sharing_setup", "Sharing setup is prepared by the local account owner. Manage existing access from Members.");
    }
    let prepare = async {
        let accounts = setup_origin(input.account_origin.trim())?;
        let core = setup_origin(input.core_origin.trim())?;
        if accounts.origin() == core.origin() {
            bail!("Accounts and the company need separate addresses.");
        }
        if accounts.scheme() != core.scheme() {
            bail!("Accounts and the company must both use HTTPS.");
        }
        let a = accounts.host_str().unwrap();
        let b = core.host_str().unwrap();
        let shared = a.split('.').rev().zip(b.split('.').rev()).take_while(|(a, b)| a == b).count();
        let same_site = a == b || (shared >= 2 && shared < a.split('.').count() && shared < b.split('.').count());
        if accounts.scheme() == "https" && !same_site {
            bail!("Use addresses on the same site, such as accounts.example.com and work.example.com.");
        }
        let email = input.owner_email.trim().to_lowercase();
        let valid_email = email.split_once('@').is_some_and(|(name, host)| !name.is_empty() && host.contains('.') && !host.starts_with('.') && !host.ends_with('.'));
        if email.len() > 254 || email.chars().any(char::is_whitespace) || email.chars().any(char::is_control) || !valid_email {
            bail!("Enter the owner's email address.");
        }
        if !matches!(input.access.as_str(), "private" | "https") {
            bail!("Choose private network or HTTPS access.");
        }
        let root = &state.daemon.root;
        let configured = crate::configured_companies(root)?;
        if !configured.iter().any(|name| name == &company) {
            bail!("This company is no longer available.");
        }
        let mut companies = Vec::new();
        for name in configured {
            let config = runtime::CompanyConfig::load(root, &name)?;
            let org = state.daemon.orgintel.get(&name).await?;
            let identity = org.company_access_identity().await?.context("The company has not finished initializing. Try again shortly.")?;
            companies.push(serde_json::json!({
                "id": name,
                "name": config.display_name.unwrap_or_else(|| company_display_name(&config.name)),
                "company_id": identity.company_id,
                "cell_id": identity.cell_id,
            }));
        }
        let image = std::env::var("RESTLESS_COMPANY_IMAGE").ok().filter(|image| {
            image.split_once("@sha256:").is_some_and(|(name, digest)| !name.is_empty() && digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        });
        Ok::<_, anyhow::Error>(serde_json::json!({
            "version": 1,
            "core_home": root,
            "company": company,
            "companies": companies,
            "account_origin": accounts.origin().ascii_serialization(),
            "core_origin": core.origin().ascii_serialization(),
            "owner_email": email,
            "access": input.access,
            "company_image": image,
        }))
    }.await;
    match prepare {
        Ok(setup) => Json(setup).into_response(),
        Err(error) => api_error(
            StatusCode::BAD_REQUEST,
            "sharing_setup",
            format!("{error:#}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharing_origins_do_not_carry_credentials_or_paths() {
        for value in [
            "http://work.example.com",
            "https://name:secret@work.example.com",
            "https://work.example.com/path",
            "https://127.0.0.1",
            "https://work.example.com?secret=value",
        ] {
            assert!(setup_origin(value).is_err(), "{value}");
        }
        assert!(setup_origin("https://work.example.com").is_ok());
        assert!(setup_origin("http://plane.localhost:7788").is_ok());
    }
}
