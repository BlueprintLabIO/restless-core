//! Runtime workspace coordinates and bounded Git observations.
//!
//! OrgIntel records the locator and exact version of an Attempt's evidence;
//! the Runtime owns the checkout. These helpers deliberately use ordinary Git
//! worktrees instead of inventing an artifact or custody lifecycle.

use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::{bail, Context as _, Result};
use restless_orgintel::WorkRow;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::runtime::{self, CompanyConfig};

/// Repo and explicit worktree segments cross the Runtime boundary.
pub(crate) fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 32
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Bounded Runtime evidence around one cognitive process. Git remains the
/// source of productive truth; this observation only tells a lead whether the
/// same preserved worktree changed without copying files or content into
/// OrgIntel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkspaceObservation {
    pub(crate) workdir: String,
    pub(crate) source_commit: Option<String>,
    pub(crate) source_tree: Option<String>,
    pub(crate) status_digest: Option<String>,
    pub(crate) dirty_entries: usize,
}

impl WorkspaceObservation {
    pub(crate) fn changed_since(&self, start: &Self) -> bool {
        self.source_commit.is_some()
            && start.source_commit.is_some()
            && (self.source_commit != start.source_commit
                || self.source_tree != start.source_tree
                || self.status_digest != start.status_digest)
    }

    pub(crate) fn fingerprint(&self) -> Option<String> {
        self.source_commit.as_ref().map(|commit| {
            format!(
                "{:x}",
                Sha256::digest(
                    format!(
                        "{commit}\\n{}\\n{}\\n{}",
                        self.source_tree.as_deref().unwrap_or(""),
                        self.status_digest.as_deref().unwrap_or(""),
                        self.dirty_entries
                    )
                    .as_bytes()
                )
            )
        })
    }

    pub(crate) fn compact(&self) -> String {
        match self.source_commit.as_deref() {
            Some(commit) => format!(
                "HEAD {} with {} changed entries",
                commit.chars().take(12).collect::<String>(),
                self.dirty_entries
            ),
            None => "no repository observation available".into(),
        }
    }
}

#[derive(Debug)]
struct CommandOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl From<std::process::Output> for CommandOutput {
    fn from(output: std::process::Output) -> Self {
        Self {
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        }
    }
}

type CommandFuture = Pin<Box<dyn Future<Output = Result<CommandOutput>> + Send>>;
type CompanyCommand = dyn Fn(Vec<String>) -> CommandFuture + Send + Sync;

fn docker_company_command(container: &str) -> Arc<CompanyCommand> {
    let container = container.to_string();
    Arc::new(move |args| {
        let container = container.clone();
        Box::pin(async move {
            let output = tokio::process::Command::new("docker")
                .args(["exec", "-u", "company", &container])
                .args(args)
                .output()
                .await
                .context("run Runtime workspace command")?;
            Ok(output.into())
        })
    })
}

fn docker_root_command(container: &str) -> Arc<CompanyCommand> {
    let container = container.to_string();
    Arc::new(move |args| {
        let container = container.clone();
        Box::pin(async move {
            let output = tokio::process::Command::new("docker")
                .args([
                    "exec",
                    "-u",
                    "root",
                    "-e",
                    "GIT_CONFIG_COUNT=1",
                    "-e",
                    "GIT_CONFIG_KEY_0=safe.directory",
                    "-e",
                    "GIT_CONFIG_VALUE_0=*",
                    &container,
                ])
                .args(args)
                .output()
                .await
                .context("run root-owned Runtime custody command")?;
            Ok(output.into())
        })
    })
}

