# Agent Baseline

One shared page, byte-identical in every repo of the family, pinned by content hash. Root `AGENTS.md` routes here; read this before task work, then the rows the root table fires.

## guidance-is-instruction

Guidance is instruction, not advice. A rule earns its place only when its effect can be pointed to in what the agent produces: a line in the artifact, a shape in the reply, a gate that fires. A rule whose effect could only be established by comparing runs with and without it is advice; it is removed, or rewritten as an instruction whose effect is visible.

## register-and-scope

Default register: the `caveman` skill at ultra level. Skill unavailable → the rules below bind unchanged.

Scopes are **inclusive** and apply simultaneously.

"**ALL** content default" = all agent output, every response for the entire session, every written artifact. Includes copies, moves, templates, seeds, any other generated output.

Only global exclusions: verbatim-mandated content — licenses, hashed content, lockfiles, vendored files, an upstream synced target.

**ALWAYS** user-confirm exclusions unless unattended.

Obligation: before end-of-turn, per file touched, non-conforming → rewrite same turn. **NEVER** defer or note-and-move-on.

## attention-economy

Scope: **ALL** content default, durable artifacts. Optimize for attention and cognitive overhead. Binds everywhere: no register below exempts an artifact from this section.

Good:

- **scannable**: short paragraphs, lead with **sloganized** ideas, bold for attention.
- **relatable**: examples, analogies, diagrams, tables, lists over more words.
- **pictorial**: **ALWAYS** mermaid diagrams, typst, LaTeX where rendering supported.
- **tabulated**: vertical (bulleted) lists or table rows when more than 3 things.
- **attentive**: task conflict or unsafe assumption → point out once, concrete.
- **clarified**: clarity-seeking user question → speak less tersely.

Bad:

- Walls of text, cluttered, non-bursty prose.
- Long horizontal lists (more than 3).
- Complex parentheticals and non-linear phrasing.
- Flat, featureless presentation of results.

## context-economy

Scope: **ALL** content default, durable artifacts. Compressed register: cut **ALL** filler, keep technical content. Binds everywhere, at the compression level the register below sets.

- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- Use abbreviations: docs vs documentation.
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].

Exclusions: **none**. A register below fixes an artifact's compression level and which structural devices it uses — never whether this section or the one above applies. A project-consumer artifact is written in the documentation register, still filler-free.

## technical-writing

Scope: results of a complex turn (debugging, implementation, research), durable technical artifacts, owner artifacts. Less compressed than the default register, still economical: terse grammatical prose, suitable for a technical note, readable in isolation. Both economies above bind unchanged.

Separate results into sections.

- Label each section with a short unambiguous concept-id anchor.
- **ALWAYS** full context. Never compress to theorem numbers, cite keys, bead ids.
- **ALWAYS** recall prompting scope: "relates to data representation".
- **ALWAYS** explain relevance: "enables incremental re-checking".
- **ALWAYS** explain impact: what changes, allows, disallows, verifies, refutes.

Register selection, not an exclusion: a project-consumer artifact takes the documentation register below. Both economies bind there too.

## documentation-writing

Scope: durable project-consumer docs, **NOT** agent artifacts, **NOT** owner artifacts. Terse-but-natural prose: concise, unambiguous, grammatical, simple, no fluff, no flourish, no poetry. Both economies bind; this is the one register whose structural devices are optional — concept-id anchors and sloganized bold are **NOT** required, because these docs read as prose.

- Concise + unambiguous: no filler, fluff, hedging, asides.
- Direct: state what **is**, not boundary conditions of X.
- No "it's not X, it's Y" / "X prevails, and Y is the crux": no conjoined conditionals + negations.
- No hedging or obfuscation to hide uncertainty: state what is known, what is unknown, why.
- No paragraph re-explaining: state topic + relevance, refer to the issue or document carrying full context.

## documentation-history

- **NEVER** history or backward-looking language in a doc. **ONLY** exception: CHANGELOG.
- **NEVER** stateful language: "owner ruling", "revised" + date.
- **NEVER** append directives modifying prior content.

Doc reflects **current** reality. Rewrite: delete stale, insert correct. No modification aside. History = version control.

## reference-stability

A reference resolving only in its original artifact arrives elsewhere meaning nothing — or something else. **NEVER** ambiguous references, anywhere: docs, specs, comments, commits, tracker, reviews, plans, notes, chat.

Good:

- Citation key, known BibTeX / Haygriva / references file.
- Full paper title + authors + date + stable identifier (DOI, ISBN, arXiv, HAL).
- Concept-id anchor, established in current context, unambiguous across contexts.
- Theorem / lemma / section anchor + document or artifact specified.

Bad:

- Bare letter-number ids: `M1`, `S1`, `P1`, `H2`, `D11`, `F3.19` — collide elsewhere.
- Descriptive gestures: "tagless-final paper", "leading implementation".
- Author-year, no register.

## tool-routing

Route by the task at hand; the trigger column **binds**. Reaching for the raw command when a row matches is a conformance miss, not a style preference. A routed tool that is missing, broken, or degraded: report it, then the raw alternative is conformant — name the fallback where it is used.

| About to | Use | Never |
| --- | --- | --- |
| run or read compiler / linter / language-server / test / text diagnostics | `aifix`: `aifix batch` scoped to the target, or pipe the output through `aifix pipeline`; work from its deduped findings | hand-triage a raw diagnostic dump |
| orient, or answer a structure / implementation query — find an item, its callers, impact, blast radius | `codegraph` (`codegraph explore`) | a grep-and-open-files walk |
| understand changes — entity-level diff, blame, conflict risk | `sem` (CLI only) | raw `git diff` / `git blame` archaeology |
| resolve a merge conflict | `weave` | hand-editing conflict markers |
| vet a URL, shell command, MCP config, or risky input | `tirith` | eyeballing it |
| read the current API surface of a popular library | `context7` | a recalled API surface |
| rebase, gate, land | `wt merge` | ad-hoc rebase scripting |

- **ALWAYS** read tool-output images: they are compact by design, and skipping one discards the cheapest evidence available.
- **ALWAYS** detect configured tools at session start; report confusing, broken, duplicate, degraded, or unavailable tool configuration.
- Else: the harness's own file, edit, search, and non-diagnostic language-server tools.
