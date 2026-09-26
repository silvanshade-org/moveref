# Source workflow

> Read when: building, testing, formatting, or gating the workspace, or writing a commit.
> Rust conventions the gates cannot check: [rust.md](rust.md). Workflow iteration on CI itself: `ci-local.md`, which a tree adopts with its first workflow.

## toolchain-and-tasks

One toolchain for the whole workspace, pinned in `rust-toolchain.toml`; every local crate builds under it, and a Dylint library loaded from Git must use the same compiler. `mise` owns the tool pins (`mise.toml`, `mise.lock`) and the task list (`.config/mise/tasks/`); a pinned tool is reached through mise, never through a host copy that happens to be on `PATH`.

| About to | Run |
| -------- | --- |
| build everything | `cargo build --workspace --all-targets` |
| test everything | `cargo nextest run --workspace` |
| test as CI does | `cargo nextest run --profile ci --workspace` |
| format the tree | `mise run treefmt` |
| check formatting without writing | `mise run treefmt:check` |
| run every repo gate | `mise run check` |
| move a local library producer's toolchain | `mise run toolchain:bump <stable>` |
| move an external consumer's toolchain and policy | `mise run toolchain:bump --workflow-rev <full-commit>` |

`mise run check` fans out to every `check:*` task, so a new gate joins the wall by taking a `check:` name and nothing else.

A workspace consuming `rust-workflow` loads its compiler plugin through `[[workspace.metadata.dylint.libraries]]`: the Git source, a full 40-hex `rev`, and `pattern = "crates/workflow-dylint"`. `mise run workflow:install` reads that same revision and runs `cargo install --git --rev --locked` for `rust-workflow-gates`, installed under `target/workflow-tools`. Neither workflow crate is a Cargo dependency or workspace member of the consumer.

The consumer runs `mise run check:dylint`, which sets `CARGO_INCREMENTAL=0` and invokes `cargo dylint --lib rust_workflow_dylint --no-deps -- --workspace --all-targets`. `check:contracts` and `check:witnesses` depend on the pinned installer and invoke the binary with `--manifest-path Cargo.toml`. Their explicit manifest names the workspace being checked; an installed binary must never infer it from its source checkout. `check:publish` rejects workflow packages anywhere in Cargo's resolved package graph, preserving the future publication boundary.

A library producer owns its plugin's unit/UI suites and Clippy pass: run those from the package directory where its linker configuration applies. Consumers run their entire workspace suite without a workflow-package exclusion; they do not repeat the producer's UI fixtures. Both roles require the pinned `cargo-dylint` and `dylint-link`, plus the library-compatible `rustc-dev` and `llvm-tools` components. After a compiler or policy upgrade, clear `~/.dylint_drivers` and `target/dylint`, then rerun the complete build, test, Clippy, rustdoc, formatting, and policy wall. A workspace adopting no Dylint policy runs its ordinary Clippy wall alone.

## tooling-posture

Tooling is not accumulated. A tool, a task, or a gate that has stopped being useful is removed rather than parked: a parked one keeps its dependencies, its pin, and its place in the wall, and reads as current to whoever finds it next.

Open every new repository with an empty root commit before its first content commit. That stable base lets later history rewrites use an ordinary rebase instead of `git rebase --root`.

**Project index, every language.** Every new project MUST retain a project-local Codegraph mise pin and run `mise exec -- codegraph init` from its root during setup, after initial source files exist. Confirm completion with a query returning an indexed source symbol. The shared tool template supplies the pin; a documented platform/tool failure requires an explicit exception, never silent omission. Indexing belongs to setup, not build or hardware gates.

**Rust for everything possible.** Bespoke tooling is a small binary in the workspace, never a script. A task may invoke a tool; it never becomes one — the moment a task body carries logic rather than a launch line, that logic moves into a crate, with the conventions and the tests every other crate owes. Scripts already in a tree are retired by the crates that replace them, on a burn-down the tree tracks; new ones do not open.

Two narrow exceptions, admitted per project and never standing:

- TypeScript, where a surface is genuinely unsuitable for Rust. The test is interfacing with an ecosystem that has no Rust path, never preference or familiarity.
- Mojo, which stands to Python as TypeScript stands to JavaScript: where the work must reach a Python ecosystem, it is the cheaper exception, because it does not maintain a second remapped surface.

A configuration file in another ecosystem's language is not tooling. A tool whose configuration is necessarily JavaScript or TypeScript keeps that file, and the formatter entries covering it stay; the rule above is about programs, not settings.

Each project keeps one root `CHANGELOG.md`, generated by `git-cliff` from Conventional Commits under a checked-in `cliff.toml`. Release tags are part of the history the format check reads and travel with the heads they delimit: branch pushes include those tags, and CI fetches full history (`fetch-depth: 0`). The generator ignores pre-release tags. `git-cliff` runs as a `treefmt` formatter before the Markdown formatter, so the existing format check is the changelog check and no parallel gate appears. Entries land with the versions they describe; version-specific surface prose lives in the release commit body rather than in READMEs. Crates keep no separate changelogs.