/// Observe a Git checkout without writing to it. Kept independent from Docker
/// so the exact preparation logic can be exercised against a real local Git
/// repository in the focused integrity scenario below.
async fn observe_git_workspace(command: &CompanyCommand, workdir: &str) -> WorkspaceObservation {
    let mut observation = WorkspaceObservation {
        workdir: workdir.to_string(),
        ..WorkspaceObservation::default()
    };
    let head = command(vec![
        "git".into(),
        "-C".into(),
        workdir.into(),
        "rev-parse".into(),
        "--verify".into(),
        "HEAD".into(),
    ])
    .await;
    let Ok(head) = head else {
        return observation;
    };
    if !head.success {
        return observation;
    }
    let commit = String::from_utf8_lossy(&head.stdout).trim().to_string();
    if commit.is_empty() {
        return observation;
    }
    observation.source_commit = Some(commit);

    let tree = command(vec![
        "git".into(),
        "-C".into(),
        workdir.into(),
        "rev-parse".into(),
        "--verify".into(),
        "HEAD^{tree}".into(),
    ])
    .await;
    if let Ok(tree) = tree {
        if tree.success {
            let tree = String::from_utf8_lossy(&tree.stdout).trim().to_string();
            if !tree.is_empty() {
                observation.source_tree = Some(tree);
            }
        }
    }

    let status = command(vec![
        "git".into(),
        "-C".into(),
        workdir.into(),
        "status".into(),
        "--porcelain=v1".into(),
        "-z".into(),
    ])
    .await;
    if let Ok(status) = status {
        if status.success {
            observation.dirty_entries = status
                .stdout
                .split(|byte| *byte == 0)
                .filter(|entry| !entry.is_empty())
                .count();
            observation.status_digest = Some(format!("{:x}", Sha256::digest(&status.stdout)));
        }
    }
    observation
}

pub(crate) async fn observe_workspace(container: &str, workdir: &str) -> WorkspaceObservation {
    let command = docker_company_command(container);
    observe_git_workspace(&*command, workdir).await
}

pub(crate) fn workdir_for(work: &WorkRow) -> Result<String> {
    if work.repo.is_none() {
        return Ok("/company".into());
    }
    let short = work.id.simple().to_string();
    let generated = format!("work-{}-r{}", &short[..12], work.revision);
    let leaf = work.worktree.as_deref().unwrap_or(&generated);
    if !valid_slug(leaf) {
        bail!("invalid Work worktree {leaf:?}");
    }
    Ok(format!("/company/worktrees/{leaf}"))
}

pub(crate) async fn recorded_start_observation(
    org: &restless_orgintel::OrgIntel,
    attempt_id: Uuid,
    workdir: &str,
) -> WorkspaceObservation {
    org.find_event_body(
        "attempt_process_started",
        "attempt_id",
        &attempt_id.to_string(),
    )
    .await
    .ok()
    .flatten()
    .and_then(|body| serde_json::from_value(body["workspace"].clone()).ok())
    .unwrap_or_else(|| WorkspaceObservation {
        workdir: workdir.to_string(),
        ..WorkspaceObservation::default()
    })
}

