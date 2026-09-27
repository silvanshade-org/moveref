# Testing and specifications

> Read when: writing item specifications or their evidence, triaging mutation survivors, choosing an oracle, or scheduling a mutation or fuzz campaign.
> Language-agnostic: the discipline binds Rust, C++, and TypeScript alike; each language hosts the same blocks in its own comment syntax, not a dialect of them.
> Rust instrument specifics (cargo-mutants, lint wiring): [rust.md](rust.md).
> Standing rule: before recording that something does not apply, is not needed, or cannot be done, get owner sign-off — a refutation is a claim too.

## specification-discipline

Every item carries a one-line summary. Every function and method carries a `# Specification` block, as does every other nontrivial item; where there is nothing to state, the block's whole body is `trivial.`, with the terminal period. Outside that presence rule: test functions, generated items, closures; every other function and method the author writes is inside it, interface and trait implementations included. Fallible items also enumerate failure modes. Nontrivial items in new or substantially refactored code also carry `# Adequacy`; a `trivial` block carries neither. Existing survivor hotspots stay in the triage lane — no blanket retrofit. “Nontrivial” means a precondition a caller can violate, a possible failure, or a non-obvious postcondition. It decides which items carry clauses and `# Adequacy`, never whether a function/method's block exists. A thin builder or trivial accessor carries `trivial.`; a trivial non-function item carries the summary alone.

The clause grammar is fixed across languages:

| Clause | States |
| ------ | ------ |
| `- requires:` | Caller preconditions — the valid input space. |
| `- ensures:` | Postconditions on success. |
| `- provides:` | What the item yields. |
| `- fails:` | Failure modes and how they surface. |
| `- panics:` | Reachable aborts/traps/panics; write `- panics: none.` explicitly. |
| `- intension:` | Promised properties of how the computation proceeds; optional, last. |

A **specification** states admitted behavior under a model, input domain, and observer. **Satisfaction** relates an implementation to that specification. **Evidence** is a proof, validated witness, or scoped verification result supporting a particular obligation. **Adequacy** asks whether the specification, observations, and chosen evidence can distinguish the deviations the hypothesis says matter. A passing suite is not a definition of any of the other three.

A refinement strengthens the admitted behavior or maps stronger evidence to weaker evidence; it is not evidence that an implementation satisfies either specification. Preconditions change the obligation's domain. Types, authored predicates, runtime checks, and backend propositions are related representations, not an automatic ladder of increasingly complete specifications. [rust.md §Documentation by specification](rust.md#documentation-by-specification) owns that representation boundary.

The property-class inventory includes values/relations across relevant inputs, branching, termination, panic/typed failure, unsafe and I/O effects, and promised work or footprint. Omissions require a semantic reason, an already-established invariant, a declared model boundary, or an explicit remaining obligation — never an invented “not needed” claim because the current tool cannot express it. Adding a required constraint strengthens the specification; proving an existing one strengthens evidence. A strong score on return-value operators does not establish a cost or effect property absent from the specification.

Project-authored technical vocabulary MUST use specification, obligation, satisfaction, evidence, and adequacy with those meanings. The retired term “contract” MUST NOT be a project synonym. Literal external titles, quotations, crate/API names, and historical filenames retain their exact spelling. This does not rename an external attribute or promise a syntax that the consumer has not adopted.

