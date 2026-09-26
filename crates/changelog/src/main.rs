//! Render the root changelog from Git history and the review's eventual squash
//! subject.

use std::env;
use std::fs;
use std::process::Command;

use serde::Deserialize;

/// Process and filesystem failures leave the checked-in changelog unchanged.
type Failure = std::io::Error;

/// The only two histories admitted by the project release-note formatter.
enum RenderMode
{
    /// A landed default-branch history, already containing squash commits.
    Main,
    /// An open pull request whose current commits will be replaced by one
    /// squash commit.
    Review(PullRequest),
}

/// The PR's eventual subject and the exact base revision it builds upon.
struct PullRequest
{
    /// The GitHub number appended to the squash subject.
    number: u64,
    /// The PR title used by the squash merge.
    title: String,
    /// The base commit common to local rendering and a GitHub PR checkout.
    base_sha: String,
}

/// Fields read from a GitHub Actions pull-request event.
#[derive(Deserialize)]
struct Event
{
    /// Review metadata supplied by GitHub for this run.
    pull_request: EventPullRequest,
}

/// Pull-request metadata from the Actions event payload.
#[derive(Deserialize)]
struct EventPullRequest
{
    /// Review number used in the eventual squash subject.
    number: u64,
    /// Review title used in the eventual squash subject.
    title: String,
    /// Exact base revision recorded when this workflow was triggered.
    base: EventBase,
}

/// Base revision in an Actions pull-request event.
#[derive(Deserialize)]
struct EventBase
{
    /// Commit hash to use as the beginning of the review range.
    sha: String,
}

/// Review metadata returned by the pinned GitHub CLI for a local branch.
#[derive(Deserialize)]
struct LocalPullRequest
{
    /// Review number used in the eventual squash subject.
    number: u64,
    /// Review title used in the eventual squash subject.
    title: String,
    /// Review base branch name, resolved against the live `origin` ref.
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
}

/// Execute a command whose output is needed to select the changelog history.
///
/// # Specification
/// - provides: the command's stdout bytes when it succeeds.
/// - fails: preserves the command's failure status and stderr in an error.
/// - panics: none.
///
/// # Errors
/// - Returns the process launch error, or the unsuccessful command status and
///   stderr.
///
/// # Adequacy
/// - hypothesis: The tagged Git fixture exercises successful captured output;
///   nonzero external commands and missing programs remain outside it.
/// - witness: `changelog::tests::review_matches_landed_squash`
fn output(command: &mut Command) -> Result<Vec<u8>, Failure>
{
    let result = command.output()?;
    if !result.status.success() {
        return Err(Failure::other(format!(
            "{} failed ({}): {}",
            command.get_program().to_string_lossy(),
            result.status,
            String::from_utf8_lossy(&result.stderr)
        )));
    }
    Ok(result.stdout)
}

/// Execute a command that writes its result itself, without capturing stdout.
///
/// # Specification
/// - provides: success only when the child exits successfully.
/// - fails: reports process launch and nonzero status failures.
/// - panics: none.
///
/// # Errors
/// - Returns the process launch error or the unsuccessful command status.
///
/// # Adequacy
/// - hypothesis: For a malformed renderer configuration, L3 checks the
///   git-cliff failure status and unchanged destination; unavailable
///   executables remain outside this fixture.
/// - witness: `changelog::tests::review_matches_landed_squash`
fn run(command: &mut Command) -> Result<(), Failure>
{
    let status = command.status()?;
    if !status.success() {
        return Err(Failure::other(format!(
            "{} failed ({status})",
            command.get_program().to_string_lossy()
        )));
    }
    Ok(())
}

/// Select the review recorded in a CI event or by the checked-out local branch.
///
/// # Specification
/// - requires: a local branch is at or behind `origin/main`, has an open PR and
///   reachable `origin`, or this is a `main`/merge-group event.
/// - provides: the live review base and squash subject, or plain landed
///   history.
/// - fails: reports an absent PR/base branch, missing event, invalid JSON, or
///   Git/GitHub CLI failure.
/// - panics: none.
///
/// # Errors
/// - Returns missing-event, invalid-JSON, invalid-UTF-8, filesystem, Git, or
///   GitHub CLI errors, including an absent live `origin` branch.
///
/// # Adequacy
/// - hypothesis: On tagged Git fixtures, L2 compares review and landed
///   histories; L3 rejects a stale event base. Local CLI selection and
///   malformed event JSON remain outside this fixture.
/// - witness: `changelog::tests::review_matches_landed_squash`
fn render_mode() -> Result<RenderMode, Failure>
{
    match env::var("GITHUB_EVENT_NAME") {
        | Ok(event) if event == "pull_request" => {
            let path = env::var_os("GITHUB_EVENT_PATH")
                .ok_or_else(|| Failure::other("missing pull-request event path"))?;
            let event: Event = serde_json::from_slice(&fs::read(path)?).map_err(Failure::other)?;
            let pr = event.pull_request;
            Ok(RenderMode::Review(PullRequest {
                number: pr.number,
                title: pr.title,
                base_sha: pr.base.sha,
            }))
        },
        | Ok(event) if event == "push" || event == "merge_group" => Ok(RenderMode::Main),
        | _ => {
            let mut branch = Command::new("git");
            branch.args(["branch", "--show-current"]);
            if core::str::from_utf8(&output(&mut branch)?)
                .map_err(Failure::other)?
                .trim()
                == "main"
            {
                return Ok(RenderMode::Main);
            }
            let mut unchanged = Command::new("git");
            unchanged.args(["merge-base", "--is-ancestor", "HEAD", "origin/main"]);
            if unchanged.status()?.success() {
                return Ok(RenderMode::Main);
            }
            let mut view = Command::new("gh");
            view.args(["pr", "view", "--json", "number,title,baseRefName"]);
            let pr: LocalPullRequest =
                serde_json::from_slice(&output(&mut view)?).map_err(Failure::other)?;
            let mut base = Command::new("git");
            base.args([
                "ls-remote",
                "origin",
                &format!("refs/heads/{}", pr.base_ref_name),
            ]);
            let base_sha = core::str::from_utf8(&output(&mut base)?)
                .map_err(Failure::other)?
                .split_once('\t')
                .map(|(sha, _)| sha.to_owned())
                .ok_or_else(|| {
                    Failure::other(format!("missing live origin branch {}", pr.base_ref_name))
                })?;
            Ok(RenderMode::Review(PullRequest {
                number: pr.number,
                title: pr.title,
                base_sha,
            }))
        },
    }
}

