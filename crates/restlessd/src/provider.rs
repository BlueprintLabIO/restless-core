//! Real providers behind the effect surface (S03-T1).
//!
//! Sprint 01/02 had exactly one world: a model playing a persona. That was the
//! right shape to build against — `ARCHITECTURE.md §10.8` claims the
//! company-side path is identical for a simulated and a real provider — but the
//! claim was never tested, because there was nothing real to test it with.
//!
//! Three rules this module exists to hold:
//!
//! * **Dispatch is per `(company, capability)`.** A company with no entry keeps
//!   the simulator. That is what makes `aris_test` structurally safe (S03-T7):
//!   the failure mode of a mistake is a simulated send, because the table has
//!   no real entry to find, not because someone remembered a rule.
//! * **The adapter runs host-side.** The provider credential is read in the
//!   daemon at the point of use and never crosses into a company container
//!   (`authority-plane §2.6`, `company-runtime §11.5`). The agent's path is
//!   unchanged: it still calls `restless effect email.send`.
//! * **Our idempotency key is the provider's.** Resend accepts an
//!   `Idempotency-Key` header, so a retry that reaches the provider is
//!   deduplicated there as well as here. Two layers, because the failure we are
//!   guarding is "the daemon died between sending and writing the receipt" —
//!   which our own replay check cannot see.

use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use sha2::Digest as _;

use crate::runtime::CompanyConfig;

/// Which provider serves one capability for one company.
///
/// `Simulated` is not a fallback that happens when configuration is missing —
/// it is the default world, and a company only leaves it by an explicit entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Simulated,
    Resend,
    /// S04-T3. A governed `git push`: the daemon pushes the company's branch
    /// to the repository's own origin, host-side.
    ///
    /// **A transport, not a vendor.** This variant never learns a forge's API —
    /// no pull requests, no reviews, no issues. The same path serves GitHub,
    /// GitLab, Gitea and a bare remote on a VPS, so adding a forge adds no code
    /// and there is no catalogue to grow (the thing `SelfReported`'s doc below
    /// warns about, and rev 1 of sprint 04 walked into).
    ///
    /// It is an adapter rather than `SelfReported` for one reason: self-reporting
    /// a push means the *company* performs it, which means a git credential
    /// inside the container — the regression S03-T4 closed for email, and the
    /// one invariant this sprint must not reopen.
    ///
    /// The pull request is the owner's to open, from the compare URL in the
    /// receipt. That is the prepared last mile, and it keeps the human
    /// authority act human.
    Git,
    /// **The general case.** The company performed the action itself — through
    /// its browser session, a CLI, a vendor dashboard, anything — and reports
    /// what happened. The daemon records the receipt without having performed
    /// the action.
    ///
    /// `authority-plane §2.2` is explicit that this is the *primary* shape and
    /// an adapter is the exception: *"A receipt does not require an API… the
    /// receipt, idempotency key, party and reconciliation are identical either
    /// way. Accountability attaches to the consequence, not to the transport.
    /// This matters because the set of consequential actions with clean APIs is
    /// much smaller than the set of consequential actions, and an adapter per
    /// provider does not scale."*
    ///
    /// We built the adapter first anyway, which got email working and got the
    /// architecture backwards. This is the path that scales.
    SelfReported,
}

impl Provider {
    /// The string that lands in the receipt. This is the value
    /// `evaluation-dogfood` reads to tell a real outcome from a rehearsed one,
    /// so it is the provider's own name, never a category.
    ///
    /// `self-reported` is deliberately not dressed up. A receipt the company
    /// wrote about itself is **weaker evidence** than one a provider confirmed,
    /// and reconciliation must be able to tell them apart — Aris once reported
    /// £45 of revenue that receipts put at £18, and the whole value of this
    /// field is that it says who is attesting.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Simulated => "simulated",
            Self::Resend => "resend",
            Self::Git => "git",
            Self::SelfReported => "self-reported",
        }
    }
}

