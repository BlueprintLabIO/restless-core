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
    pub(crate) status_digest: Option<String>,
    pub(crate) dirty_entries: usize,
}

impl WorkspaceObservation {
    pub(crate) fn changed_since(&self, start: &Self) -> bool {
        self.source_commit.is_some()
            && start.source_commit.is_some()
            && (self.source_commit != start.source_commit
                || self.status_digest != start.status_digest)
    }

    pub(crate) fn fingerprint(&self) -> Option<String> {
        self.source_commit.as_ref().map(|commit| {
            format!(
                "{:x}",
                Sha256::digest(
                    format!(
                        "{commit}\\n{}\\n{}",
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
pub(crate) async fn ensure_worktree(config: &CompanyConfig, work: &WorkRow) -> Result<String> {
    let container = runtime::container_name(&config.name);
    let repo = work.repo.as_deref().context("Work repo is missing")?;
    let path = workdir_for(work)?;
    let leaf = path
        .rsplit('/')
        .next()
        .context("Work worktree path has no leaf")?;
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
    if exists.success() {
        return Ok(path);
    }
    let mkdir = tokio::process::Command::new("docker")
        .args([
            "exec",
            "-u",
            "company",
            &container,
            "mkdir",
            "-p",
            "/company/worktrees",
        ])
        .output()
        .await
        .context("create worktree directory")?;
    if !mkdir.status.success() {
        bail!(
            "worktree directory failed: {}",
            String::from_utf8_lossy(&mkdir.stderr)
        );
    }
    let branch = format!("work/{leaf}");
    let mut command = tokio::process::Command::new("docker");
    command.args([
        "exec",
        "-u",
        "company",
        &container,
        "git",
        "-C",
        &format!("/company/repos/{repo}"),
        "worktree",
        "add",
        &path,
        "-b",
        &branch,
    ]);
    if let Some(base) = work.base_ref.as_deref() {
        command.arg(base);
    }
    let output = command.output().await.context("create Work worktree")?;
    if !output.status.success() {
        let reuse = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-u",
                "company",
                &container,
                "git",
                "-C",
                &format!("/company/repos/{repo}"),
                "worktree",
                "add",
                &path,
                &branch,
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
    Ok(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedReviewCopy {
    pub(crate) workdir: String,
    pub(crate) source_commit: String,
    pub(crate) source_before: WorkspaceObservation,
    pub(crate) source_after: WorkspaceObservation,
}

fn review_workdir_for(attempt_id: Uuid) -> String {
    format!("/company/reviews/attempt-{}", attempt_id.simple())
}

fn valid_commit(commit: &str) -> bool {
    commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
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

    let source_after = observe_git_workspace(command, source_workdir).await;
    if source_after != source_before {
        bail!(
            "completed Attempt source {source_workdir} changed while review evidence was prepared; review target is unavailable"
        );
    }
    Ok(PreparedReviewCopy {
        workdir: review_workdir.to_string(),
        source_commit: source_commit.to_string(),
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
    let review_workdir = review_workdir_for(attempt_id);
    let repo_path = format!("/company/repos/{repo}");
    let command = docker_company_command(container);
    prepare_review_copy_with(
        &*command,
        &source_workdir,
        &repo_path,
        &review_workdir,
        source_commit,
    )
    .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        observe_git_workspace, prepare_review_copy_with, CommandFuture, CommandOutput,
        CompanyCommand,
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

        std::fs::write(
            review.join("review-notes.txt"),
            "supporting evidence only\n",
        )
        .expect("write review-only output");
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

        std::fs::remove_dir_all(&root).expect("remove isolated review test repository");
    }
}