/// Generate release notes using actual landed commits or one synthetic review
/// commit.
///
/// # Specification
/// - requires: all stable version tags and the PR base are present in the local
///   repository.
/// - provides: a Markdown candidate that excludes review-only commits and the
///   changelog itself.
/// - fails: rejects a stale/non-ancestor PR base, or failed Git, git-cliff, or
///   Markdown formatting.
/// - panics: none.
///
/// # Errors
/// - Returns a stale-base error containing its SHA, or the failed Git,
///   git-cliff, rumdl, or filesystem error.
///
/// # Adequacy
/// - hypothesis: On a tagged fixture, L2 review/squash byte equality
///   distinguishes wrong skipped commits or PR subjects; L3 stale ancestry and
///   malformed policy distinguish failure from silently reusing stale notes.
///   Tool unavailability is outside this fixture.
/// - witness: `changelog::tests::review_matches_landed_squash`
fn render(mode: &RenderMode) -> Result<(), Failure>
{
    let mut cliff = Command::new("git-cliff");
    cliff.args([
        "--config",
        "cliff.toml",
        "--offline",
        "--output",
        ".git-cliff-output",
        "--exclude-path",
        "CHANGELOG.md",
    ]);
    if let RenderMode::Review(ref pr) = *mode {
        let mut ancestor = Command::new("git");
        ancestor.args(["merge-base", "--is-ancestor", &pr.base_sha, "HEAD"]);
        let status = ancestor.status()?;
        if !status.success() {
            return Err(Failure::other(format!(
                "pull request base {} is not an ancestor of HEAD ({status})",
                pr.base_sha
            )));
        }

        let mut rev_list = Command::new("git");
        rev_list.args(["rev-list", &format!("{}..HEAD", pr.base_sha)]);
        let skipped = output(&mut rev_list)?;
        for revision in core::str::from_utf8(&skipped)
            .map_err(Failure::other)?
            .lines()
        {
            cliff.args(["--skip-commit", revision]);
        }
        cliff.args(["--with-commit", &format!("{} (#{})", pr.title, pr.number)]);
    }
    run(&mut cliff)?;
    run(Command::new("rumdl").args(["fmt", ".git-cliff-output"]))?;
    Ok(())
}

/// Install a changed candidate without rewriting an already-current changelog.
///
/// # Specification
/// - provides: the generated root file, leaving its bytes and metadata
///   untouched when current.
/// - fails: propagates unexpected read, removal, and rename failures.
/// - panics: none.
///
/// # Errors
/// - Returns unexpected read, delete, or rename errors; absence of the
///   destination is admitted.
///
/// # Adequacy
/// - hypothesis: For generated fixture notes, L3 forced old modification time
///   detects rewriting identical bytes, while L2 review/squash bytes detects
///   changed contents; permission failures remain outside the fixture.
/// - witness: `changelog::tests::review_matches_landed_squash`
#[expect(
    clippy::std_instead_of_core,
    reason = "core::io::ErrorKind is unstable"
)]
fn install() -> Result<(), Failure>
{
    let generated = fs::read(".git-cliff-output")?;
    match fs::read("CHANGELOG.md") {
        | Ok(existing) if existing == generated => {
            fs::remove_file(".git-cliff-output")?;
            return Ok(());
        },
        | Ok(_) => {},
        | Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
        | Err(error) => return Err(error),
    }
    fs::rename(".git-cliff-output", "CHANGELOG.md")?;
    Ok(())
}

/// Render the release history for the current review or landed branch.
///
/// # Specification
/// - requires: pinned CLI tools and Git version tags are available in the
///   checkout.
/// - provides: one current root changelog, matching the PR's eventual squash
///   history.
/// - fails: rejects absent review metadata and propagates Git, rendering, and
///   I/O errors.
/// - panics: none.
///
/// # Errors
/// - Returns selection, rendering, or installation errors and preserves the
///   prior root file when selection or rendering fails.
///
/// # Adequacy
/// - hypothesis: On a tagged fixture, L2 compares final PR and squash bytes,
///   and L3 observes stale-base and policy-error exits without changing the
///   destination; process-launch and permission errors are unexercised.
/// - witness: `changelog::tests::review_matches_landed_squash`
fn main() -> Result<(), Failure>
{
    let mode = render_mode()?;
    render(&mode)?;
    install()?;
    Ok(())
}
