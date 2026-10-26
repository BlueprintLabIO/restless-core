# Sprint 108: heavy jobs on the owner's own computers

Owner direction (26 Oct 2026): computers, including any machine reachable over SSH, should be a
resource a Restless Cloud company can use, without moving the company or its files; and it must be
safe.

What prompted it: moving the Restless company onto the owner's PC to get its Rust builds off the
plane copied its whole disk (about 9 GB, mostly `node_modules` and toolchains) at 0.4 MB/s, and the
company was paused throughout. The owner's point: the reason to connect a computer is to use it,
not to move everything onto it. The code is already on GitHub.

## Design

- **Adding a computer stays as it is.** A PC runs the connect script from Settings › Computers; a
  server is added with `restless connect ssh://user@host` (S106), which runs the same script there
  over the owner's own SSH once. Either way the computer runs `restless-machine`, which dials out to
  the plane. The plane never holds an SSH key and the computer opens no inbound port, and every
  Docker call goes through the machine's filter (Restless's own objects only, no host access;
  hardened in the 24 Oct audit, H2). This was chosen over the plane SSHing in with a stored key:
  that key would be a standing credential to the owner's machine sitting in the plane.
- **A lent computer runs jobs.** The owner lends a computer to a company in Company › Computer ›
  Heavy jobs (the existing helper assignment). An agent runs
  `restless job run --on "<computer>" --repo owner/name --ref <commit> -- <command>`, and
  `restless job status <id>` returns its state and log tail.
- **What a job is:** a throwaway container of the company image on that computer, as the
  `company` user, with bounded CPU (all but one core by default), memory (half the computer's by
  default), processes (4,096) and time (an hour by default, at most six). Its home is a per-company
  cache volume, so toolchains, cargo and npm caches and build output survive between jobs; each
  job's checkout is removed however it ends.
- **The code:** a fresh clone with a token minted for that one repository with read-only contents,
  only from a GitHub App account whose grant lets this company read the repository. A personal
  access token is never handed to a job. The token is removed from the environment before the
  command runs and the clone's remote holds no credential.
- **Isolation:** a job's status answers only to the company whose label it carries. Jobs never run
  on the plane through `--on`. The local provider (`restless job run` without `--on`) is unchanged.

## Done

- [x] Engine: `burst_jobs` machine provider (`on`, `repo`, `ref`), `account_broker::job_clone_token`,
  `github_app::read_token_for_repo`; coordination passes the database through.
- [x] CLI: `restless job run --on --repo --ref`, with help that tells agents when to use it.
- [x] Company › Computer: "Heavy jobs on your computer", one switch per computer (lend or stop),
  with the exact command agents use. A gallery page (`/gallery/computers`) shows it on example data.
- [x] Fix: a change the plane confirms with 204 counted as a failure in the web client
  ("That did not save", and Settings › Computers' "The computer was not removed" after a removal
  that worked). `ownerJson` now treats 204 and an empty body as success.
- [x] Tests: request validation (repository and ref names refuse options, `..` and shell),
  container naming, the token leaving before the command; the machine filter accepts the exact job
  container and refuses it with privileged mode, a host folder, host networking or an added
  capability; the 204 case in the web client.
- [x] Checked on the owner's PC's Docker with the company image and the exact script and options:
  cloned a public repository at a ref, ran as `company`, token unset, remote clean, the second job
  read what the first left in the cache, exit codes passed through (0 and 3), checkouts removed
  even when the clone failed.
- [x] Checked in the gallery at 1280 px and 375 px: lend and stop both save with the right
  message, rows stay 44 px, the long name truncates, no console errors.

## Checked end to end on a local stack (26 Oct)

Work stays local while the owner's GitHub account is flagged, so this ran on `restless-dev
s108_test` built from this branch, with a separate test computer: Docker-in-Docker running its own
`restless-machine` helper, paired to the local plane with a code (a second helper on the PC's own
Docker cannot be used: the machine network belongs to the PC's real helper, and the filter
correctly refuses another machine's network).

- [x] The company's own `restless job --help` (from the image built here) shows `--on`, `--repo`,
  `--ref` and when to use them.
- [x] Lend: `POST /api/companies/s108_test/machines` answered 204 and the test computer showed as a
  helper.
- [x] An agent in the company ran `restless job run --on "s108 box" --repo octocat/Hello-World --ref
  master -- ...`: it started with 19 CPUs and 19g (all but one core, half the memory), the clone
  was at 7fd1a60, it ran as `company`, the token was unset, the remote held no credential, and
  `restless job status` returned succeeded, exit 0 and the log tail; the job container was gone from
  the test computer afterwards.
- [x] With no GitHub account covering the repository, the job says so in `note` and clones
  anonymously (public repositories only); a private one fails in the log (git 128), nothing else.
- [x] A failing job (exit 7) reported failed with its log, and the CLI kept the tail at
  /company/run/jobs/<id>.log.
- [x] Refused with a plain reason: a computer that is not the company's, the plane by name, a
  repository with shell in it, a ref like an option, --ref without --repo, --workdir on another
  computer, the computer offline, and a computer no longer lent.

## Still to check on Cloud

- [ ] After a release: lend Yail's PC to the Restless company and build restless-core there with a
  read token from the GitHub App account.

## Next

- The stuck move (E1 to E3 in Sprint 107): when the receiving side of a move stops, the plane's
  side waited forever, and restarting the computer's helper did not end it either; a move needs a
  deadline and Cancel. The link's 256 KB per-stream window caps a bulk copy at window / round trip.
- Jobs in the Work view: a job an agent started, with its computer, status and log, for the owner.
