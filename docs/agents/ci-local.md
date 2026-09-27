# Local CI with act

Run `.github/workflows/ci.yml` locally via [act](https://github.com/nektos/act) + Docker. Iterate on workflow changes without burning hosted minutes or triggering run approvals — agents never approve GitHub Actions runs, so act-local is the only autonomous iteration loop for CI.

The local loop and [container pattern](#container-image) bind every Rust tree. Port gandr's applicable workflow shape rather than designing a second one: the composite setup action at `.github/actions/setup-rust/action.yml`, `.actrc`, path-filtered parallel jobs, workflow lint, image build and switch. Preserve the adopting project's gate commands. The script names, external-policy commands and platform lanes below are source examples; a root binding maps them to the project's own tools and omits only lanes the project does not adopt.

## Anchor-first sharing

Hosted workflows MUST use a disabled `template` job as an organized catalogue of reusable YAML anchors. Sharing takes priority: refactor differences into action inputs, job parameters, or shared mappings rather than copy orchestration. New functionality MUST reuse existing template anchors or introduce anchors when recurrence is likely. Define job-metadata anchors at their first schema-valid site; preserve readable, gate-specific commands in the consuming jobs.

YAML aliases reuse complete nodes; they neither merge mappings nor splice step sequences. Anchors are document-local. Use composite actions for shared multi-step or cross-workflow behavior, with anchored calls inside each workflow; NEVER add dependency jobs or serialize parallel lanes merely to share setup. A demonstrated performance cost is the exception to structural sharing, not a reason to abandon reuse: record the affected path and comparable measurements, then retain shared logic through actions wherever possible. Preserve cache keys and writers, exact policy-cache freshness, image/bootstrap selection, checkout depth, event/path conditions, and every gate obligation.

Every workflow MUST start with a brief binding comment so a reader entering through that file encounters the rule. The comment repeats the local editing constraint, not a second policy. Quenchant's gate workflow uses:

```yaml
# <critical>
# MUST preserve anchor-first CI: maximal sharing, no copied orchestration.
# New functionality MUST reuse template anchors or add anchors for likely reuse.
# MUST parameterize differences; performance exceptions require measured evidence.
# Share exceptional logic through actions; preserve parallelism, caches, and gates.
# </critical>
```

The image workflow uses the same notice with its final obligation naming native parallel image builds. Its shared definitions include this fragment; live build and manifest jobs consume the three aliases in their existing step order:

```yaml
jobs:
  template:
    if: false
    runs-on: &linux ubuntu-latest
    permissions: &image-permissions
      contents: read
      packages: write
    steps:
      - &git-checkout
        name: git checkout
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - &registry-login
        name: log in to GHCR
        uses: docker/login-action@dbcb813823bdd20940b903addbd779551569679f # v4.6.0
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - &setup-buildx
        name: set up buildx
        uses: docker/setup-buildx-action@37fe631027851001ddb9b187196cc803df7f5f0e # v4.3.0
```

Each consumer supplies `permissions: *image-permissions` and reuses `*git-checkout`, `*registry-login`, and `*setup-buildx`; the build matrix still selects native runners. The gate workflow similarly shares Rust setup, tool installation, cache operations, reports, and repeated job metadata. Job parameters select tool sets and report destinations. Tool versions resolve in step context because job-level `env` cannot refer to workflow `env`. Only the history-dependent formatting job requests a full checkout; only the existing main warmer jobs save shared caches.

Verify the expanded execution contract and exercise every applicable job under act, including parameter variants and image/bootstrap conditions. Compare timings under matching events, platforms, images, and cache state; hosted acceptance remains the [container measurement contract](#container-image). Revisit this sharing rule only on measured regression or changed workflow-engine capabilities, never for editing convenience.

## Inline run: rule

An inline `run:` step MUST be one command or one pipeline. Branching orchestration belongs in the project's Rust tooling; steps pass inputs through `env:`, never `${{ }}` inside `run:`. Gandr's existing `scripts/ci/` commands below are source examples, not scripts a new tree should introduce. Quenchant's `changes` job uses a Git/JQ pipeline without shell control flow or a third-party filter action; separate fetch and filter steps expose failure through their outcomes, which force the Rust category on.

- `scripts/ci/changed-categories.sh` — the changes job's path filter: reads the event name and base SHAs from `CI_EVENT_NAME`, `CI_BASE_SHA`, `CI_BEFORE_SHA`, and the queue's base branch from `CI_MERGE_GROUP_BASE`; on a `merge_group` event the wire category is the entry's diff against that base branch, the rust category stays forced on; emits `rust=`/`wire=` to `GITHUB_OUTPUT`, or to stdout when unset. The fixture `scripts/ci/changed-categories.test.sh` builds a throwaway repo and drives every branch; `mise run check:ci-scripts` runs it.
- `scripts/ci/check-private-paths.sh` — the private operational boundary for tracked files and commit messages: it bars internals, never provenance; `scripts/ci/check-private-paths.test.sh` proves the provenance fields pass and private references fail, and `mise run check:ci-scripts` runs it.
- `scripts/ci/check-pins.sh` — the workflow's tool pins (`CARGO_NEXTEST_VERSION`, `DYLINT_VERSION`) against `mise.toml`, and the consumer's compiler/tool versions against its full-revision Git library source; `mise run check:ci-pins` runs it. Dylint authoring dependencies belong to the library producer, not the consumer manifest.
- the `.github/actions/setup-rust` composite action (invoked from the workflow as `./.github/actions/setup-rust`) — the rustup cache restore and save, the toolchain install, and the shared Rust dependency cache, shared by every Rust job.

## Requirements

- Docker Desktop running (`docker info` succeeds).
- `mise.toml` MUST pin `github:nektos/act`; gandr and quenchant use `0.2.89`. Reach the pin through `mise exec -- act`; `mise install` resolves it.

## Invocation

Repo root carries `.actrc` (image pin + `--pull=false` for repeat-run speed). Run a single job — heavy-lane jobs (`cargo-build-crates`, `cargo-dylint-gates`) condition on the merge-group or push event, so drive them with `merge_group` (the `changes` job short-circuits on event name alone, no payload needed); light-lane jobs take `pull_request`:

```sh
mise exec -- act merge_group -W .github/workflows/ci.yml -j cargo-dylint-gates
```

List jobs without running them:

```sh
mise exec -- act -l
```

Time a job the same way hosted CI's job/step timings are read, for before/after comparisons:

```sh
time mise exec -- act merge_group -W .github/workflows/ci.yml -j cargo-dylint-gates
```

## Measuring before/after locally

`git show <ref>:.github/workflows/ci.yml > .github/workflows/ci.yml` swaps in another ref's workflow file on disk — act copies the working tree into the container by default (no `actions/checkout` network fetch), so this is enough to time an old workflow revision against the current one with a warm image and warm act-cache-server, no separate clone needed. Restore with `git checkout -- .github/workflows/ci.yml` after. Exclude the very first invocation on a host from any delta — it pays a one-time image pull that later runs skip (`--pull=false`).

## Cross-OS lanes are hosted-only

The three cross-OS nextest lanes in `ci.yml` — `cargo-test-macos` on `macos-latest`, `cargo-test-windows-x64` on `windows-latest`, `cargo-test-windows-arm64` on `windows-11-arm` — are excluded from local iteration: act runs Linux images (`.actrc` pins `catthehacker/ubuntu:act-latest`), so a lane whose `runs-on` is a macOS or Windows host has no runnable form under it. Validate those lanes statically instead — `zizmor .github/workflows`, `act -l` for the job list without running, and the workflow read against the hosted run's own behaviour — because the hosted merge-group run is the only oracle for a lane this host cannot emulate, and a pushed PR's run actually registering the new jobs (not failing at parse or dispatch) is part of validating them.

## Toolchain bump

A library producer runs `mise run toolchain:bump <stable>` to move its nightly and local `clippy_utils` tag together. An external consumer runs `mise run toolchain:bump --workflow-rev <full-commit>` only after that source revision's CI passes: the task takes the source's nightly and changes the consumer's Dylint revision together. Its gate installer reads the same metadata, so library and binary cannot carry independent revisions. Run `check:ci-pins` against the new source, clear `~/.dylint_drivers` and `target/dylint`, then rerun the full gate wall.

## External workflow consumer

Load `rust_workflow_dylint` from the workspace's Git metadata with `CARGO_INCREMENTAL=0`. Cache `target/dylint` only by an exact key containing the metadata file, `rust-toolchain.toml`, the Dylint version, and runner OS/architecture; never supply restore prefixes. A restored library from another revision is another policy even when Cargo considers its artifact fresh.

Install `rust-workflow-gates` with the metadata revision through `cargo install --git --rev --locked --bin rust-workflow-gates rust-workflow-gates`. Both `contracts` and `witnesses` require the consumer's `--manifest-path`. Prove discovery and both commands from an unrelated working directory. Consumer build, nextest, Clippy, rustdoc, and cross-target commands cover every local workspace member; plugin unit/UI tests belong to the producer.

A cold hosted run needs access to the Git source. An authenticated local pass does not prove public-source access: while the source remains private, a consumer PR stays draft and states that hosted CI and landing wait for public access. No consumer Actions secret is added for a dependency intended to be public.

## The big-endian lane is hosted-only

`big-endian.yml` runs the test suites under `cross test --target s390x-unknown-linux-gnu`: QEMU user-mode emulation inside the `cross` Docker image. act runs workflows in its own container and cannot nest Docker, so the lane cannot run locally. Dispatch it instead:

```sh
gh workflow run big-endian.yml
```

The lane triggers on a weekly schedule, manual dispatch, and merge-group entries whose diff touches a wire crate (`crates/storage-records`, `crates/kernel-term`, or `crates/surface-syntax`). It runs `cross test --workspace --target s390x-unknown-linux-gnu`, covering every local workspace member. The external compiler plugin is outside that workspace and is not cross-compiled. While the lane runs, a `gandr-storage-records` endianness test asserts the target is really big-endian, so a dispatch against a misconfigured target fails loudly rather than passing vacuously.

## Container image

Hosted CI MUST use the applicable gandr shape:

- **Prebuilt tools.** `.github/docker/ci.Dockerfile` bakes the pinned rustup toolchain, its components and targets, the CI subset of `mise.toml`, and the matching Dylint driver. Dependency artifacts and the project gate library remain cached separately.
- **Main as warmer.** Keep the `push` trigger on `main` and never cancel it for a later push. `Swatinem/rust-cache` retains workspace artifacts through `cache-workspace-crates`; only `main` writes shared caches. Pull requests and merge-group runs restore them: a queue's ephemeral ref is a sibling of PR branches and disappears after landing.
- **Exact policy caches.** Use explicit `actions/cache/restore` and `actions/cache/save` pairs for the Dylint driver and `target/dylint`; save before rust-cache's post-job pruning. Driver keys contain runner OS/architecture, `rust-toolchain.toml` and Dylint version. Gate-library keys also contain policy sources or pinned Git metadata, `Cargo.toml` and `Cargo.lock`, without restore prefixes. A path crate's mtime-based freshness can load stale policy from a prefix hit. An image-baked driver needs no second restore.
- **Parallel gate jobs.** Build/tests, Dylint, Clippy, anodized invocation policy, formatting/repository policy, and workflow lint run separately. Preserve the project's deny, witness, no-std, enforcement and rustdoc checks within those lanes. Set `CARGO_INCREMENTAL: 0`: reused incremental state crashes the Dylint driver. Heavy lanes run on `merge_group` and `push`; light lanes remain on pull requests.
- **Path-filtered jobs, not workflows.** `changes` skips Rust for documentation-only diffs while formatting and workflow lint always run. Unknown bases, failed fetches or diffs MUST force Rust on; merge-group entries MUST check the complete Rust merge result. Rename detection stays off so deleting Rust by moving it into docs cannot skip Rust checks.
- **Workflow lint.** Keep the `workflow-lint` job: pinned `taiki-e/install-action`, then `zizmor .github/workflows`. Copy the `self-repository` and `unpinned-images` annotations only where the same local-action and relayed-content-tag constructs recur. Every action revision MUST be immutable and admitted by the host's allowlist.

The image workflow builds each platform natively: `ubuntu-latest` for amd64 and `ubuntu-24.04-arm` for arm64. Builds push by digest; a merge job publishes the multi-platform manifest. Never compile the image under QEMU. `MISE_DISABLE_TOOLS` names tools CI never invokes, appears identically in the workflow and Dockerfile, and is checked during image construction. Install the whole CI subset within the cached setup step on the fallback path: later mise auto-installs otherwise escape that cache.

`.github/workflows/ci-image.yml` publishes two tags: SHA-256 over `mise.toml`, `mise.lock`, and `rust-toolchain.toml` concatenated in that order, plus `latest`. Rebuild on a `main` push touching those pins or the Dockerfile, on the weekly schedule, or on manual dispatch. The base image is digest-pinned. Dockerfile edits rebuild the existing pin-file tag; `latest` is a discovery tag, never the CI pin.

`ci-image-ref` relays the workflow-level image switch through `needs` into every Rust job's `container:` field, where the `env` context is unavailable. An empty switch selects the bare runner and the shared `setup-rust` action installs the pinned toolchain; a published pin-file tag selects the image and skips installation while retaining caches. Empty is the bootstrap or measured rollback state, not a second permanent CI design.

Private GHCR job images require `packages: read` on the consuming jobs and package access for the workflow repository. Define `permissions: &image-read-permissions { contents: read, packages: read }` in the disabled template; image-consuming jobs MUST reuse `permissions: *image-read-permissions`. Keep unrelated jobs at their existing permissions. The runner authenticates repository-linked GHCR pulls with its existing `GITHUB_TOKEN`; NEVER add a PAT or change package visibility to repair a missing job permission. Verify an actual hosted container pull: local Docker credentials have a separate access boundary. Failed activation MUST restore the prior image-switch value while the source fix awaits landing.

Gandr uses `GANDR_CI_IMAGE` for `ghcr.io/gandr-lang/gandr-ci`. Quenchant uses `QUENCHANT_CI_IMAGE` for `ghcr.io/gandr-lang/quenchant-ci`, supplied by the repository variable of that name. The first `main` push builds the image; activate its published tag before measuring the second `main` run. The same selection applies to act. Authenticate package reads through `docker login ghcr.io`; for quenchant, pass the tag without changing the workflow:

```sh
mise exec -- act merge_group -W .github/workflows/ci.yml -j cargo-dylint-gates --var QUENCHANT_CI_IMAGE=<published-pin-file-tag>
mise exec -- act merge_group -W .github/workflows/ci.yml -j cargo-build-crates --var QUENCHANT_CI_IMAGE=<published-pin-file-tag>
```

Measure before and after on hosted runs with the same event, applicable lanes and cache state. Record run URL, revision, image tag, overall elapsed time and each job's timings from `gh run view <id> --json jobs`; obtain step detail with `gh api repos/<owner>/<repo>/actions/jobs/<id>`. Exclude the first local image pull from local deltas. A short PR run with skipped heavy jobs is not evidence for a full `main` run. Retain the image only when warm-run wall time does not regress and cold-run wall time improves; the adoption target is a green full `main` run within five minutes. If the first warm run exceeds that target, identify the slow step and repair the same adoption change. Any dependency-baked image variant, rebuilt on `Cargo.lock`, requires its own measured decision.

Omit gandr's big-endian, Miri and cross-OS lanes only when absent from the adopting project's existing coverage. Keep applicable lanes and their platform constraints. Revisit this pattern only on a measured regression, changed hosting capability, or a documented project requirement it cannot meet.

### Adopter measurements

These observations retain their execution boundary; cross-platform and cross-event rows are not speedup ratios.

| Adopter and observation | Elapsed | Scope and evidence |
| ----------------------- | ------- | ------------------ |
| Quenchant before the image port, hosted `main`, 2026-09-09 | 16m49s overall; Dylint 16m38s; build/test 4m21s | [Run 34402774193](https://github.com/gandr-lang/quenchant/actions/runs/34402774193), creation to last job completion; full hosted gate set. |
| Quenchant after the port, native arm64 act, 2026-09-10 | Build/test 71.07s; Dylint 126.03s | [Eight-job acceptance](https://github.com/gandr-lang/quenchant/pull/3#issuecomment-5611148612), with the image already local; each invocation includes prerequisite jobs. |
| Gandr reference, hosted `main`, 2026-09-08 | 13m54s overall; Dylint 13m44s | [Run 34248970279](https://github.com/gandr-lang/gandr/actions/runs/34248970279), including heavy and cross-OS lanes. |
| Gandr reference, hosted pull request, 2026-09-08 | 2m39s overall | [Run 34249391296](https://github.com/gandr-lang/gandr/actions/runs/34249391296), with both heavy jobs skipped; this is not a full-CI timing. |

Quenchant's local acceptance verifies every applicable `ci.yml` job. Its post-port hosted timing remains a separate measurement; neither the local numbers nor gandr's short PR run establishes the five-minute hosted target.

## Platform notes

- Host is macOS; act needs Docker Desktop's daemon socket up (`docker info`) before any invocation works. Docker's privileged-helper install can stick on an interactive macOS admin prompt (`supervisor.log` shows an `osascript ... administrator privileges` call that never exits) — that needs a human at the GUI. Switching Docker Desktop to unprivileged user-socket mode (`EnableDefaultDockerSocket=false`, `RequireVmnetd=false` in its settings, then restart) avoids the prompt entirely; `docker info` reports `Context: desktop-linux` when this is active. If neither resolves it, static analysis + hosted-run timings (`gh api repos/<owner>/<repo>/actions/jobs/<id>`) are the fallback.
- On Apple Silicon, `.actrc` pins `catthehacker/ubuntu:act-latest`, which ships arm64 manifests — Docker pulls the native arch automatically (`docker image inspect catthehacker/ubuntu:act-latest --format '{{.Architecture}}'` confirms `arm64`), no `--container-architecture` override needed. Force `--container-architecture linux/amd64` only to debug an x86_64-specific failure.
- act always prints "You are using Apple M-series chip and you have not specified container architecture" on this host — cosmetic boilerplate, not a fault signal; every job here has run clean under the native arm64 image.
- act 0.2.89 runs a built-in cache server by default (`--cache-server-path`, defaults under `~/.cache/actcache`), so the workflow's `actions/cache` steps work locally with no extra flags.
- Run separate act invocations sequentially when they share prerequisite jobs: act assigns the same container names to those jobs, so concurrent invocations collide before execution. The CI image MUST expose its pinned Node executable on `PATH`; act uses it for JavaScript actions rather than the hosted runner's bundled Node.
- History-dependent jobs need a complete Git directory inside act's copied checkout. A linked worktree's `.git` file can point outside the container; run such jobs from a disposable full clone of the same branch, with release tags present. This applies to git-cliff and repository metadata gates, not merely Rust compilation.
- `secrets.GITHUB_TOKEN` is empty under act unless supplied. The zizmor job reads it for `GH_TOKEN`; pass a real token only if a step needs live GitHub API calls, and use act's bare env-form secret (`-s KEY` with no value reads the value from `act`'s own process environment) rather than a `KEY=value` argument, so the token never lands in argv or shell history: `GITHUB_TOKEN="$(gh auth token)" act pull_request -j workflow-lint -s GITHUB_TOKEN`.
- Composite actions with inline shell functions (`taiki-e/install-action`'s `bail() { ... }`) can surface as spurious `⭐ Run Main bail() {` log lines — an act log-parsing quirk, not a step failure; the real step still reports its own success line.
- Local per-step wall time is not a hosted-CI predictor: this M-series host compiles the whole workspace in ~15-20s where GH's shared runner takes 70-100s for the same step. Use act-local timing only for relative before/after on one host, not as an absolute hosted-minutes estimate.
- A hosted `startup_failure` (e.g. from the repo's Actions allowlist, a repo Settings policy naming which `uses:` sources are permitted) is invisible to actionlint, act, and strict YAML parsing alike — none of them see repo-level policy. The hosted run is the only oracle for it, so a pushed PR's run actually registering jobs (not failing at parse/dispatch, before any job runs) is part of workflow validation, not an optional extra check.
