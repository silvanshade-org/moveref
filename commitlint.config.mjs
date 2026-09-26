import { execFileSync } from "node:child_process";

// Real trailer tokens. The conventional-commits parser treats ANY `word:` line
// as the footer start, so the stock footer-leading-blank rule misfires on
// wrapped prose; this closed list is what makes the replacement rule sound.
const TRAILER_TOKENS = [
  "BREAKING CHANGE",
  "BREAKING-CHANGE",
  "Acked-by",
  "Cc",
  "Closes",
  "Co-Authored-By",
  "Fixes",
  "Refs",
  "Reported-by",
  "Reviewed-by",
  // The provenance block's own two tokens. Without them the block's first line
  // reads as prose, so the leading-blank check fires on the line below it
  // rather than on the block, and an agent commit is refused for a defect it
  // does not have.
  "Role",
  "Session",
  "Signed-off-by",
  "Tested-by",
];
const TRAILER_LINE = new RegExp(`^(?:${TRAILER_TOKENS.join("|")}):[ \\t]`, "i");

const trailerLeadingBlank = (parsed) => {
  const raw = parsed.raw ?? [parsed.header, parsed.body, parsed.footer].filter(Boolean).join("\n");
  const lines = raw.split("\n");
  const first = lines.findIndex((line) => TRAILER_LINE.test(line.trimEnd()));
  if (first <= 0) return [true, ""];
  if ((lines[first - 1] ?? "").trim() === "") return [true, ""];
  return [
    false,
    `the trailer block must be preceded by a blank line; found "${(lines[first] ?? "").trim()}"`,
  ];
};

// Closed vocabulary. Grow deliberately; per-surface growth is the failure mode.
//
// delta: the scope list is per project. Crate scopes are the workspace's crate
// categories -- the `crates/<category>-*` prefix -- never a single crate
// directory, so a scope survives a crate split. Infra scopes name the surfaces
// that carry no crate.
const SCOPES = [
  "agents",
  "ci",
  "config",
  "core",
  "github",
  "kernel",
  "repo",
  "review",
  "storage",
  "support",
  "surface",
  "theory",
  "workflow",
];

// A harness-forensics trailer (`<harness>-Session:`) records which tool drove a
// commit. That is contributor-concern, never project-concern, and it outlives
// the session it points at, so it must never reach a published history. The
// pattern matches by shape rather than by a list of tool names, so a harness
// nobody here has heard of is refused on the same terms.
const HARNESS_TRAILER_LINE = /^[A-Za-z][A-Za-z0-9]*-Session:/i;

const noHarnessTrailer = (parsed) => {
  const raw = parsed.raw ?? [parsed.header, parsed.body, parsed.footer].filter(Boolean).join("\n");
  const offenders = raw.split("\n").filter((line) => HARNESS_TRAILER_LINE.test(line.trim()));
  if (offenders.length === 0) return [true, ""];
  return [
    false,
    `harness trailers are contributor-concern: ${offenders.map((line) => line.trim()).join(", ")}`,
  ];
};

// The author git will record, resolved the way a prepare-commit-msg hook
// resolves it: `git var GIT_AUTHOR_IDENT` honours GIT_AUTHOR_* exported by
// rebase and cherry-pick, so a replayed owner commit stays exempt and a
// replayed agent commit stays bound.
const commitAuthor = () => {
  try {
    return execFileSync("git", ["var", "GIT_AUTHOR_IDENT"], { encoding: "utf8" }).trim();
  } catch {
    return "";
  }
};

// An agent commit carries a three-line provenance block: the broad role, the
// opaque session token, and the owner co-author line, in that order. The three
// rules below enforce one side each. None of them repeats the harness-trailer
// refusal above, which owns the plaintext-forensics class by shape.
const sessionTrailerRequired = (parsed) => {
  const raw = parsed.raw ?? "";

  // Every Session line must be opaque — one valid line must not mask a
  // malformed or plaintext sibling. Multiple valid lines stay legal: squash
  // messages concatenate the branch commits' messages. The form check binds
  // whatever the author: a broken trailer is broken.
  const sessions = raw.split("\n").filter((line) => /^Session:/i.test(line.trimEnd()));
  if (sessions.length > 0) {
    const bad = sessions.find((line) => !/^Session: 1\.[A-Za-z0-9_-]+$/.test(line.trimEnd()));
    if (bad) return [false, `malformed Session trailer "${bad.trim()}": opaque form required`];
    return [true, ""];
  }

  // No Session line: the obligation binds agent authors only. An owner-solo
  // commit sits outside the block and carries no token, so a non-agent author
  // passes; the resolution honours the ambient GIT_AUTHOR_*, so a replayed
  // owner commit stays exempt under rebase and cherry-pick.
  if (!commitAuthor().startsWith("agent-")) return [true, ""];
  return [
    false,
    "commit needs an opaque Session trailer beside its Role and owner co-author lines",
  ];
};