The changelog is generated for the history that will land, not the history on the branch. Squash merge replaces a PR's commits with one commit titled from the PR, so a changelog rendered from the branch commits is wrong on `main` the moment the PR lands and the format check fails there first (`gandr-lang/infinitum` and `quenchant`, 2026-09-22, both on their first hosted run). On a branch the changelog task therefore renders the range since the merge base with the branch commits skipped and one synthetic commit, the PR title with its number, in their place; on `main` it renders plainly. An open PR is the input, so the task refuses to guess without one: open the PR, regenerate, push; retitling the PR means regenerating. The formatter runs with treefmt's content cache off for this step, because commit identity changes without any watched file changing. The merge queue is the enforcement: its run regenerates against the real squash commit, so a stale changelog bounces in the queue rather than landing red.

Prefer the task to the bare binary. A task body carries the pinned tool, the environment, and the flags the tree has settled on, so the same binary run by hand carries the host's instead — and its output is not the gate's output.

## gates

A new tree's hosted CI MUST open in gandr's shape, minus lanes that do not apply, as specified in [ci-local.md §Container image](ci-local.md#container-image); NEVER start with bare-runner-only CI.

| Gate | Command | Refuses |
| ---- | ------- | ------- |
| conflict markers | `mise run check:conflict-markers` | an unresolved Git conflict marker in a tracked file |
| private paths | `mise run check:private-paths` | a private-material reference, or a tracked file under a refused directory |
| shared baseline | `mise run check:baseline-hash` | a `docs/agents/baseline.md` that is not the pinned page |
| rustdoc | `mise run check:doc` | a rustdoc lint over private items |
| CI filter | `mise run check:ci-scripts` | a regression in the changed-categories filter |
| CI pins | `mise run check:ci-pins` | a workflow tool pin drifted from its source of truth |
| external Dylint policy | `mise run check:dylint` | a compiler-plugin finding in the consumer workspace |
| contracts feature graph | `mise run check:contracts` | contract-defeating features in the resolved build graph |
| adequacy witnesses | `mise run check:witnesses` | missing, ambiguous, or wrong-target runnable witnesses |
| publication surface | `mise run check:publish` | unpublished workflow tools entering the Cargo dependency graph |
| formatting, size budgets, shell lint | `mise run treefmt:check` | a file the formatter would rewrite, an over-budget page, a shellcheck finding |
| clippy wall | `cargo clippy --workspace --all-targets -- --deny warnings` | any lint the `[workspace.lints]` wall denies |

The table and hook tiers describe a Rust consumer's wall. The shared task set supplies conflict-marker, baseline-hash, rustdoc, formatting, and external-workflow tasks; a repository adopts the workflow tasks only when it consumes that policy. The shared hook floor is config validation, conflict markers, formatting, and the commit message. The private-path boundary, CI filter, pin-drift check, and push tier are each a repository's own, and the Clippy wall runs wherever a workspace carries the `[workspace.lints]` tables.

`prek` installs the local hooks (`prek install`, hook types from `prek.toml`): the conflict-marker, private-path and formatting gates before every commit, commitlint on the message, and the push-identity guard before every push. The guard resolves the credential the push would use and refuses an unexpected login.

A task that takes arguments declares them. An undeclared value is appended to the last line of the task body, so a task read as parameterized runs unparameterized and silently: declare the usage, and the argument becomes a named value.

`prek` stashes unstaged changes before it runs, so hooks see staged content only. A partial `git add` can therefore fail on a defect the working tree has already fixed, and a change to the hook configuration itself takes effect only once staged.

A formatter cache keys on file content and does not invalidate when its own configuration changes. A configuration change is verified by running the formatter once with its cache disabled; a cached pass proves nothing about the new setting.

Content a gate cannot read is content the gate does not check. Before a new kind of embedded block enters a tree — a diagram language, a generated table, an included fragment — name what verifies it, or record that nothing does.

## commits

Conventional Commits, enforced by commitlint (`commitlint.config.mjs`, prek `commit-msg` hook):

- Type and scope both required, each from the closed vocabulary in `commitlint.config.mjs`. Crate scopes name a `crates/<category>-*` category, never one crate directory, so a scope survives a crate split. Grow either list deliberately via PR; per-surface growth is the failure mode.
- Subject: no trailing period, at most 72 header characters, 50 where the subject fits.
- Body: blank line after the subject, lines at most 100 characters. Body only where the "why" is not obvious from the subject.
- Trailer block preceded by a blank line (`trailer-leading-blank`; the stock `footer-leading-blank` misfires on wrapped prose and stays disabled).
- Human trailers (`Fixes`, `Refs`, `Reviewed-by`, …) where meaningful.

## shared-baseline

`docs/agents/baseline.md` is one shared page, byte-identical wherever it is tracked and pinned by content hash. Editing it in place breaks `check:baseline-hash`: change the page and its pinned hash in the same commit, and the copies elsewhere move with it.