/// Create or reuse the workspace recorded on Work. Git remains the source of
/// file truth; OrgIntel stores only the path and exact artifact versions.
pub(crate) async fn ensure_worktree(
    config: &CompanyConfig,
    work: &WorkRow,
    effective_base_ref: Option<&str>,
    attempt_id: Uuid,
    org: &restless_orgintel::OrgIntel,
) -> Result<String> {
    let container = runtime::container_name(&config.name);
    let repo = work.repo.as_deref().context("Work repo is missing")?;
    let path = workdir_for(work)?;
    let leaf = path
        .rsplit('/')
        .next()
        .context("Work worktree path has no leaf")?;
    let repo_path = format!("/company/repos/{repo}");
    let runtime_root = format!("/company/run/attempts/{attempt_id}");
    let normalise = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "0:0",
            &container,
            "sh",
            "-c",
            "set -eu; mkdir -p \"$2/cache\" \"$2/tmp\" \"$2/godot\" /company/worktrees /company/reviews; chown 2000:2000 \"$1\" \"$2\" /company/worktrees /company/reviews; if test -e \"$3\"; then chown -R 2000:2000 \"$3\"; fi",
            "restless-workspace",
            &repo_path,
            &runtime_root,
            &path,
        ])
        .output()
        .await
        .context("normalise Attempt workspace ownership")?;
    if !normalise.status.success() {
        bail!(
            "workspace ownership normalisation failed: {}",
            String::from_utf8_lossy(&normalise.stderr)
        );
    }

    let image = tokio::process::Command::new("docker")
        .args(["inspect", "--format", "{{.Image}}", &container])
        .output()
        .await
        .context("fingerprint Runtime image")?;
    if !image.status.success() {
        bail!("could not fingerprint Runtime image");
    }
    let environment_fingerprint = format!(
        "{:x}",
        Sha256::digest(String::from_utf8_lossy(&image.stdout).trim().as_bytes())
    );

    let attempt = org
        .list_work_attempts(Some(work.id))
        .await?
        .into_iter()
        .find(|attempt| attempt.id == attempt_id)
        .context("Attempt disappeared before workspace materialisation")?;
    if attempt.requested_source_ref.as_deref() != effective_base_ref {
        bail!(
            "Attempt requested source {:?}, but dispatch supplied {:?}",
            attempt.requested_source_ref,
            effective_base_ref
        );
    }
    let requested_source_ref = attempt.requested_source_ref.clone();
    let (exact_commit, exact_tree) = if attempt.materialized_at.is_some() {
        let exact_commit = attempt
            .source_commit
            .context("materialized Attempt lost its exact source commit")?;
        let exact_tree = attempt
            .source_tree
            .context("materialized Attempt lost its exact source tree")?;
        // Rebinding is an idempotent verification. A changed Runtime image or
        // source coordinate fails before any workspace or model process runs.
        org.bind_attempt_execution_coordinates(
            attempt_id,
            requested_source_ref.as_deref(),
            Some(&exact_commit),
            Some(&exact_tree),
            &environment_fingerprint,
        )
        .await?;
        (exact_commit, exact_tree)
    } else {
        let requested = requested_source_ref.as_deref().unwrap_or("HEAD");
        let resolved = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &container,
                "git",
                "-C",
                &repo_path,
                "rev-parse",
                "--verify",
                &format!("{requested}^{{commit}}"),
            ])
            .output()
            .await
            .context("resolve frozen Attempt source")?;
        if !resolved.status.success() {
            bail!(
                "requested source {requested:?} is not an exact reachable commit: {}",
                String::from_utf8_lossy(&resolved.stderr).trim()
            );
        }
        let exact_commit = String::from_utf8_lossy(&resolved.stdout).trim().to_string();
        let tree = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &container,
                "git",
                "-C",
                &repo_path,
                "rev-parse",
                "--verify",
                &format!("{exact_commit}^{{tree}}"),
            ])
            .output()
            .await
            .context("resolve frozen Attempt tree")?;
        if !tree.status.success() {
            bail!("could not resolve tree for source {exact_commit}");
        }
        let exact_tree = String::from_utf8_lossy(&tree.stdout).trim().to_string();
        // Freeze the resolved symbolic ref before creating or reusing a
        // mutable worktree. Later branch movement cannot change this Attempt.
        org.bind_attempt_execution_coordinates(
            attempt_id,
            requested_source_ref.as_deref(),
            Some(&exact_commit),
            Some(&exact_tree),
            &environment_fingerprint,
        )
        .await?;
        (exact_commit, exact_tree)
    };

    let exists = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container,
            "test",
            "-f",
            &format!("{path}/.git"),
        ])
        .status()
        .await
        .context("probe Work worktree")?;
    if !exists.success() {
        let branch = format!("work/{leaf}");
        let output = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &container,
                "git",
                "-C",
                &repo_path,
                "worktree",
                "add",
                &path,
                "-b",
                &branch,
                &exact_commit,
            ])
            .output()
            .await
            .context("create Work worktree")?;
        if !output.status.success() {
            let reuse = tokio::process::Command::new("docker")
                .args([
                    "exec", "-u", "company", &container, "git", "-C", &repo_path, "worktree",
                    "add", &path, &branch,
                ])
                .output()
                .await
                .context("reuse Work branch")?;
            if !reuse.status.success() {
                bail!(
                    "worktree setup failed: {}",
                    String::from_utf8_lossy(&reuse.stderr)
                );
            }
        }
    }

    let observed = observe_workspace(&container, &path).await;
    if observed.source_commit.as_deref() != Some(exact_commit.as_str())
        || observed.source_tree.as_deref() != Some(exact_tree.as_str())
    {
        bail!(
            "materialized workspace differs from requested source: requested {exact_commit}/{exact_tree}, observed {}/{}",
            observed.source_commit.as_deref().unwrap_or("missing"),
            observed.source_tree.as_deref().unwrap_or("missing")
        );
    }
    let cache = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container,
            "sh",
            "-c",
            "set -eu; if [ -f \"$1/project.godot\" ]; then mkdir -p \"$2/godot\"; if [ ! -e \"$1/.godot\" ] && [ ! -L \"$1/.godot\" ]; then ln -s \"$2/godot\" \"$1/.godot\"; fi; common=$(git -C \"$1\" rev-parse --git-common-dir); mkdir -p \"$common/info\"; grep -qxF .godot \"$common/info/exclude\" 2>/dev/null || printf '.godot\\n' >> \"$common/info/exclude\"; fi",
            "restless-cache",
            &path,
            &runtime_root,
        ])
        .output()
        .await
        .context("externalise Attempt caches")?;
    if !cache.status.success() {
        bail!(
            "could not externalise Attempt cache: {}",
            String::from_utf8_lossy(&cache.stderr)
        );
    }
    org.bind_attempt_execution_coordinates(
        attempt_id,
        requested_source_ref.as_deref(),
        Some(&exact_commit),
        Some(&exact_tree),
        &environment_fingerprint,
    )
    .await?;
    Ok(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedReviewCopy {
    pub(crate) workdir: String,
    pub(crate) source_commit: String,
    pub(crate) content_digest: String,
    pub(crate) file_count: usize,
    pub(crate) access_probed: bool,
    pub(crate) source_before: WorkspaceObservation,
    pub(crate) source_after: WorkspaceObservation,
}

fn review_workdir_for(source_commit: &str) -> String {
    format!("/company/reviews/git/{source_commit}")
}

fn valid_commit(commit: &str) -> bool {
    commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_integration_branch(branch: &str) -> bool {
    !branch.is_empty()
        && branch.len() <= 255
        && !branch.starts_with(['-', '/'])
        && !branch.ends_with(['/', '.'])
        && !branch.contains("..")
        && !branch.contains("//")
        && !branch.contains("@{")
        && branch
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
}

fn output_error(action: &str, output: &CommandOutput) -> anyhow::Error {
    anyhow::anyhow!(
        "{action}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

async fn require_success(
    command: &CompanyCommand,
    argv: Vec<String>,
    action: &str,
) -> Result<CommandOutput> {
    let output = command(argv).await?;
    if !output.success {
        return Err(output_error(action, &output));
    }
    Ok(output)
}

/// Fast-forward one checked-out shared branch to an exact accepted Attempt
/// commit. The update runs in the branch's own checkout so Git updates its
/// index and files together; moving the ref from another worktree would leave
/// this checkout falsely dirty and is deliberately refused.
async fn promote_integration_commit_with(
    command: &CompanyCommand,
    repo_path: &str,
    branch: &str,
    source_commit: &str,
) -> Result<WorkspaceObservation> {
    if !valid_integration_branch(branch) || !valid_commit(source_commit) {
        bail!("integration promotion needs a bounded branch and exact Git commit");
    }
    let before = observe_git_workspace(command, repo_path).await;
    if before.dirty_entries != 0 {
        bail!(
            "integration checkout {repo_path} has {} changed entries; refusing to overwrite shared state",
            before.dirty_entries
        );
    }
    let current_branch = require_success(
        command,
        vec![
            "git".into(),
            "-C".into(),
            repo_path.into(),
            "symbolic-ref".into(),
            "--quiet".into(),
            "--short".into(),
            "HEAD".into(),
        ],
        "read checked-out integration branch",
    )
    .await?;
    if String::from_utf8_lossy(&current_branch.stdout).trim() != branch {
        bail!("integration checkout {repo_path} is not on declared branch {branch:?}");
    }
    require_success(
        command,
        vec![
            "git".into(),
            "-C".into(),
            repo_path.into(),
            "cat-file".into(),
            "-e".into(),
            format!("{source_commit}^{{commit}}"),
        ],
        "verify integration candidate commit",
    )
    .await?;
    require_success(
        command,
        vec![
            "git".into(),
            "-C".into(),
            repo_path.into(),
            "merge-base".into(),
            "--is-ancestor".into(),
            "HEAD".into(),
            source_commit.into(),
        ],
        "require fast-forward integration history",
    )
    .await?;
    require_success(
        command,
        vec![
            "git".into(),
            "-C".into(),
            repo_path.into(),
            "merge".into(),
            "--ff-only".into(),
            "--no-edit".into(),
            source_commit.into(),
        ],
        "fast-forward accepted integration commit",
    )
    .await?;
    let after = observe_git_workspace(command, repo_path).await;
    if after.source_commit.as_deref() != Some(source_commit) || after.dirty_entries != 0 {
        bail!("integration promotion did not leave the exact clean accepted commit");
    }
    Ok(after)
}

pub(crate) async fn promote_integration_commit(
    container: &str,
    repo: &str,
    branch: &str,
    source_commit: &str,
) -> Result<WorkspaceObservation> {
    if !valid_slug(repo) {
        bail!("invalid integration repository {repo:?}");
    }
    let command = docker_company_command(container);
    promote_integration_commit_with(
        &*command,
        &format!("/company/repos/{repo}"),
        branch,
        source_commit,
    )
    .await
}

/// Remove only transient state owned by one exact Attempt. The source
/// worktree and Git checkpoint remain available for recovery/review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AttemptCleanupReceipt {
    pub(crate) attempt_id: Uuid,
    pub(crate) removed_paths: Vec<String>,
    pub(crate) residue_count: usize,
}

pub(crate) async fn cleanup_attempt_runtime(
    container: &str,
    workdir: &str,
    attempt_id: Uuid,
) -> Result<AttemptCleanupReceipt> {
    let attempt = attempt_id.to_string();
    let attempt_runtime = format!("/company/run/attempts/{attempt}");
    let gate_runtime = format!("/company/run/gates/{attempt}");
    let godot_link = format!("{workdir}/.godot");
    let output = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "root",
            container,
            "sh",
            "-lc",
            "set -eu; if test -L \"$1/.godot\"; then rm -f \"$1/.godot\"; fi; rm -rf \"/company/run/attempts/$2\" \"/company/run/gates/$2\"; test ! -e \"$1/.godot\"; test ! -L \"$1/.godot\"; test ! -e \"/company/run/attempts/$2\"; test ! -e \"/company/run/gates/$2\"",
            "attempt-cleanup",
            workdir,
            &attempt,
        ])
        .output()
        .await
        .context("clean exact Attempt Runtime state")?;
    if !output.status.success() {
        bail!(
            "clean exact Attempt Runtime state: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(AttemptCleanupReceipt {
        attempt_id,
        removed_paths: vec![godot_link, attempt_runtime, gate_runtime],
        residue_count: 0,
    })
}

async fn review_copy_is_exact(
    command: &CompanyCommand,
    review_workdir: &str,
    source_commit: &str,
) -> Result<()> {
    let observation = observe_git_workspace(command, review_workdir).await;
    if observation.source_commit.as_deref() != Some(source_commit) {
        bail!("review copy {review_workdir} is not the completed Attempt commit {source_commit}");
    }
    if observation.dirty_entries != 0 {
        bail!(
            "review copy {review_workdir} already has {} changed entries; refusing to overwrite review evidence",
            observation.dirty_entries
        );
    }
    Ok(())
}

async fn review_copy_file_count(command: &CompanyCommand, review_workdir: &str) -> Result<usize> {
    let output = require_success(
        command,
        vec![
            "git".into(),
            "-C".into(),
            review_workdir.into(),
            "ls-tree".into(),
            "-r".into(),
            "--name-only".into(),
            "HEAD".into(),
        ],
        "enumerate declared review files",
    )
    .await?;
    Ok(String::from_utf8(output.stdout)
        .context("review manifest paths are not UTF-8")?
        .lines()
        .count())
}

async fn probe_review_copy_as_reviewer(
    command: &CompanyCommand,
    review_workdir: &str,
    alias: &str,
    source_commit: &str,
    source_tree: &str,
) -> Result<usize> {
    let probe = require_success(
        command,
        vec![
            "bash".into(),
            "-c".into(),
            "set -eu; git_review() { git -c safe.directory=\"$1\" -C \"$1\" \"${@:2}\"; }; test \"$(readlink \"$2\")\" = \"$1\"; test ! -w \"$1\"; test \"$(git_review \"$1\" rev-parse HEAD)\" = \"$3\"; test \"$(git_review \"$1\" rev-parse 'HEAD^{tree}')\" = \"$4\"; git_review \"$1\" diff --quiet --ignore-submodules --; git_review \"$1\" diff --cached --quiet --ignore-submodules --; git_review \"$1\" ls-tree -r -z --name-only HEAD | while IFS= read -r -d '' path; do test -r \"$1/$path\"; test ! -w \"$1/$path\"; done; git_review \"$1\" ls-tree -r --name-only HEAD | wc -l".into(),
            "review-access-probe".into(),
            review_workdir.into(),
            alias.into(),
            source_commit.into(),
            source_tree.into(),
        ],
        "probe review evidence as reviewer identity",
    )
    .await?;
    String::from_utf8(probe.stdout)
        .context("review access probe count is not UTF-8")?
        .trim()
        .parse::<usize>()
        .context("review access probe did not return an exact file count")
}

pub(crate) async fn probe_hardened_review_copy(
    container: &str,
    review_workdir: &str,
    alias: &str,
    source_commit: &str,
    source_tree: &str,
) -> Result<usize> {
    let reviewer_command = docker_company_command(container);
    probe_review_copy_as_reviewer(
        &*reviewer_command,
        review_workdir,
        alias,
        source_commit,
        source_tree,
    )
    .await
}

/// Prepare one detached Git worktree for a completed Attempt review. The
/// exact same command-level routine is invoked in a real local-Git scenario
/// below and through the company Runtime wrapper, so it has no hidden path
/// that can write the source checkout.
async fn prepare_review_copy_with(
    command: &CompanyCommand,
    source_workdir: &str,
    repo_path: &str,
    review_workdir: &str,
    source_commit: &str,
) -> Result<PreparedReviewCopy> {
    if !valid_commit(source_commit) {
        bail!("completed Attempt has no exact Git commit for review preparation");
    }
    let source_before = observe_git_workspace(command, source_workdir).await;
    if source_before.source_commit.as_deref() != Some(source_commit) {
        bail!(
            "completed Attempt source {source_workdir} no longer points at recorded commit {source_commit}; review target is unavailable"
        );
    }

    let exists = command(vec!["test".into(), "-e".into(), review_workdir.into()]).await?;
    if exists.success {
        review_copy_is_exact(command, review_workdir, source_commit).await?;
    } else {
        let parent = review_workdir
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .context("review worktree has no parent")?;
        require_success(
            command,
            vec!["mkdir".into(), "-p".into(), parent.into()],
            "create review worktree directory",
        )
        .await?;
        require_success(
            command,
            vec![
                "git".into(),
                "-C".into(),
                repo_path.into(),
                "cat-file".into(),
                "-e".into(),
                format!("{source_commit}^{{commit}}"),
            ],
            "verify recorded review commit",
        )
        .await?;
        require_success(
            command,
            vec![
                "git".into(),
                "-C".into(),
                repo_path.into(),
                "worktree".into(),
                "add".into(),
                "--detach".into(),
                review_workdir.into(),
                source_commit.into(),
            ],
            "prepare detached review copy",
        )
        .await?;
        review_copy_is_exact(command, review_workdir, source_commit).await?;
    }

    let file_count = review_copy_file_count(command, review_workdir).await?;
    let source_after = observe_git_workspace(command, source_workdir).await;
    if source_after != source_before {
        bail!(
            "completed Attempt source {source_workdir} changed while review evidence was prepared; review target is unavailable"
        );
    }
    let source_tree = source_after
        .source_tree
        .clone()
        .context("completed Attempt commit has no exact Git tree")?;
    Ok(PreparedReviewCopy {
        workdir: review_workdir.to_string(),
        source_commit: source_commit.to_string(),
        content_digest: format!("git-tree:{source_tree}"),
        file_count,
        access_probed: false,
        source_before,
        source_after,
    })
}

/// Use a detached ordinary Git worktree for review evidence. A failed
/// preparation is returned as an error to the caller so it can present an
/// unavailable review target rather than falling back to the source checkout.
pub(crate) async fn prepare_review_copy(
    container: &str,
    work: &WorkRow,
    attempt_id: Uuid,
    source_commit: &str,
) -> Result<PreparedReviewCopy> {
    let repo = work
        .repo
        .as_deref()
        .context("completed Work has no repository-bound source to prepare")?;
    if !valid_slug(repo) {
        bail!("invalid completed Work repo {repo:?}");
    }
    let source_workdir = workdir_for(work)?;
    let review_workdir = review_workdir_for(source_commit);
    let repo_path = format!("/company/repos/{repo}");
    // Custody preparation runs as root so the producer/reviewer identity can
    // never replace the content-addressed directory or its alias. Ordinary
    // Git remains the source of truth; this is a hardened detached worktree,
    // not a parallel artifact lifecycle.
    let root_command = docker_root_command(container);
    let mut prepared = prepare_review_copy_with(
        &*root_command,
        &source_workdir,
        &repo_path,
        &review_workdir,
        source_commit,
    )
    .await?;
    let alias = format!("/company/reviews/by-attempt/{}", attempt_id.simple());
    require_success(
        &*root_command,
        vec![
            "sh".into(),
            "-c".into(),
            "set -eu; mkdir -p /company/reviews/by-attempt; if test -L \"$2\"; then test \"$(readlink \"$2\")\" = \"$1\"; elif test -e \"$2\"; then exit 71; else ln -s \"$1\" \"$2\"; fi; chown -R 0:2000 \"$1\"; find \"$1\" -type d -exec chmod 0550 {} +; find \"$1\" -type f -exec chmod 0440 {} +; chown 0:2000 /company/reviews/by-attempt; chmod 0550 /company/reviews/by-attempt; chown -h 0:2000 \"$2\"".into(),
            "harden-review-evidence".into(),
            review_workdir.clone(),
            alias.clone(),
        ],
        "harden detached review evidence",
    )
    .await?;
    let source_tree = prepared
        .source_after
        .source_tree
        .as_deref()
        .context("prepared review copy has no exact source tree")?;
    let probed_files = probe_hardened_review_copy(
        container,
        &prepared.workdir,
        &alias,
        &prepared.source_commit,
        source_tree,
    )
    .await?;
    if probed_files != prepared.file_count {
        bail!(
            "review access probe observed {probed_files} files but custody declared {}",
            prepared.file_count
        );
    }
    prepared.access_probed = true;
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        observe_git_workspace, prepare_review_copy_with, probe_review_copy_as_reviewer,
        promote_integration_commit_with, CommandFuture, CommandOutput, CompanyCommand,
    };

    fn local_git_command() -> Arc<CompanyCommand> {
        Arc::new(|argv| {
            Box::pin(async move {
                let (program, args) = argv.split_first().expect("workspace command has a program");
                let output = tokio::process::Command::new(program)
                    .args(args)
                    .output()
                    .await?;
                Ok(CommandOutput::from(output))
            }) as CommandFuture
        })
    }

    async fn git(root: &std::path::Path, args: &[&str]) {
        let output = tokio::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .await
            .expect("run local git");
        assert!(
            output.status.success(),
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[tokio::test]
    async fn exact_promotion_fast_forwards_the_checked_out_branch_and_keeps_it_clean() {
        let root = std::env::temp_dir().join(format!(
            "restless-integration-promotion-{}",
            uuid::Uuid::new_v4()
        ));
        let repo = root.join("repo");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(&repo).expect("create test repository");
        git(&repo, &["init", "--initial-branch", "main"]).await;
        git(&repo, &["config", "user.email", "promotion@test.invalid"]).await;
        git(&repo, &["config", "user.name", "Promotion Test"]).await;
        std::fs::write(repo.join("README.md"), "seed\n").expect("write seed");
        git(&repo, &["add", "README.md"]).await;
        git(&repo, &["commit", "-m", "seed"]).await;
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "accepted-candidate",
                candidate.to_str().expect("utf8 candidate path"),
                "HEAD",
            ],
        )
        .await;
        std::fs::write(candidate.join("accepted.md"), "accepted\n").expect("write candidate");
        git(&candidate, &["add", "accepted.md"]).await;
        git(&candidate, &["commit", "-m", "accepted candidate"]).await;

        let command = local_git_command();
        let candidate_observation =
            observe_git_workspace(&*command, candidate.to_str().unwrap()).await;
        let candidate_commit = candidate_observation
            .source_commit
            .expect("candidate has an exact commit");
        let promoted = promote_integration_commit_with(
            &*command,
            repo.to_str().unwrap(),
            "main",
            &candidate_commit,
        )
        .await
        .expect("fast-forward accepted commit");

        assert_eq!(
            promoted.source_commit.as_deref(),
            Some(candidate_commit.as_str())
        );
        assert_eq!(promoted.dirty_entries, 0);
        assert_eq!(
            std::fs::read_to_string(repo.join("accepted.md")).unwrap(),
            "accepted\n"
        );
        std::fs::write(repo.join("uncommitted.txt"), "do not overwrite\n")
            .expect("write shared dirty state");
        let refused = promote_integration_commit_with(
            &*command,
            repo.to_str().unwrap(),
            "main",
            &candidate_commit,
        )
        .await
        .unwrap_err();
        assert!(refused
            .to_string()
            .contains("refusing to overwrite shared state"));

        std::fs::remove_dir_all(&root).expect("remove isolated promotion test repository");
    }

    #[tokio::test]
    async fn detached_review_copy_keeps_completed_source_commit_and_status_unchanged() {
        let root = std::env::temp_dir().join(format!(
            "restless-review-copy-integrity-{}",
            uuid::Uuid::new_v4()
        ));
        let repo = root.join("repo");
        let source = root.join("source");
        let review = root.join("reviews").join("attempt-test");
        std::fs::create_dir_all(&repo).expect("create test repository");
        git(&repo, &["init", "--initial-branch", "main"]).await;
        git(&repo, &["config", "user.email", "review@test.invalid"]).await;
        git(&repo, &["config", "user.name", "Review Test"]).await;
        std::fs::write(repo.join("candidate.txt"), "authoritative candidate\n")
            .expect("write candidate");
        git(&repo, &["add", "candidate.txt"]).await;
        git(&repo, &["commit", "-m", "candidate"]).await;
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "completed-attempt",
                source.to_str().expect("utf8 source path"),
                "HEAD",
            ],
        )
        .await;

        let command = local_git_command();
        let before = observe_git_workspace(&*command, source.to_str().unwrap()).await;
        let source_commit = before
            .source_commit
            .clone()
            .expect("completed source has a commit");
        let prepared = prepare_review_copy_with(
            &*command,
            source.to_str().unwrap(),
            repo.to_str().unwrap(),
            review.to_str().unwrap(),
            &source_commit,
        )
        .await
        .expect("prepare detached review copy");
        let alias = root.join("review-alias");
        std::os::unix::fs::symlink(&review, &alias).expect("create stable review alias");
        let hardened = std::process::Command::new("chmod")
            .args(["-R", "a-w"])
            .arg(&review)
            .status()
            .expect("harden local review fixture");
        assert!(hardened.success());
        let source_tree = prepared
            .source_after
            .source_tree
            .as_deref()
            .expect("source tree");
        let probed = probe_review_copy_as_reviewer(
            &*command,
            review.to_str().unwrap(),
            alias.to_str().unwrap(),
            &source_commit,
            source_tree,
        )
        .await
        .expect("reviewer identity can read but not write the declared snapshot");
        assert_eq!(probed, prepared.file_count);
        assert!(prepared.content_digest.starts_with("git-tree:"));
        assert!(
            !prepared.access_probed,
            "generic preparation does not fake the Runtime identity probe"
        );
        assert!(
            std::fs::write(review.join("tracked.txt"), "reviewer mutation\n").is_err(),
            "the reviewer cannot change published bytes"
        );
        let after = observe_git_workspace(&*command, source.to_str().unwrap()).await;
        assert_eq!(prepared.source_commit, source_commit);
        assert_eq!(prepared.source_before, prepared.source_after);
        assert_eq!(before, after, "review output cannot touch source checkout");
        assert_eq!(after.dirty_entries, 0, "source checkout stays clean");
        assert_eq!(
            observe_git_workspace(&*command, review.to_str().unwrap())
                .await
                .source_commit,
            Some(source_commit),
            "review copy is detached at the recorded candidate commit"
        );

        let restored = std::process::Command::new("chmod")
            .args(["-R", "u+w"])
            .arg(&review)
            .status()
            .expect("restore cleanup permission");
        assert!(restored.success());
        std::fs::remove_dir_all(&root).expect("remove isolated review test repository");
    }
}