// The three roles the Role trailer shares with the frontmatter an agent post
// opens with: broad, non-identifying, keyed to the assignment the commit
// executes.
const ROLES = ["coordinator", "reviewer", "worker"];
const ROLE_LINE = new RegExp(`^Role: (?:${ROLES.join("|")})$`);

// The Role trailer rides beside the Session trailer on agent commits. Its form
// check binds whatever the author — a broken trailer is broken — and its
// presence binds agent authors only, exactly as the Session side does. Multiple
// valid lines stay legal for the same reason: squash messages concatenate the
// branch commits' messages.
const sessionRoleRequired = (parsed) => {
  const raw = parsed.raw ?? "";
  const roles = raw.split("\n").filter((line) => /^Role:/i.test(line.trimEnd()));
  if (roles.length > 0) {
    const bad = roles.find((line) => !ROLE_LINE.test(line.trimEnd()));
    if (bad)
      return [false, `malformed Role trailer "${bad.trim()}": one of ${ROLES.join(", ")} required`];
    return [true, ""];
  }
  if (!commitAuthor().startsWith("agent-")) return [true, ""];
  return [false, `commit needs a Role trailer, one of ${ROLES.join(", ")}`];
};

const OWNER_COAUTHOR = "Co-authored-by: silvanshade <silvanshade@users.noreply.github.com>";

// The Session trailer pairs with the Role line and the owner co-author line:
// the three-line block above. No Session line -> owner-solo commit, no
// obligation (sessionTrailerRequired owns the Session side). With any Session
// line, exactly one owner line — a multi-Session (squash-concatenated) message
// carries the owner line once, not once per session — and one Role line per
// Session line, so an authored commit carries exactly one of each and a squash
// carries the pairs it concatenated.
const sessionCoauthorPairing = (parsed) => {
  const raw = parsed.raw ?? "";
  const lines = raw.split("\n").map((line) => line.trimEnd());
  const sessions = lines.filter((line) => /^Session:/i.test(line));
  if (sessions.length === 0) return [true, ""];

  const owners = lines.filter((line) => /^Co-authored-by:[ \t]+silvanshade/i.test(line));
  if (owners.length === 0)
    return [
      false,
      `Session trailer requires the owner co-author line crediting the coordinating owner: ${OWNER_COAUTHOR}`,
    ];
  const bad = owners.find((line) => line !== OWNER_COAUTHOR);
  if (bad) return [false, `malformed owner co-author line "${bad}": exact form required`];
  if (owners.length > 1)
    return [false, `exactly one owner co-author line per commit, found ${owners.length}`];
  const roles = lines.filter((line) => /^Role:/i.test(line));
  if (roles.length !== sessions.length)
    return [
      false,
      `each Session trailer pairs with one Role trailer: found ${roles.length} for ${sessions.length}`,
    ];
  return [true, ""];
};

export default {
  extends: ["@commitlint/config-conventional"],
  plugins: [
    {
      rules: {
        "trailer-leading-blank": trailerLeadingBlank,
        "no-harness-trailer": noHarnessTrailer,
        "session-trailer-required": sessionTrailerRequired,
        "session-role-required": sessionRoleRequired,
        "session-coauthor-pairing": sessionCoauthorPairing,
      },
    },
  ],
  rules: {
    "header-max-length": [2, "always", 72],
    "header-trim": [2, "always"],
    "subject-empty": [2, "never"],
    "subject-full-stop": [2, "never", "."],
    "body-leading-blank": [2, "always"],
    "body-max-line-length": [2, "always", 100],
    // Disabled: the conventional-commits parser reclassifies wrapped prose
    // bodies as footer whenever a line starts with `word:`; the custom
    // trailer-leading-blank rule above is the sound replacement.
    "footer-leading-blank": [0, "always"],
    "trailer-leading-blank": [2, "always"],
    "no-harness-trailer": [2, "always"],
    "session-trailer-required": [2, "always"],
    "session-role-required": [2, "always"],
    "session-coauthor-pairing": [2, "always"],
    // Stock conventional types plus config, for changes to the repository's
    // configuration surfaces (lint vocabularies, tool settings).
    "type-enum": [
      2,
      "always",
      [
        "build",
        "chore",
        "ci",
        "config",
        "docs",
        "feat",
        "fix",
        "perf",
        "refactor",
        "revert",
        "style",
        "test",
      ],
    ],
    "type-empty": [2, "never"],
    "scope-empty": [2, "never"],
    "scope-case": [2, "always", "lower-case"],
    "scope-enum": [2, "always", SCOPES],
  },
};
