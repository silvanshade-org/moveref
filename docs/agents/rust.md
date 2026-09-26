# Rust coding conventions

> Read when: writing, changing, or reviewing Rust in this project.
> Enforcement: `Cargo.toml` `[workspace.lints]` wall + dylint gate set. This file: rationale + the parts those linters cannot check.
> Evidence obligations for item specifications: [testing-contracts.md](testing-contracts.md).
> RFC 2119 applies to MUST, REQUIRED, SHOULD, RECOMMENDED, MAY, and OPTIONAL; `NEVER` reads as `MUST NOT` and `AVOID` as `SHOULD NOT`.
> Standing rule: recording that something does not apply, is not needed, or cannot be done requires owner sign-off — a refutation is a claim too.

- [Non-negotiable](#non-negotiable)
- [Shape](#shape)
- [Representation](#representation)
- [Correctness](#correctness)
- [Performance](#performance)
- [Style](#style)
- [Lints and enforcement](#lints-and-enforcement)
- [Dependencies and the workspace](#dependencies-and-the-workspace)
- [Documentation by specification](#documentation-by-specification)
- [cargo-mutants specifics](#cargo-mutants-specifics)
- [Effort obligations](#effort-obligations)
- [Verification](#verification)

Next split: Representation into its own page, retaining every rule and example behind a link here; signature boundaries form its independent subject.

## Non-negotiable

- **The design spec owns the semantics; the Rust implements it literally.** A spec-implementation divergence is a spec change or a bug, never an implementation liberty.
- **The lint wall is a design constraint.** NEVER weaken it to land code; the only sanctioned relaxations are the narrow reasoned `#[expect]`, the `clippy.toml` test configuration, and the ticketed crate-local override block, all stated under Lints and enforcement.
- **Exported types preserve durable identity and capability boundaries.**
- **Three bans carry no exception a project may take on its own**: recursion of any depth profile, ownership that routes a type through itself by an owning pointer, and a bare primitive — or an `Option` — in a crate-defined signature. Each is stated in full below with its exception set — an owner-approved `# Termination` specification for the first, none at all for the second, exactly two signature cases for the third. The third also carries a containment rule, that primitives are unpacked only inside the trait implementation that needs them, whose one escape hatch is a commented site where no trait applies and which no dylint checks yet.
- **Signatures preserve information.** A signature that drops which failure occurred, why a value is absent, which state a value is in, or which domain a value belongs to is a defect, and lowering a return type's information is never a correct refactor. Stated with its instances under Representation.
- **Deferred behavior stays explicit and incapable of fake success.**
- **Every Rust change passes all five gate classes** — type check, lints, rustdoc, tests, formatter — with warnings counted as failures.

## Shape

The sites below carry the source workspace's own vocabulary rather than a shared rule. Every rule stands as written — one naming authority, intent over mechanism, a name that survives a crate split, load-bearing machinery owned rather than depended on — and a project reads each site as an example of the rule it illustrates, binding it to its own vocabulary in its root `AGENTS.md`.

| Site | Reads as |
| ---- | -------- |
| the `<category>-<name>` directory schema and its `gandr-<directory>` package prefix, in the naming bullet | the project's own directory schema and package prefix |
| the category list in that same bullet | the project's own categories |
| the `# consumers:` line of the dependencies example | a crate of the project's own workspace |
| the flat data path in the crate bullet below | the project's own pipeline |
| the checker and the machine in the keystone clause under **Partial functions banned** | the project's own core engines |
| `gandr-owned` in the external-implementations bullet — the one site of a different kind, part of a rule every project keeps | the project's own owner name, and never a dropped rule |

- **Crate boring, explicit.** One job per crate — one crate owns one concern end to end; flat data path (parser → CST → lowering → core IR → checker/machine). Clean cutover over compatibility shims; no aliases or dead paths unless the owner approves the exception, and a compatibility alias or deprecated path that does exist is removed together with its callers in one change. General/reusable machinery gets its own crate; a design pass owes the crate/module-boundary judgement, not only the edits.
- **Naming follows the category schema; the workspace is the naming authority.** Directory = `<category>-<name>`; package = `gandr-<directory>`. Categories: `core-*` (frozen core — checker + sequent IL + the incremental typing riding on it), `kernel-*` (certified TCB + its substrate), `theory-*` (reusable metatheory machinery), `surface-*` (syntax, grammar, parsing, lowering/engine, corpus, render, editor faces), `runtime-*` (host effects, codecs, FFI), `storage-*` (CAS tier), `workflow-*` (repo tooling + gates). Port/new crate: read `crates/` first, derive the name from the schema — intent over mechanism (`surface-engine`, not `-pipeline`); avoid collision with overloaded terms (`runtime-codecs`, not `-data`); a remote/wire face of a local surface takes a `-remote` suffix beside its local sibling.
- **Cross-concern values cross as explicit public domain types.** A value leaving its owning crate carries a named domain type from that crate, never a structural stand-in; domain logic stays independent of generated binding types, which live at the generating boundary, never in the logic underneath.
- **Every new crate carries one focused crate-level README.**
- **Choose architecture with performance in view.** Before concrete implementation: enumerate the plausible architectures; compare runtime + memory profiles alongside correctness, extensibility, maintainability, implementation complexity. Prefer the best-performing design that does not materially sacrifice the other qualities. No default to the most direct design when improving it later would require an expensive change to ownership, representation, interfaces, or persistence; no micro-optimizing local code without evidence.
- **Prefer incremental, resumable, streaming-compatible execution.** Where the problem permits: bounded steps over explicit state, not one monolithic call. Progress checkpointable: interrupted work resumes without replaying completed steps. Streams: consume/produce without retaining the complete input or result in memory.
- **Prefer first-order representations.** Continuations, callbacks, control states, work items = explicit data + an interpreter or state machine; defunctionalize higher-order machinery where practical. First-order data: execution inspectable, serializable, persistable, cacheable, testable, resumable. Opaque closures / dynamic dispatch only when those properties do not matter and the higher-order form materially improves the design.

## Representation

**Signatures preserve information.** A signature that drops which failure occurred, why a value is absent, which state a value is in, or which domain a value belongs to is a defect: the caller cannot recover what the type no longer says, and every later reader pays for it. The rules in this section are that principle in particular places, and the table is the instruction.

| Weaker signature | What it drops | What the rule requires |
| ---------------- | ------------- | ---------------------- |
| `Option<T>` for a fallible operation | which failure occurred, and why | `Result<T, E>` with a named error enum; a non-failure absence goes to `Maybe<T, R>` rather than `Option` |
| `bool` for a classification | which of more than two states, and the names of the two | an enum, even where there are exactly two states |
| a bare primitive across a boundary | the domain meaning of the value | the nominal wrapper required by **Crate-defined signatures preserve semantic information** below |
| `Option<T>` in a crate-defined signature | the reason for absence | `Maybe<T, R>` with a sealed per-site reason enum; `Option` only at the two external boundaries |
| a refactor lowering a return type's information (`Result` → `Maybe` → `Option`, enum → `bool`, wrapper → primitive) | information the caller already had | never correct; a review-blocking finding in that direction |

**`Option` is disallowed in crate-defined signatures, under the same rule as bare primitives.** The same two exceptions — a method implementing a trait from a non-stdlib external dependency, and a method implementing a standard-library trait, each only where that trait's required signature contains it — the same isolation to maximally local scope, and the same escape-hatch comment where no trait applies. `None` is a unit: it cannot say whether a value was not in the map, not computed, filtered out, or lost to a swallowed error. Three shapes carry an empty arm, and which one a signature uses is the instruction:

| Shape | The empty arm carries | The reviewer reads |
| ----- | --------------------- | ------------------ |
| `Result<T, E>` | a failure | the failure is handled or propagated |
| `Maybe<T, R>` | evidence of a non-failure absence: a sealed per-site reason enum | whether `R` is the right reason at this site |
| `Option<T>` | nothing | permitted only at the two external boundaries; a finding anywhere else |

`Maybe<T, R>` is a nominal enum (`Present(T)` / `Absent(R)`), not an alias: no `?`, no `From<Maybe<T, R>> for Result<T, E>`, no `impl Try` — an absence is matched, never propagated on the error channel, which is what "not a failure" means to the type checker. `R` is a sealed reason enum defined per site (`Exhausted`, `Unbound`, `NotCached`, `OutOfRange`), never a shared catch-all, because the reason is the evidence and a generic reason erases it again. A `Result` whose error type is not a failure — no `Error` implementation, matched as evidence — is a `Maybe` misfiled. The item's `# Specification` `- provides:` clause names the reason enum and what each variant means. Where a site both fails and can be absent the shapes compose rather than merge — `Result<Maybe<T, R>, E>` — and a failure is never an `R` variant. Refactor direction is fixed: `Result` → `Maybe` → `Option` lowers information and is a review-blocking finding, while `Option` → `Maybe` and `Maybe` → `Result` are the correcting direction.

The `Iterator` case, worked. `impl Iterator` for a crate type unpacks to `Option<Item>` in its `next` implementation and nowhere else: that is the standard-library-trait exception, and that implementation is the border. The crate's own API over the same sequence returns `Maybe<Item, Exhausted>` when exhaustion is the only non-failure absence. A site that can also filter or miss uses a sealed per-site reason enum with those non-failure variants; a site that can fail returns `Result<Maybe<Item, R>, E>`, and a failure is never an `R` variant. The difference is what a reviewer does with it: an `Option` makes the reviewer stop, because `None` says nothing, while `Maybe<Item, Exhausted>` makes the reviewer read the reason and ask whether it is the right one at this site. The reason enum is the review prompt.

`Maybe<T, R>` has no shared home yet: the `rust-shape` crate that would carry it does not exist, so a project builds or vendors the shape at its first use and the rule binds by review. No dylint check enforces it either — planned as one more type in the primitives predicate's list — so review reads for it.

Two corollaries. `.ok()` on a `Result` at a boundary is the same defect in method form: the `# Specification` `- fails:` clause then describes what the signature no longer carries. `Option<bool>` and `Result<bool, E>` combine the rules above: use `Result<State, Error>` for a fallible classification, use `Maybe<State, R>` where the empty arm is a non-failure absence carrying its reason, and otherwise model the complete closed state set directly as one enum. `Option<State>` is not available as the compromise — the rule above disallows it in a crate-defined signature, so it survives only at the two external boundaries.

- **Recursive owned data flat, id-addressed; ownership never routes a type through itself.** A recursive type's children = indices into an arena or interning table — never nodes linked through `Box`, `Rc`, `Arc`, or any other owning pointer. Pointer-linked recursion defeats destruction and duplication totality, threading conformance, and content identity at once. General class: any `Deref`-owning type whose clone/drop recurses through the pointee; the named three = the floor a reviewer checks without analysis. Outside recursive positions: `Rc` unused at all; `Arc` = the one permitted shared-ownership pointer, whole values at boundaries; `Box` = ordinary owned indirection.
- **Crate-defined signatures preserve semantic information.** A function/method defined by a workspace crate must not accept or return a bare primitive value (`bool`, `char`, numeric primitives, `str`) — directly or beneath references, pointers, tuples, arrays, slices, generic containers — before reaching a nominal type boundary. Applies regardless of visibility: free/const/async/extern functions; inherent methods; local-trait declarations, defaults, implementations. Exceptions, exactly two: a method implementing a trait from a non-stdlib external dependency, only where that trait's required signature contains the primitive; and a method implementing a standard-library trait, only where that trait's required signature contains the primitive. A crate-defined inherent signature is never exempt merely because a standard-library inherent function has the same shape. Primitives are isolated to maximally local scope: that border is usually drawn at the `core::ops::*` and similar trait implementations — the primitive is unpacked inside the trait implementation that gives the wrapper its operation, and nowhere else. Where a trait exists to contain the unpacking (`core::ops`, `core::cmp`, `core::fmt`, `core::hash`, `From`/`TryFrom`, `Iterator`, and the like), it MUST be used; unpacking the primitive by hand in an arbitrary function is disallowed. Escape hatch, and only where no such trait applies: unpack at the narrowest site and carry a comment stating why no trait fits. No dylint check enforces this containment yet — planned, and tricky to implement — so review reads for it. Remedy: a nominal domain wrapper rather than a type alias; each single-field wrapper `#[repr(transparent)]`; implement the utility traits needed for effective use. Wrappers keep semantically distinct values from becoming interchangeable; preserve meaning for humans + agents.
- **Single-field structs transparent.** Every named or tuple struct with exactly one field carries `#[repr(transparent)]`. Exception: a concrete layout/ABI/soundness reason documented in the item's `# Specification`.
- **`non_exhaustive` is not used.** Workspace is `publish = false` end to end, no public API: the attribute protects no external consumer, and inside the workspace it costs a wildcard arm at every match, defeating exhaustiveness on the enums where a new variant is supposed to break every consumer until considered. The posture reverses only if a published boundary exists.
- **Lifetimes name semantic roles.** Relationship-bearing names (`'source`, `'arena`, `'world`); never alphabetical/positional (`'a`, `'b`). Carry the same name through related struct/impl/trait/associated-type signatures: the borrowing relationship stays traceable.
- **A hand-written relation is total over its own definition; delegating an arm is a claim.** A custom comparison/ordering/hash that hand-writes some arms and delegates others to a derived or foreign relation (`==`, `cmp`, a sibling helper) asserts the two relations agree on every position reachable through that arm — usually false the moment the property being relaxed reaches the delegated arm through a child. Write every arm of the relation being defined, or state at the delegation site why agreement holds on everything reachable. A deliberately coarse arm (one that intentionally identifies more than its exact siblings) carries a witness that the looseness is intended + why. Ported code is the hot spot: a port lands as one large diff, so its relation definitions arrive unexamined.
- **A durable identifier takes its own newtype.** Two confusable identifiers never share a type, whatever representation they carry underneath.
- **A closed set of states is an enum, never a stringly status value.**
- **Capability-bearing values keep their fields private, and every public constructor preserves every invariant the type promises.**
- **Durable payloads derive `Serialize` and `Deserialize`.**
- **Comparable identifiers SHOULD derive `Eq`, `Ord`, and `Hash`.**
- **Owned payloads SHOULD cross durable boundaries; borrowed views SHOULD stay local.**
- **Collections reflect semantics.** Order, uniqueness, and lookup behavior follow the collection's meaning, never whichever container was at hand.
- **Optionality represents genuine absence, never an undocumented sentinel.**
- **Impossible states SHOULD be unrepresentable.**
- **Public names use the vocabulary of the surface that owns them** — the design spec where the project has one, otherwise the page or the issue that commissions the surface.

## Correctness

- **Recursion banned.** No tail-call optimization guarantee in Rust: recursion whose depth scales with input (list length, term/AST depth, environment size) = a latent stack overflow on real data. Call-graph lints for it have known blind spots (derived-trait recursion, drop glue, closure-mediated self-calls): a "bounded" exception is unverifiable in the general case. Instead: the explicit worklist, heap frame-stack, or iterative loop, always. New recursion of any depth profile = a review-blocking finding; an exception requires an owner-approved `# Termination` specification on the item (grammar below). A specification oracle in a total metalanguage is the specification, not an implementation blueprint: the Rust is its iterative shadow; a divergence in shape is expected, only a divergence in result is a bug (differentials compare answers, not call graphs).
- **Partial functions banned.** No indexing or slicing; use `.get(..)`, `split_first`, iteration. No `unwrap`/`expect` on a fallible value in shipping code — return a typed error. Lint-denied: `unwrap_used`, `expect_used`, `unwrap_in_result`, `get_unwrap`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`, `exit`, integer `/`, bare overflowing arithmetic. The keystone: the checker and the machine are **total** — structured errors, never divergence.
- **A function is total over the states its own types represent.** Every admitted state handled; a fallthrough arm standing in for an unhandleable state is a representation defect, not a default.
- **Fallible boundaries return typed errors once the behavior lands; error context identifies the failed operation and, when one is involved, the relevant durable identity.**
- **`unsafe` requires a measured need, a safety proof, and owner review**, plus the documentation obligations under Documentation by specification.
- **Concurrency carries a documented ownership and cancellation model, and cancellation preserves all state invariants, including durable-state invariants where present.**

## Performance

- **Design memory behavior deliberately.** Zero-copy data flow + borrowed views where they do not impose disproportionate lifetime/API complexity. Allocation necessary: minimize allocation count + copying via capacity planning, arenas, interning, buffer/object reuse, workload-justified caching. Cache cost includes invalidation, retained memory, synchronization.
- **Avoidable allocation, cloning, and serialization are defects.** NEVER clone to satisfy the borrow checker without proving the ownership need.
- **Hot-path collections SHOULD preallocate wherever cardinality is known.**
- **Large payloads stay out of logs and diagnostic strings.**
- **Measure before adding a cache, an arena, or a custom allocator as an optimization.** The arena a recursive type's children index into is a representation requirement (see Representation), not an optimization: no measurement needed.

## Style

- **The `?` operator is a statement, not a subexpression.** Never bury `?` inside a larger expression (`f(x?)`, `Some(v?.0)`, `break g()?`): bind the fallible step with `let` first, then use the bound name. Collision with an existing name: prefer shadowing (`let node = node(parent)?;`) over a near-duplicate name.
- **Shadow rather than suffix.** A binding never used again after the re-binding: reuse the same name, not a suffixed variant. Trivial projections (unwrapping a wrapper field, dereferencing, `.trim()`) never justify a new name. A distinct name: only a genuine transformation (parsing, decoding, conversion into a different domain type) or a value that must stay live beside its successor. Lint states, by owner ruling: `shadow_same`, `shadow_unrelated`, and `shadow_reuse` are all off — the latter two misfire in practice, and `shadow_same` bars only rebinding a value to itself, which this rule permits. The ruling overrides the wall where they conflict; a later seat does not re-enable them.
- **Qualification consistent, the qualifier earns its place.** A file refers to a given module's items with one qualification throughout. External crates: full paths by default (`std::fs::read_to_string`, `syn::Item`); no importing the module itself; no importing external types under light-to-moderate use. A short prefix outweighs the heavy-use convenience: `syn::` and `serde_json::` stay fully qualified even where a type is used heavily. Repetition in the path justifies importing the module as the qualifier (`use yaml_rust2::yaml;` then `yaml::Hash`). Local project crates: import types more liberally where the type name is informative. An item's own name uninformative: qualify through an informative module path; no informative path → reorganize the module hierarchy rather than mint an alias.
- **No `as` conversions.** `as` truncates, wraps, changes signedness silently. Convert through the value's total API: `uN::from`/`uN::try_from` with the error consumed, or serialize the source width and take the bytes needed.
- **Arithmetic checked, never bare.** `saturating_*` for monotone counters/depths; `checked_*` where overflow must surface; `wrapping_*` only for hashing. `arithmetic_side_effects` denied workspace-wide.
- **Embedded syntax written raw.** A string literal carrying actual surface syntax — or any multi-line embedded content (fixtures, expected renderings, corpus snippets) — is a raw string (`r#"…"#` style) with real newlines, never an escaped-`\n` literal split across backslash continuations. Why: escaped snippets unreadable + undiffable; rustfmt's continuation reflow moves the backslash breaks, so the literal's visual shape drifts from its content.
- **Boring control flow and explicit state transitions.** An early return keeps the successful path flat.
- **Name types by role.** NEVER a `Data`, `Info`, or `Manager` suffix without a semantic need.
- **Comments explain a constraint, an invariant, or a surprising tradeoff** — never a restatement of what the code says.

## Lints and enforcement

Gates-first: the lint wall + dylint gate set land before or with the code they govern, never after. The gate set itself is planned separately; this section states the posture the gates enforce.

- **Test code under the same wall; the only sanctioned test relaxation = `clippy.toml` configuration.** The repo `clippy.toml` enables clippy's native `allow-*-in-tests` options (`dbg`, `expect`, `indexing-slicing`, `panic`, `print`, `unwrap`): relaxes exactly those lints inside test code, no source attribute. Attribute-based test relaxations — crate-level `#![cfg_attr(test, allow(...))]` or per-file `#![allow(...)]` test walls — prohibited: they leak across items, creep in scope, bury drift. Every lint without a clippy.toml in-tests option — notably `arithmetic_side_effects` — binds tests exactly as production.
- **Panic policy.** Production paths: typed errors. A panic acceptable only as a `debug_assert!` of an internal invariant, in test/bench code, or in a spec-deferred skeleton under the expectation named below. Every reachable panic: routed through a structured error variant, or documented in the item's `# Specification`.
- **No silencing a diagnostic to pass a gate.** A warning is a failure; fix the source or file a ticket, and never bury drift in a `// TODO` (`todo` lint-denied). A blanket `allow` and a global suppression are both prohibited. The narrow reasoned expectation Non-negotiable names takes exactly three shapes, and no others:
  - `#[expect(lint, reason = "...")]` on the item that needs it, scoped tightly (`allow_attributes` and `allow_attributes_without_reason` both denied).
  - A file-level `#![expect(...)]` in a `harness = false` bench: benches are not test code for the clippy.toml in-tests options.
  - `todo!()` in a spec-deferred skeleton, under one narrow `#[expect(…, reason = "…")]` naming the governing spec section, which never simulates success, persistence, routing, or command execution.
- **Crates join the workspace; detached crates prohibited.** Every new crate = a root-workspace member from day one: no crate-local `[workspace]` tables, no out-of-workspace drivers with their own lockfiles, toolchain pins, or lint posture. Detached crates escape the `[workspace.lints]` wall; silently accumulated lint debt returns as a remediation project. A crate that cannot satisfy the wall yet still joins the workspace + carries a crate-local override block referencing a triage ticket: the debt stays visible and scoped. That block is the ticketed crate-local override Non-negotiable names, and the one crate-scoped relaxation that is not the prohibited blanket `allow`: ticketed, scoped to the crate carrying the debt, removed with the ticket.

## Dependencies and the workspace

- **External implementations = design references, not automatic dependencies.** Machinery load-bearing for the core or certified-kernel boundary (recursion/control runtimes, graph representations + algorithms, proof-state machinery, semantic normalization) stays gandr-owned even when reimplementation costs more. Preference order: `core`/`alloc` → a focused local crate → existing workspace dependencies; external convenience must not enlarge the trusted or bootstrapping-critical base. A new dependency needs a concrete capability the standard library and the current dependency graph both lack. Non-kernel dependencies passing that boundary test: `default-features = false` + an explicit feature list, minimal and explicit wherever default features add weight.
- **Dependencies live once, at the workspace root, inherited everywhere.** Every external dependency (normal/build/dev) declared in the root `Cargo.toml` under `[workspace.dependencies]`; member crates reference it with `{ workspace = true }`. A crate-local `version =` re-declaration = a review finding; a shared SDK's version stays workspace-wide and singular for the same reason. Member manifests inherit the workspace package and lint settings rather than restating them. One crate needs a feature no other consumer needs: enable it on the workspace declaration, not fork the pin. Every `[workspace.dependencies]` entry: one brief comment directly above saying what the dependency does, followed by a `# consumers:` block — a dashed comment list, one consuming crate per line, dev-only consumers marked `(dev)`:

  ```toml
  # BLAKE3 hashing for content and manifest digests.
  # consumers:
  # - storage-records
  [workspace.dependencies.blake3]
  version = "1.8.5"
  default-features = false
  features = ["std"]
  ```

  Comment: what the crate _is_ — a phrase, not a paragraph. Consumer changes: the crate's name joins/leaves the `# consumers:` block in the same change; the last consumer leaves → the workspace declaration is removed in the same change.
- **A dependency upgrade updates the lockfile and passes the whole gate graph.**
- **Publishing is manual, never automated.** Every crate carries `publish = false`, including any published-name holder; flipping the field is itself the deliberate act of a manual publish. No CI job, hook, or agent process runs `cargo publish`. The posture is revisited with the public-visibility programme.

## Documentation by specification

A specification states admitted behavior; satisfaction relates an implementation to it; evidence supports a particular obligation under stated assumptions; adequacy asks whether the specification, observations, and evidence distinguish the deviations that matter. These meanings are fixed by [testing-contracts.md](testing-contracts.md#specification-discipline). Project-authored technical vocabulary uses those terms, not the retired synonym “contract”; literal external titles, quotations, API identifiers, and historical filenames remain exact.

One authored specification has several representations, not an automatic ladder of increasingly complete proofs:

- **Types** delimit representable states. The Brouwer–Heyting–Kolmogorov / propositions-as-types reading interprets logical witnesses in an appropriate type theory ([Homotopy Type Theory](https://arxiv.org/abs/1308.0729), §1.11); an arbitrary Rust function is not thereby a proof of its behavior or termination.
- **Authored predicates**, in the syntax the project actually adopts, express obligations beyond the type. A runtime check establishes only the interpreted predicate on an executed call.
- **Backend clauses**, including Verus-native clauses, express propositions under the backend's semantics and assumptions. Extraction or a handwritten mirror requires a correspondence to the production body and authored specification before its result transfers.
- **Target-language predicates** can express the same intended obligation in another language only through a justified interpretation. Merely sharing an attribute name does not establish equal meaning or an implemented adapter.

Every evidence route records its source item/specification revision, supported fragment, arithmetic/effect model, premises, bounds, and exact outcome. Unsupported, inconclusive, timed-out, and refuted are distinct. A generated assertion failure remains evidence until classified; generated code is not a blanket exclusion. A mutation score is not specification completeness, and a finite survivor is not a proof of semantic equivalence.

**Instrumentation mode is not specification presence.** A project MAY adopt optional executable specification checks, including a mode that uses published instrumentation with `std` and omits it for true `no_std`. Required runtime validation and safety checks remain ordinary code. A disabled expansion must remove every supported nested marker as well as the outer marker, without inventing unsupported nested syntax. The authored obligations remain authoritative even when checks are absent from the binary. A future adapter must read authored source or an explicitly preserved specification representation and relate it to the configured body; stripped HIR MUST NOT be presumed to retain erased clauses. This is a representation rule, not a claim that any particular wrapper or adapter is implemented.

**Specification before implementation:** [testing-contracts.md §from-specification-to-evidence](testing-contracts.md#from-specification-to-evidence) owns the authoring order. Summary and specification MUST remain with the implementing body. A rewrite MUST check them against the new code; adding or rewriting an item undocumented is incomplete work. Documentation states the specification; it never narrates the implementation.

Every item (public or private — `missing_docs_in_private_items` denied) carries a one-line summary, and the rustdoc gate binds private items exactly as it binds public ones. Every function and method carries a `# Specification` rustdoc block, as does every other nontrivial item; where there is nothing to state, the block's whole body is the single word `trivial`. Outside that presence rule: `#[test]` functions, `#[automatically_derived]` items, closures; every other function and method the crate authors is inside it, trait impl methods included. Fallible functions: also `# Errors`. Nontrivial items in new or substantially-refactored code: also `# Adequacy` ([testing-contracts.md](testing-contracts.md)); a `trivial` block carries neither.

An in-scope item links by its bare name (`` [`Value`] ``); a cross-module item: reference-style link — the short label in the prose, its path collected once as a definition at the end of the doc block — no full path repeated inline at every occurrence.

```rust
/// Convert a zero-based protocol coordinate into a one-based coordinate.
///
/// # Specification
/// - requires: `value` is a zero-based coordinate from an external protocol.
/// - ensures: returns `value + 1` when representable.
/// - provides: a one-based coordinate for the display layer.
/// - fails: returns `CoordinateError::Overflow` on `u32::MAX` rather than panicking.
/// - panics: none.
///
/// # Errors
/// - `CoordinateError::Overflow`: `value` is `u32::MAX`, so no one-based
///   coordinate exists.
///
/// # Adequacy
/// - hypothesis: L3 only — the `+ 1` and the overflow guard are separated
///   solely by the boundary pair `u32::MAX` / `u32::MAX - 1` plus one ordinary
///   value asserted exactly.
/// - witness: `position::tests::one_based_ordinary_value_is_exact`
/// - witness: `position::tests::one_based_boundary_at_u32_max`
```

A function with nothing to state:

```rust
/// Return the number of nodes in the arena.
///
/// # Specification
/// trivial.
```

- Clauses in fixed order, as `-` bullets: `requires` (caller preconditions) → `ensures` (postconditions on success) → `provides` (what the item yields) → `fails` (failure modes + how they surface) → `panics` → optional `intension` (last); omit a clause only when it does not apply. Write `- panics: none.` explicitly: absence of panic is a specified obligation. An `unsafe` item: add `- unsafe invariants:`, the rustdoc `# Safety` section, `// SAFETY:` comments (`undocumented_unsafe_blocks` denied).
- `- intension:` states properties of _how_ the computation proceeds (enumeration/tie-break order, traversal strategy, cost, determinism, trace shape) — only those the item **promises**, each observable through a **declared semantic projection** the API exposes. Intensional tests assert only declared projections; extensional clauses never reference intensional observations ([testing-contracts.md](testing-contracts.md) §extension-vs-intension).
- `# Errors` coexists with `# Specification`: `- fails:` the design-level statement, `# Errors` the per-variant enumeration.
- `# Termination` mandatory on any owner-approved recursive exception (recursion otherwise banned; see Correctness). Fixed grammar; each field: a concrete explanation, not an assertion that termination is obvious:

  ```rust
  /// # Termination
  /// - reason: why recursion is the appropriate control structure here.
  /// - measure: the quantity that strictly decreases on every recursive edge.
  /// - boundedness: where the finite or well-founded bound comes from.
  /// - input recursion: none.
  ```

  Tail-call position does not remove this obligation (no TCO guarantee); a genuinely iterative implementation is not recursive and needs no termination section.
- `# Adequacy`: `- hypothesis:` — a falsifiable claim naming the valid-input domain, observer, relevant mutation classes, and which evidence-ladder rung distinguishes each decision surface, plus the pointwise residue's inputs and observations — then one `- witness:` bullet per witnessing test. Fixed grammar: where adequacy gates exist, they machine-extract it; otherwise review enforces the same rule. A witness resolves within its own crate's targets, library + integration alike; a test in another crate exercising this item is worth naming in prose, not a witness. Every claim remains bounded by the instrument's supported scope.
- "Nontrivial": has a precondition a caller can violate, can fail, or has a non-obvious postcondition. It decides which items carry clauses, and which carry `# Adequacy`, never whether the block exists: a thin builder or trivial accessor carries the `trivial` block, and a data constant or other trivial non-function item carries the one-line summary alone.
- The canonical `trivial` body is `trivial.`, with the terminal period; `specification_present` also accepts `trivial` without the period, and in either case no bullet marker and no clause stands beside it.

## cargo-mutants specifics

The concept (adequacy, the ladder, the survivor taxonomy): [testing-contracts.md](testing-contracts.md); this section is the Rust instrument.

Mutation tooling is the project's own. A project that runs campaigns pins cargo-mutants and defines contained `mutants:campaign` and `mutants:replay` mise tasks — the campaign task refusing zero viable mutants or no exercised tests, the replay task validating the campaign base plus exactly one before-image match — and the shared task set supplies neither; until those tasks exist the rules below bind by review.

- **Vocabulary as cargo-mutants produces it.** A mutant = one small mechanical change: replace a function body with a default value, delete a unary operator, swap a binary operator. A mutant that does not build is **unviable** and was never executed. Preserve its compiler diagnostic: it can document an L0 exclusion or an instrument/build problem, but never a runtime kill, survivor, or inflated denominator. A score states the eligible viable catalogue and all exclusions.
- **A campaign with no viable mutants is an infrastructure failure; the tasks fail on it.** Zero viable = not a score of zero and not a score of one: the absence of a measurement — every mutant died at the compiler for a reason the mutants themselves do not explain (a toolchain the sandbox lacks, a wrong build scope, a lint denial that killed the edit before a test saw it). A baseline that exercised no tests: refused on the same ground. Both refusals name the infrastructure + point the reader to the build logs: a surviving mutant and an unbuildable one call for opposite work.
- **Reproducible mutation records.** Every recorded mutation = a canonical record: repository-relative file + semantic item, the exact before/after edit, the bounded base identity, one evidence-bearing verdict — killed with a test, compile_error with the compiler diagnostic, timed_out with retained evidence, or survivor with the surviving-test evidence. A compile error is never a survivor; a timeout remains indeterminate rather than collapsing into either. Replay: validates the campaign base; requires exactly one before-image match; ambiguous or stale-source entries rejected rather than guessed.
- **Campaign lifecycle.** Mutation experiments are scheduled standalone campaigns through contained tasks only — never pre-push, pre-merge, or CI gates, and never beside implementation. [Testing guidance](testing-contracts.md#fuzzing-overlap) owns the dedicated execution window and backlog rule. A cheap contained replay of named mutants MAY provide focused verification when supported; it is not a campaign or an adequacy score. A full mutation run is outside the completion scope of every task, feature, and epic. Production additions or substantial refactors record a future standalone campaign's commit range, intended scope, and hypotheses in the consumer's mutation backlog, without a blocking dependency back to the implementation.
- **Instrument limits.** cargo-mutants: no argument-swap or wrong-algorithm operator → the score under-measures the fault model this discipline defends against (agent-authored code's characteristic faults). Judge the intensional face by the external-oracle rule, not the score.
- **Per-file coverage floors.** Crate-level coverage judgment banned: a line can be executed by a test that asserts nothing about it — adequacy, not coverage, is the metric that binds.
- **Instrumentation outcomes.** Retain the authored item, generated location, mutation, input, observer, and exact result. Classify weakened enforcement, changed specification interpretation, body defects, malformed instrumentation, and tool limits separately. Never discard an assertion failure solely because it arises in generated code; never collapse timeout into semantic survival.

## Effort obligations

- **Never simplify away**: input validation at a trust boundary, error handling that prevents data loss, security measures, accessibility basics, anything explicitly requested. A request for the full version is built, never re-argued.
- **A deliberate corner-cut carries its ceiling in the source.** A known-suboptimal choice (a global lock, an O(n²) scan, a naive heuristic) takes an `economy:` comment naming the ceiling and the upgrade path — `// economy: one lock per arena, per-node locks if contention shows`. Uncommented, the ceiling is invisible to the next reader and the upgrade path is re-derived from scratch.

## Verification

Every Rust change runs the whole gate graph; all five classes must pass: type check, lint wall, private-item rustdoc gate, test suite over every target, formatter in check mode. A warning is a failure. Fix the source; NEVER suppress a diagnostic globally to make a gate green.

The five classes bind everywhere; the task names are the project's. The shared task set supplies `check:doc`, `check:conflict-markers`, `check:baseline-hash`, and `treefmt:check`, and a project adds its own `check:*` names for the type check, the lint wall, and the tests.