/// Resolve the provider for one capability.
///
/// Unknown provider names are a hard error rather than a silent fall back to
/// the simulator: a typo in `email.send = "resned"` must not quietly send
/// nothing while reporting success. Failing closed here is the whole point of
/// the table.
pub fn resolve(config: &CompanyConfig, capability: &str) -> Result<Provider> {
    // S04-T1. A throwaway company cannot reach the world, whatever its config
    // says. `clone_config` already strips the dispatch table, so this is the
    // second lock rather than the only one — it covers a hand-written `_test`
    // config, which is exactly how the first contamination happened (the live
    // company was the convenient one).
    if crate::runtime::is_test_company(&config.name) {
        return Ok(Provider::Simulated);
    }
    match config.providers.get(capability) {
        None => Ok(Provider::Simulated),
        Some(name) => match name.as_str() {
            "simulated" => Ok(Provider::Simulated),
            "resend" => Ok(Provider::Resend),
            "git" => Ok(Provider::Git),
            "self" | "self-reported" => Ok(Provider::SelfReported),
            other => bail!(
                "company {} maps {capability} to unknown provider {other:?}; \
                 known providers are: simulated, resend, git, self",
                config.name
            ),
        },
    }
}

/// What a real send needs, parsed from the same `args` the simulator receives.
/// The Exec's request shape does not change when a company goes live — that is
/// §10.8's claim, and this struct is where it is either true or false.
#[derive(Debug, Deserialize)]
struct EmailArgs {
    to: String,
    subject: String,
    /// Plain-text body. Accepts `body` or `text`, because both appear in the
    /// personas the Exec has been writing against for two sprints and breaking
    /// its habit on the first live send would be a gratuitous failure.
    #[serde(alias = "text")]
    body: String,
    /// Optional display name for the sender; the address itself is owner
    /// configuration, never the agent's choice.
    #[serde(default)]
    from_name: Option<String>,
}

/// Send one email through Resend, host-side.
///
/// Returns the outcome object that becomes the receipt's `outcome`. Shape is
/// deliberately close to the simulator's so downstream reconciliation
/// (`reconcile::outcome_of`) reads both without special cases.
pub async fn resend_send(
    config: &CompanyConfig,
    args: &serde_json::Value,
    idempotency_key: &str,
) -> Result<serde_json::Value> {
    let parsed: EmailArgs = serde_json::from_value(args.clone()).context(
        "email.send needs {\"to\", \"subject\", \"body\"} — \
         the same arguments the simulator takes",
    )?;
    let api_key = crate::credential::resolve(config, "email.send")
        .context("resolving the Resend credential")?;
    let from_address = config
        .from_address
        .as_deref()
        .context("company config needs from_address to send real email")?;
    let from = match &parsed.from_name {
        Some(name) => format!("{name} <{from_address}>"),
        None => from_address.to_string(),
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        // Our key becomes the provider's, so a retry is deduplicated on both
        // sides of the boundary.
        .header("Idempotency-Key", idempotency_key)
        .json(&serde_json::json!({
            "from": from,
            "to": [parsed.to],
            "subject": parsed.subject,
            "text": parsed.body,
        }))
        .send()
        .await
        .context("POST https://api.resend.com/emails")?;

    let status = response.status();
    let body: serde_json::Value = response
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({ "note": "provider returned a non-JSON body" }));

    if !status.is_success() {
        // Honest status, per cross-layer §4.7: a provider refusal is reported
        // as a failure with the provider's own words, never smoothed into a
        // success or paraphrased into our vocabulary.
        return Ok(serde_json::json!({
            "status": "failed",
            "http_status": status.as_u16(),
            "provider_error": body,
            "note": format!("Resend refused the send with HTTP {}", status.as_u16()),
        }));
    }

    Ok(serde_json::json!({
        "status": "sent",
        "provider_message_id": body.get("id"),
        "to": parsed.to,
        "note": "accepted by Resend for delivery; \
                 delivery itself is a later webhook, not this receipt",
    }))
}

/// What a governed push needs. The company names its own repo and branch; the
/// remote is **probed from the repository**, never taken from the agent — an
/// agent that could name its own remote could push the owner's code anywhere.
#[derive(Debug, Deserialize)]
struct PushArgs {
    /// Directory under `/company/repos`.
    repo: String,
    /// Branch to push. Must already exist in the company's worktree.
    branch: String,
}

