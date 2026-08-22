---
name: publish-git
description: Safely reconcile, verify, and publish Git refs through the governed Restless effect path. Use for pushes or deployments driven by a Git push; do not use for read-only fetches, local Git work, or provider API administration.
---

# Publish Git

Use ordinary Git directly for inspection, fetching, merging, rebasing, conflict resolution, and
tests. Only the push is a consequential external effect.

## Establish the real state

1. Probe `command -v restless git git-credential-restless` and inspect their help where applicable.
   A command reached through `bash` is available even though it is not a separate ACP-native tool.
2. Run `restless credential check` and identify the exact configured binding. Presence proves only
   that Restless can resolve the reference; it does not prove provider acceptance.
3. Fetch the target remote directly, then inspect the branch, upstream divergence, merge base, and
   working tree. Never claim a branch is ready from `HEAD` alone.
4. Preserve existing work. Do not detach the primary checkout to compare or test another revision;
   use `git show`, `git diff`, or a disposable worktree under `/company/worktrees` instead.
5. Reconcile the current remote target, resolve conflicts by product intent, run the repository's
   relevant gates, and require a clean publishing worktree before pushing.

If release diagnosis exposes substantive application-source, dependency, test, build, or CI repair,
keep ownership explicit. An Exec scopes the failure and commissions exact Work to an existing suitable
Staff actor; it does not absorb the implementation into the release wake. Staff performs that repair
inside its claimed Attempt and links the resulting commit. Exec resumes release judgement and the
governed push from that accepted artifact. A role or skill never grants effect authority by itself.

## Publish once

Choose a precise effect class such as `repo.push`, the canonical external party, the exact working
directory, and an idempotency key stable for the target ref and commit. Bind the secret only to
`RESTLESS_GIT_PASSWORD`; configure the non-secret username separately. For an HTTPS remote, the
shape is:

```sh
restless effect \
  --class repo.push \
  --purpose "<business reason>" \
  --party <canonical-party> \
  --artifact "commit <full-sha>" \
  --secret RESTLESS_GIT_PASSWORD=<credential-binding> \
  --key <stable-ref-and-sha-key> \
  --cwd <absolute-repository-path> \
  -- git \
    -c safe.directory=<absolute-repository-path> \
    -c credential.username=x-access-token \
    -c credential.helper=restless \
    push origin <source-ref>:<target-ref>
```

The helper receives the injected value through the child environment and Git credential protocol;
the value never belongs in argv, a remote URL, persistent login state, a file, or a receipt.

If Authority denies the effect, report the exact denial or prepare its requested approval. Do not
describe `restless effect` as unavailable unless its executable/help probe failed. If an execution
has an unknown outcome, reconcile that same idempotency key; never retry under a new key.

## Verify the consequence

Compare the provider-observed remote ref with the intended full commit SHA. Use an ordinary read
(`git ls-remote` or provider CLI/API status); public reads are not effects, and a Git command run
under the isolated effect UID must carry the same explicit `safe.directory` setting as the push.
Then observe the actual CI/deployment and the requested live outcome. A successful push is evidence
of publication, not evidence that CI passed or the product deployed. A compile phase followed by a
failed build is a failed build; a test command with failed files is not “all tests passed,” even when
the failures predate the candidate.