Documentation MUST accompany the implementing body, following [the authoring order](#from-specification-to-evidence). A change that adds or rewrites an item undocumented is incomplete work. Rust uses rustdoc, with `# Errors`, `# Termination`, and `# Safety` as required by [rust.md](rust.md#documentation-by-specification); C++ uses `///` comments with `panics` read as aborts/traps; TypeScript uses TSDoc. Where gates extract these blocks, their grammar is fixed: same clause names, order, and bullet shape. An absent extractor never removes the authoring obligation or warrants claiming an unimplemented gate exists.

## extension-vs-intension

Extensional properties concern the semantic result and specified errors/effects; intensional properties concern how the computation proceeds. The `requires`/`ensures`/`provides`/`fails`/`panics` clauses form the extensional face; `intension` declares the intensional face. Specify the observer: output equality, termination behavior, declared work count, and private representation are different observations.

[Niu, Sterling, Grodin, and Harper, *A Cost-Aware Logical Framework*](https://doi.org/10.1145/3498670), §§1–2, supplies a formal phase distinction in which behavior cannot depend on cost. The engineering discipline here follows that separation; it does not assert a proved embedding of Rust, C++, or TypeScript into calf. Extensional clauses MUST NOT depend on intensional observations. Retuning an intension and its witnesses MUST leave every extensional witness green. Intensional assertions use only declared semantic projections the API exposes, never a test-only accessor or a representation snapshot.

For example, a normalizer promises a unique ordered representation of the input key set. Sorting/deduplicating and ordered-set insertion may satisfy the same value specification with different comparison counts. Those counts are permitted variation unless a declared work law chooses between them. A measurement definition records which events count and in which environment; changing that definition is not silently comparable performance evidence.

## adequacy

A **mutant** is a program variant produced by a small mechanical fault injection. A suite **kills** it when a baseline-passing test exposes the intended semantic difference. A **survivor** passes the exercised checks. A mutation score is the fraction of the declared eligible viable catalogue killed; state exclusions, indeterminate outcomes, and the denominator. It measures that campaign's ability to notice those perturbations, not specification completeness or universal correctness. A line can execute without any assertion noticing its result.

The retained RIPR vocabulary (Ammann and Offutt, _Introduction to Software Testing_) names four links: **reach** the changed site; **infect** the state on a distinguishing input; **propagate** the difference to an observation; **reveal** it through an oracle. Every rung below strengthens some link. A failure in build, extraction, or harness infrastructure is not automatically the last link.

An adequacy hypothesis MUST name the fault/variation class, valid inputs, observers, and instrument scope. A survivor that the hypothesis requires distinguishing falsifies that hypothesis; classify it and repair the missing part. Do not assume every survivor is a defect, that a classifier must have exactly one cause, or that equivalence follows from failure to find a counterexample.

### tests-are-opens

[Escardó, *Synthetic Topology: Of Data Types and Classical Spaces*](https://doi.org/10.1016/j.entcs.2004.09.017), Part I §§3.1–3.4, relates opens to semidecidable positive observations. Semidecidable means finite evidence can confirm the property; its negation need not be confirmable. It does **not** mean its negation can never be confirmable — decidable predicates are a special case.

Given candidate programs and a chosen observation family, let each pass set consist of programs for which the observation is confirmed. Those sets generate a topology. A finite suite passing on the baseline defines their finite intersection: an **open pass neighborhood**. A survivor belongs to that finite neighborhood. This does not establish equivalence under a larger observation family, nor an unspecified “local equivalence.” With total Boolean observations whose complementary outcomes are also observable, each pass set is clopen and so is the finite intersection. Without those hypotheses, clopenness is not justified.

Positive observations induce a specialization preorder: one program is below another when every positive observation of the first also holds of the second. Mutual specialization is observational equivalence for the full declared family; identifying equal observation profiles gives the Kolmogorov quotient (Escardó, Part II §§5.1–5.2). Finite survival is only agreement on the exercised profile. A harness timeout is indeterminate; calling every non-pass outcome Boolean false does not turn it into a semantic refutation.

Two practical consequences:

1. Boundary differences can receive little probability under the actual generator: `<` and `<=` differ at equality. The probability depends on the domain and distribution; “measure zero” is not a general statement about finite machine inputs. Derive the boundary and bias the generator toward it.
2. An explicitly enumerable finite input class can be exhausted, including finite products across arguments. Credit exactly that finite restriction and decidable observations. Compactness alone does not supply an effective enumeration or prove an unbounded property.

### the-adequacy-ladder

Prefer the highest applicable design rung, and state how it serves the hypothesis:

| Rung | Mechanism | Evidence and limit |
| ---- | --------- | ------------------ |
| **L0 types** | No-default values, newtypes, illegal states unrepresentable. | A rejected illegal variant; not a runtime kill or an inflated score. |
| **L1 evidence** | Return a checkable witness/certificate; validate it against the input. | Detects invalid evidence within the validator's justified relation; merely returning a certificate proves nothing. |
| **L2 agreement** | Independent naïve reference, pinned conformance golden, or semantic stage-boundary artifact. | Detects observed disagreement on exercised inputs; shared bugs remain possible. |
| **L3 pointwise** | Boundary inputs with exact semantic variant/value assertions. | Distinguishes the stated tie-break, guard, or comparison residue. |

Binding rules:

- **External oracle:** an L1/L2 oracle MUST be external to the mutated code: an independent reference, replay checker, or pinned semantic golden. Self-agreement is blind to mutants shifting both runs identically; such surfaces take a pinned external golden. A changed differential surface also earns a directed oracle relating modes, a generator biased toward the weak mode, and a near-miss removing the relevant structure. For a checker, exercise infer/check consistency in both directions and route each directed rule through both modes.
- **Declared projections:** intensional assertions use only declared `intension` projections. Fingerprints compare live computations, never serve as pinned semantic goldens.
- **Clause witnesses:** every `fails`/`ensures` clause has a witness asserting the exact specified variant, value, or semantic relation on a triggering input. Preserve intentional variation; do not pin one permitted representation merely because it is current. “Is an error” alone is insufficient. A bare expected-panic test is disqualified because it accepts any panic and can reward a mutant. Clause witnesses are not a claim of specification completeness.
- **Boundary-biased inputs:** for dense decision surfaces, prefer one boundary-biased property test to scattered redundant unit cases; enumerate the named finite classes exhaustively.
- **Design for adequacy:** prefer APIs returning checkable evidence; concentrate independent checking and directed boundary rigor in the small validators. Never assume a validator is correct because it agrees with its producer.

For the normalizer, membership equality alone permits duplicates; sortedness alone permits returning empty output. `[b,b]` plus strict increase detects duplicate retention; `[a]` plus membership equality detects lost input. More random cases checked by the same weak predicate do not repair the missing observation.

### survivor-taxonomy

Classify against the declared hypothesis and retain exact evidence:

| Class | Meaning | Repair or disposition |
| ----- | ------- | --------------------- |
| missing-input (unreached) | No valid exercised case reaches the change. | Improve input reachability, often at the uncovered file/decision. |
| missing-input (no boundary) | The line runs, but not where its outcomes differ. | Derive equality, emptiness, guard-true, or exactly-one-true cases. |
| missing-observation (oracle) | A distinguishing supported observation exists but no assertion notices it. | Assert exact semantics or validate external evidence. |
| missing-projection (API) | A domain law chooses a correct behavior that the current public surface cannot reveal. | Refine specification and API first, then the witness. |
| missing-specification | Current clauses omit or misstate the independent domain law. | Correct the authored obligation and every affected interpretation. |
| equivalent (semantic) | An argument establishes no separation for all valid inputs and the full declared observer. | Exact exclusion with assumptions and rationale; finite survival is not that argument. |
| accepted-unspecified-variation | Only undeclared representation/intension changes, with no principled choice required by the domain. | Exact permitted-variation rationale and reversal condition; never a test-only accessor. |
| instrument-limitation | Mutation, build, extraction, execution, or classification fails before the claimed observation. | Record the failure and scope; never count it as a semantic survivor, kill, or equivalence proof. |

**Killability is an API-adequacy obligation.** For every viable mutation class changing a domain-significant result on a valid input, the public surface MUST provide a principled semantic observation separating the correct and changed behavior. Derive the distinguishing input independently of the current API; ask whether a domain law chooses a result; strengthen an existing oracle or refine specification/API first. Otherwise record the exact equivalence or intended-variation rationale. Never expose raw private state, add a test-only accessor, or invent an arbitrary requirement to improve a score.

**Generated instrumentation is evidence-bearing.** Do not blanket-discard generated assertion failures or mutations in an emitted check. Classify whether the change weakens enforcement, changes the obligation's interpretation, perturbs the body, is malformed, or hits an instrument limit. Preserve authored-item linkage, exact outcome, and the reason for any exclusion. A compile error is not a survivor; a timeout remains indeterminate.

### completeness-and-coherence

A characteristic specification describes exactly a selected equivalence class, or a selected preorder cone with direction stated. Completeness MUST name the model class, logic, observer/equivalence, and intended allowed variation. A specification may intentionally admit several observably different results. [Aceto et al., *The Complexity of Deciding Characteristic Formulae in van Glabbeek's Branching-Time Spectrum*](https://arxiv.org/abs/2405.13697v3), v3 §2 Definition 8, Remark 9, and Definition 12, supplies the established model-relative terminology for specified LTSs and modal logics; §4 Remark 31 treats finite loops using greatest fixed points. These are not theorems for arbitrary implementation languages or annotations. A mutation score supplies no such characterization.

One authored item can feed runtime, property, fuzz, mutation, bounded, and deductive routes. Each adapter MUST state its interpretation, supported fragment, assumptions, bounds, body/specification revision, and exact result. A handwritten proof mirror requires a correspondence to production before evidence transfers. Equal labels do not establish equal propositions; machine and mathematical arithmetic need an overflow correspondence, and termination-insensitive evidence cannot discharge termination.

Local results compose only when their assumptions, observers, and interpretations align. Restrictions may forget information or narrow inputs; reverse extension requires evidence. A general site/presheaf, descent theorem, logical-relations model, or typed rule-composition structure requires its own construction and laws. None is implied by collecting tool reports. The operational rule is to combine only justified conclusions and retain missing components, not to make consumers depend on a research analogy.

## from-specification-to-evidence

Each clause induces an evidence obligation; executable witnesses credit only their exercised scope:

- **`ensures`:** witness each postcondition on an ordinary input and every boundary the hypothesis names, asserting the exact specified result or relation.
- **`fails`:** trigger each failure mode and assert its exact variant and discriminating payload.
- **`requires`:** defines valid inputs, not behavior to demand outside them. Cover valid boundaries; back internal assumptions with debug assertions. Test input-validation boundaries separately when they promise rejection. Never violate an unsafe precondition to create a test.
- **`panics: none.`:** existing semantic/boundary executions can witness it under mutation without a dedicated test; they do not prove termination or no panic on all unexecuted inputs.
- **`intension`:** witness each declared property through its supported projection. Revise those witnesses with an intentional retuning and keep every extensional witness green.
- **`hypothesis`:** the test plan names the rung and residue: L1 validates evidence against input, with directed rigor in the validator; L2 uses an independent differential/golden; L3 enumerates named boundaries and observations. Exclude no meaningful decision surface by merely naming a rung.
- **`witness`:** each path resolves to exactly one runnable test in the item's own suite; absent, renamed, ambiguous, and wrong-target paths fail. Where the consumer has a gate, it enforces this mechanically; otherwise review checks it. A reviewer can apply the relevant mutant and observe the named witness fail. A cross-crate test may be named in prose, not substituted for a same-suite witness.
- **Authoring order:** for every new item, the specification — types, adopted authored predicates, and prose — MUST be authored and reviewed before the implementing commit. The body's PR MUST cite the specification it discharges; refinements during implementation MUST be separate, named edits. Hypothesis before tests; witnesses last. A survivor contradicting the hypothesis prompts classification and a coordinated update to specification, inputs/API/oracle, hypothesis, and witnesses.
- **A fix reproduces first:** where reproducible, obtain the failing scenario before changing the source and show that the same scenario no longer fails. A user-supplied reproduction is evidence, not a request to re-confirm the report. Where reproduction is unavailable, state the missing condition and what was exercised instead. A surrounding green suite is not proof of an unreproduced fix.

## fuzzing-overlap

Fuzzing and mutation testing vary different sides of the RIPR relation; neither replaces the other:

- **Fuzzing varies inputs against fixed code.** Coverage feedback seeks unreached/infecting states. Crash, trap, sanitizer, or explicit specification assertions determine what it can reveal. Exercise trust-boundary rejection and total-function obligations; feedback alone does not validate wrong answers.
- **Mutation varies code against fixed observations.** It tests whether the oracles notice a perturbation on reached inputs. A crash-only fuzz corpus can reach much and reveal little.
- **Overlap:** in-body `requires`/`ensures` checks upgrade the fuzz oracle from crash detection to the interpreted specification. A fuzz-discovered input can become an L3 boundary witness; a survivor can expose an unobserved region and direct a fuzz target. Neither implication is automatic.
- **Division of labor:** boundary-biased property tests are the standing per-change instrument. Fuzz and mutation campaigns are scheduled sweeps, never landing gates.
- **Campaigns run separately from implementation.** A resource-heavy mutation campaign takes a dedicated execution window without competing work on the host. The implementing work runs no campaign and claims no campaign adequacy score. Production additions or substantial refactors append to the consumer's mutation backlog: crate/package, commit range, intended scope, and hypothesis items. A campaign processes the admitted backlog scope, records results and classifications per row, and leaves the remainder explicit. The backlog path is the consumer's, not a hardcoded source-workspace path. Specification assertions remain shared infrastructure for runtime checks, fuzz oracles, and documented obligations.

## per-language-instruments

The discipline is identical; the instrument differs:

- **Rust:** cargo-mutants vocabulary, viability, records, containment and replay follow [rust.md §cargo-mutants specifics](rust.md#cargo-mutants-specifics). The consumer owns its pin/tasks. Until adopted, use the curated catalogue below without claiming the missing campaign mechanism exists.
- **C++:** a curated mutant catalogue applies hand-maintained changes to decision surfaces and runs the host's test binaries. Claims are only as wide as the exercised catalogue; new decision surfaces require justified catalogue growth.
- **TypeScript:** Stryker is a candidate, adopted only after evaluation of operator coverage and runner integration. Until then, use the curated catalogue and instrument-independent L0–L2 mechanisms.
- **Other language hosting:** one-line item summary and the same specification clause grammar in that language's own comment syntax. An unavailable extractor is not an exemption.

Sanitizers and Miri-style interpreters strengthen memory/undefined-behavior observations. They replace none of the semantic evidence ladder: no memory error is not the same as a correct answer.