/// S04-T3. Push the company's branch to its own origin, host-side.
///
/// Three boundaries, all load-bearing:
///
/// * **The credential never enters the container.** The branch leaves as a
///   `git bundle` — one file, no network from inside — and the host pushes it.
///   `AC3` greps the container environment and the whole volume for the secret
///   and expects zero, exactly as S03-T4 verified for Resend.
/// * **The company never writes to the default branch.** Probed from the
///   repository's own refs, not from an API, and refused before anything moves.
/// * **No forge API.** The receipt carries a compare URL built by string
///   template. The owner opens and merges the pull request; that is the human
///   authority act, and it stays human.
pub async fn git_push(
    config: &CompanyConfig,
    args: &serde_json::Value,
    idempotency_key: &str,
) -> Result<serde_json::Value> {
    let parsed: PushArgs =
        serde_json::from_value(args.clone()).context("repo.push needs {\"repo\", \"branch\"}")?;
    let token =
        crate::credential::resolve(config, "repo.push").context("resolving the git credential")?;
    let container = crate::runtime::container_name(&config.name);
    let workdir = format!("/company/repos/{}", parsed.repo);

    let configured_remote = docker_capture(
        &container,
        &workdir,
        &["git", "remote", "get-url", "origin"],
    )
    .await?;
    let configured_remote = configured_remote.trim().to_string();
    // A repository may have been cloned from a URL containing credentials.
    // They are not needed for the receipt and must never survive into one.
    let remote = public_remote(&configured_remote);

    // The default branch comes from the repository's own remote refs. No API
    // call, so this works for any forge — and a repo that renames `main` cannot
    // silently become pushable.
    let base = docker_capture(
        &container,
        &workdir,
        &["git", "symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .await
    .context("probing the repository's default branch from origin/HEAD")?
    .trim()
    .rsplit('/')
    .next()
    .filter(|branch| !branch.is_empty())
    .context("origin/HEAD did not name a default branch")?
    .to_string();

    if parsed.branch == base {
        bail!(
            "refusing to push {}: it is the default branch of {remote}. Push a feature branch — \
             the owner opening and merging the pull request is the approval",
            parsed.branch
        );
    }

    // The commit being shipped, recorded in the receipt so the owner can check
    // what was pushed against what the company says it pushed.
    let sha = docker_capture(&container, &workdir, &["git", "rev-parse", &parsed.branch])
        .await
        .context("resolving the branch head")?
        .trim()
        .to_string();

    // Bundle out, push from the host.
    //
    // The bundle is **thin** — only the commits the branch adds on top of the
    // remote's own tip. The company's clone is shallow (depth 2 for a 140MB
    // product), so a full bundle cannot traverse parents and `git bundle` fails
    // outright; and a full one would be 58MB per push where this is kilobytes.
    let bundle_in_container = format!("/tmp/{}.bundle", parsed.branch.replace('/', "-"));
    let bundled = docker_capture(
        &container,
        &workdir,
        &[
            "git",
            "bundle",
            "create",
            &bundle_in_container,
            &format!("origin/{base}..{}", parsed.branch),
        ],
    )
    .await;
    if let Err(error) = bundled {
        // "Refusing to create empty bundle" is not a failure to report as one:
        // it means the branch adds nothing to the base, which is a no-op push.
        let text = format!("{error:#}");
        if text.contains("empty bundle") {
            return Ok(serde_json::json!({
                "status": "no_op",
                "note": format!("{} adds no commits on top of {base}", parsed.branch),
                "branch": parsed.branch,
                "commit": sha,
                "base": base,
                "remote": remote,
                "compare_url": compare_url(&remote, &base, &parsed.branch),
            }));
        }
        return Err(error.context("bundling the branch out of the container"));
    }

    // A scratch directory keyed by the idempotency key: unique per intent, and
    // re-entrant when a retry reaches here after a partial run.
    // An idempotency key is opaque caller input, not a path component. Hashing
    // preserves stable retry identity without allowing `../` or separators to
    // steer the cleanup outside the staging root.
    let staging = staging_dir(idempotency_key);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).with_context(|| format!("create {}", staging.display()))?;
    let bundle_on_host = staging.join("branch.bundle");

    let copy = tokio::process::Command::new("docker")
        .args([
            "cp",
            &format!("{container}:{bundle_in_container}"),
            &bundle_on_host.to_string_lossy(),
        ])
        .output()
        .await
        .context("docker cp the bundle")?;
    if !copy.status.success() {
        bail!(
            "docker cp failed: {}",
            String::from_utf8_lossy(&copy.stderr).trim()
        );
    }

    let authed = authenticated_remote(&configured_remote, &token)
        .with_context(|| format!("cannot authenticate to remote {remote:?} over https"))?;
    let bare = staging.join("repo.git");
    run_git(&staging, &["init", "-q", "--bare", "repo.git"], &token).await?;
    // Fetch the base shallowly to satisfy the thin bundle's prerequisite. This
    // is the only network read, and it costs one commit.
    run_git(
        &bare,
        &[
            "fetch",
            "-q",
            "--depth=1",
            &authed,
            &format!("{base}:refs/heads/{base}"),
        ],
        &token,
    )
    .await
    .context("fetching the base commit the bundle depends on")?;
    run_git(
        &bare,
        &[
            "fetch",
            "-q",
            &bundle_on_host.to_string_lossy(),
            &format!("{}:refs/heads/{}", parsed.branch, parsed.branch),
        ],
        &token,
    )
    .await
    .context("unbundling the branch")?;

    let push = tokio::process::Command::new("git")
        .args([
            "push",
            &authed,
            &format!("{}:refs/heads/{}", parsed.branch, parsed.branch),
        ])
        .current_dir(&bare)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .context("git push")?;
    let _ = std::fs::remove_dir_all(&staging);

    if !push.status.success() {
        // Honest status (cross-layer §4.7): a refusal is reported as what it is.
        return Ok(serde_json::json!({
            "status": "failed",
            "error": redact(&push.stderr, &token),
            "branch": parsed.branch,
            "commit": sha,
            "base": base,
            "remote": remote,
            "compare_url": compare_url(&remote, &base, &parsed.branch),
        }));
    }

    Ok(serde_json::json!({
        "status": "pushed",
        "branch": parsed.branch,
        "commit": sha,
        "base": base,
        "remote": remote,
        // The prepared last mile: the owner clicks this, reads the diff, opens
        // and merges. No API call made it, and none is needed to use it.
        "compare_url": compare_url(&remote, &base, &parsed.branch),
    }))
}

/// Run a git command, redacting the token from any failure text.
async fn run_git(cwd: &std::path::Path, args: &[&str], token: &str) -> Result<()> {
    let out = tokio::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .context("spawn git")?;
    if !out.status.success() {
        bail!("git {} failed: {}", args[0], redact(&out.stderr, token));
    }
    Ok(())
}

/// Never let a token reach a log, an event, or a receipt through an error path.
fn redact(stderr: &[u8], token: &str) -> String {
    String::from_utf8_lossy(stderr)
        .replace(token, "***")
        .chars()
        .take(400)
        .collect()
}

/// Turn an origin URL into one the host can push to with a token.
///
/// Only https is supported: an ssh remote would need a key, which is a
/// different credential shape and not one this sprint holds.
fn authenticated_remote(remote: &str, token: &str) -> Option<String> {
    let rest = remote.strip_prefix("https://")?;
    // Strip any embedded credentials already present rather than nesting them.
    let rest = rest.rsplit('@').next()?;
    Some(format!("https://x-access-token:{token}@{rest}"))
}

/// The remote shape that may enter a receipt, error, or compare link.
fn public_remote(remote: &str) -> String {
    let Some(rest) = remote.strip_prefix("https://") else {
        return remote.to_string();
    };
    let rest = rest.rsplit('@').next().unwrap_or(rest);
    format!("https://{rest}")
}

/// Stable scratch location for one intent, with no caller-controlled path
/// component. This directory may be removed before a retry is staged.
fn staging_dir(idempotency_key: &str) -> std::path::PathBuf {
    let digest = sha2::Sha256::digest(idempotency_key.as_bytes());
    std::env::temp_dir().join(format!("restless-push-{digest:x}"))
}

/// A compare link for the common forges, by string template.
///
/// This is deliberately **data, not an adapter**: no request is made, no
/// credential used, and an unrecognised host yields the remote itself rather
/// than a guess. Adding a forge here is one line and no behaviour.
fn compare_url(remote: &str, base: &str, branch: &str) -> String {
    let Some(rest) = remote.strip_prefix("https://") else {
        return remote.to_string();
    };
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    if rest.starts_with("github.com/") {
        return format!("https://{rest}/compare/{base}...{branch}?expand=1");
    }
    if rest.starts_with("gitlab.com/") {
        return format!(
            "https://{rest}/-/merge_requests/new?merge_request%5Bsource_branch%5D={branch}"
        );
    }
    format!("https://{rest}")
}

/// Run one command in the company container and return stdout.
async fn docker_capture(container: &str, workdir: &str, command: &[&str]) -> Result<String> {
    let mut args: Vec<String> = vec![
        "exec".into(),
        "-u".into(),
        "company".into(),
        "-w".into(),
        workdir.into(),
    ];
    args.push(container.to_string());
    args.extend(command.iter().map(|part| (*part).to_string()));
    let output = tokio::process::Command::new("docker")
        .args(&args)
        .output()
        .await
        .context("spawn docker exec")?;
    if !output.status.success() {
        bail!(
            "{} failed: {}",
            command.first().copied().unwrap_or("command"),
            String::from_utf8_lossy(&output.stderr)
                .chars()
                .take(400)
                .collect::<String>()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(entries: &[(&str, &str)]) -> CompanyConfig {
        let mut providers = std::collections::BTreeMap::new();
        for (capability, provider) in entries {
            providers.insert((*capability).to_string(), (*provider).to_string());
        }
        CompanyConfig {
            name: "aris".to_string(),
            mission: String::new(),
            spend_ceiling_usd: 30.0,
            model: "moonshot/kimi-k3".to_string(),
            providers,
            from_address: None,
            credentials: std::collections::BTreeMap::new(),
            approved_parties: Vec::new(),
        }
    }

    /// S04-T1's second lock. `clone_config` strips the dispatch table, so a
    /// `_test` company normally has nothing real to find. This asserts the
    /// case that stripping cannot cover: a `_test` config carrying a real
    /// entry, hand-written or edited. It still cannot reach the world.
    ///
    /// This test exists because the first contamination happened exactly this
    /// way — the live company was the convenient one to try things on.
    #[test]
    fn a_test_company_cannot_reach_a_real_provider_even_when_configured_to() {
        let mut config = config_with(&[("email.send", "resend")]);
        config.name = "aris_test".to_string();
        assert_eq!(
            resolve(&config, "email.send").unwrap(),
            Provider::Simulated,
            "a _test company must not resolve a real provider, whatever its config says"
        );
        // The same config under a live name does reach the provider — otherwise
        // this test would pass for the wrong reason.
        config.name = "aris".to_string();
        assert_eq!(resolve(&config, "email.send").unwrap(), Provider::Resend);
    }

    /// The structural guarantee S03-T7 rests on: a company with no entry for a
    /// capability cannot reach a real provider, so a `_test` company's worst
    /// case is a simulated send. This is not a rule someone must remember — it
    /// is the absence of a table row.
    #[test]
    fn no_entry_means_simulated() {
        let config = config_with(&[]);
        assert_eq!(resolve(&config, "email.send").unwrap(), Provider::Simulated);
        // And a live company's OTHER capabilities stay simulated too.
        let live = config_with(&[("email.send", "resend")]);
        assert_eq!(resolve(&live, "email.send").unwrap(), Provider::Resend);
        assert_eq!(resolve(&live, "web.deploy").unwrap(), Provider::Simulated);
    }

    /// A typo must not silently simulate while the run reports success. The
    /// whole value of the receipt is that `provider` tells you which world you
    /// were in; a misspelling that falls back would make it lie.
    #[test]
    fn an_unknown_provider_fails_closed() {
        let config = config_with(&[("email.send", "resned")]);
        let error = resolve(&config, "email.send").unwrap_err().to_string();
        assert!(error.contains("resned"), "{error}");
        assert!(error.contains("simulated, resend"), "{error}");
    }

    /// §10.8's claim in a test: the arguments the simulator has been taking for
    /// two sprints parse for the real adapter unchanged.
    #[test]
    fn the_simulator_argument_shape_parses_for_the_real_provider() {
        let args = serde_json::json!({
            "to": "yaillives@gmail.com",
            "subject": "Your free 11+ practice paper",
            "body": "Hello — here is the sample you asked for."
        });
        let parsed: EmailArgs = serde_json::from_value(args).expect("simulator shape must parse");
        assert_eq!(parsed.to, "yaillives@gmail.com");
        // And the `text` alias, which the personas also use.
        let aliased = serde_json::json!({ "to": "a@b.c", "subject": "s", "text": "t" });
        assert!(serde_json::from_value::<EmailArgs>(aliased).is_ok());
    }

    /// The remote we push to is derived from the repository's own origin, and
    /// a token must never be nested inside one that already carries
    /// credentials — that produces a URL that leaks one secret while failing to
    /// use the other.
    #[test]
    fn an_authenticated_remote_never_nests_credentials() {
        assert_eq!(
            authenticated_remote("https://github.com/BlueprintLabIO/study.git", "tok"),
            Some("https://x-access-token:tok@github.com/BlueprintLabIO/study.git".into())
        );
        // Already carries a credential: the old one is replaced, not wrapped.
        assert_eq!(
            authenticated_remote("https://someone:old@github.com/o/r.git", "tok"),
            Some("https://x-access-token:tok@github.com/o/r.git".into())
        );
        // ssh needs a key, which is a different credential shape than this
        // sprint holds. Refused rather than silently mangled.
        assert_eq!(authenticated_remote("git@github.com:o/r.git", "tok"), None);
        assert_eq!(
            authenticated_remote("ssh://git@github.com/o/r.git", "tok"),
            None
        );
    }

    #[test]
    fn receipt_remote_strips_embedded_credentials() {
        assert_eq!(
            public_remote("https://someone:old@github.com/o/r.git"),
            "https://github.com/o/r.git"
        );
        assert_eq!(
            public_remote("git@github.com:o/r.git"),
            "git@github.com:o/r.git"
        );
    }

    #[test]
    fn an_idempotency_key_never_becomes_a_path() {
        let staged = staging_dir("../../escape/attempt");
        assert_eq!(staged.parent(), Some(std::env::temp_dir().as_path()));
        assert!(!staged.to_string_lossy().contains("escape"));
    }

    /// The compare link is data, not an adapter: no request, no credential, and
    /// an unrecognised forge yields the remote rather than a guessed URL.
    #[test]
    fn the_compare_url_is_a_template_and_never_carries_a_secret() {
        assert_eq!(
            compare_url(
                "https://github.com/BlueprintLabIO/study.git",
                "main",
                "feat/x"
            ),
            "https://github.com/BlueprintLabIO/study/compare/main...feat/x?expand=1"
        );
        assert!(
            compare_url("https://gitlab.com/o/r.git", "main", "feat/x").contains("merge_requests")
        );
        // Unknown forge: the remote itself, not an invented path.
        assert_eq!(
            compare_url("https://git.example.com/o/r.git", "main", "b"),
            "https://git.example.com/o/r"
        );
        // An ssh remote has no https compare page to offer.
        assert_eq!(
            compare_url("git@github.com:o/r.git", "main", "b"),
            "git@github.com:o/r.git"
        );
    }

    /// A token must never survive into an error string, which is the one path
    /// that reaches a log and an event body.
    #[test]
    fn a_failing_push_never_echoes_the_token() {
        let token = "ghp_supersecretvalue";
        let stderr =
            format!("fatal: could not read from https://x-access-token:{token}@github.com");
        let rendered = redact(stderr.as_bytes(), token);
        assert!(!rendered.contains(token), "token leaked: {rendered}");
        assert!(rendered.contains("***"));
    }
}
