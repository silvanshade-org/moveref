# AGENTS.md

This repository is public. Read this file, then `docs/agents/baseline.md`, then every matching row before acting. Parent and deeper guidance apply together; nearest guidance wins on overlap.

| About to | Read | Never |
| -------- | ---- | ----- |
| write or emit anything | `docs/agents/baseline.md` | use a register or reference the baseline rules out |
| write, change, or review Rust | `docs/agents/rust.md` | weaken the lint wall to make a change pass |
| write a test or claim evidence for a specification | `docs/agents/testing-contracts.md` | count a passing test as proof of an unobserved property |
| build, format, gate, or commit | `docs/agents/source-workflow.md` | invoke a pinned binary outside mise |
| change hosted CI | `docs/agents/ci-local.md` | replace the shared CI shape with bare-runner-only jobs |

## vendored-page-bindings

`docs/agents/rust.md`, `docs/agents/testing-contracts.md`, `docs/agents/source-workflow.md`, and `docs/agents/ci-local.md` are byte-identical shared guidance. `docs/agents/baseline.md` and `docs/agents/baseline.sha256` are a coupled copy checked by `mise run check:baseline-hash`. Project-specific bindings belong here; none weakens a shared rule.

| Shared site | Binding here | Reversal |
| ----------- | ------------ | -------- |
| Rust Shape: crate names, categories, and data path | The product is one library package, `moveref`; a category prefix adds no distinction. Its data path is uninitialized slot storage, placement construction, owning move reference, then destruction or release. | A second crate category appears. |
| Rust Correctness: checker and machine | Slot status and reference tracking in `src/slot_storage.rs`, together with `MoveRef` destruction, are the lifecycle correctness engine. | A separate engine owns these invariants. |
| Rust Representation: `Maybe<T, R>` | Build the type here at its first reasoned absence; change existing signatures and callers together. | A shared crate supplies the type. |
| Rust Enforcement and Verification | `Cargo.toml` owns the lint wall; `mise run check` runs local gates. External Dylint policy commands describe roles this library does not adopt. | This library adopts that policy. |
| Source workflow: publication and index | The library publishes manually. `mise.toml` pins Codegraph for project-local indexing. | Publication or indexing workflow changes. |
| Local CI: jobs, image, and platform lanes | `.github/workflows/ci.yaml` owns Rust quality, dependency policy, tests, Miri, formatting, and workflow checks; `.github/workflows/ci-image.yaml` builds the shared image. The owner-set `MOVEREF_CI_IMAGE` or manual `ci_image` input selects a published image; neither set selects runner bootstrap. | A measured image rollback or platform change. |

The repository is `silvanshade-org/moveref`; API documentation is on docs.rs.
