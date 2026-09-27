//! Exercise tagged history across a real pull-request-to-squash transition.

// Miri cannot launch the Git and git-cliff subprocesses this fixture exercises.
#[cfg(test)]
#[cfg(not(miri))]
mod tests
{
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    /// Errors from a real Git invocation or the release-note renderer.
    type Failure = Box<dyn core::error::Error + Send + Sync>;

    /// Arguments for an isolated fixture Git process.
    #[derive(Clone, Copy)]
    struct GitArgs<'command>(&'command [&'command str]);

    /// The two event histories exercised by the renderer fixture.
    #[derive(Clone, Copy)]
    enum Event
    {
        /// Replace transient review commits with the proposed squash subject.
        PullRequest,
        /// Read the landed Git history directly.
        Push,
    }

    /// Run a Git fixture operation, surfacing its exact failure.
    ///
    /// # Specification
    /// - provides: stdout from a successful Git command in the isolated
    ///   repository.
    /// - fails: includes status and stderr if Git rejects the operation.
    /// - panics: none.
    ///
    /// # Errors
    /// - Returns process, UTF-8, or unsuccessful Git status errors.
    ///
    /// # Adequacy
    /// - hypothesis: L3 constructs a tagged history and verifies Git operations
    ///   succeeded before observing the renderer; unavailable Git is outside
    ///   this fixture.
    /// - witness: `tests::review_matches_landed_squash`
    fn git(
        repo: &Path,
        args: GitArgs<'_>,
    ) -> Result<String, Failure>
    {
        let result = Command::new("git")
            .args(args.0)
            .current_dir(repo)
            .output()?;
        if !result.status.success() {
            return Err(format!(
                "git failed ({}): {}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            )
            .into());
        }
        Ok(String::from_utf8(result.stdout)?)
    }

    /// Commit a real content change so git-cliff's path filter retains it.
    ///
    /// # Specification
    /// - ensures: the given content and subject become the fixture's next
    ///   commit.
    /// - fails: propagates filesystem or Git failures.
    /// - panics: none.
    ///
    /// # Errors
    /// - Returns write or Git errors.
    ///
    /// # Adequacy
    /// - hypothesis: L2 compares genuine file-changing review and squash
    ///   commits, rather than path-filtered empty commits.
    /// - witness: `tests::review_matches_landed_squash`
    fn commit(
        repo: &Path,
        content: &str,
        subject: &str,
    ) -> Result<(), Failure>
    {
        fs::write(repo.join("source.txt"), content)?;
        git(repo, GitArgs(&["add", "source.txt"]))?;
        git(repo, GitArgs(&["commit", "-qm", subject]))?;
        Ok(())
    }

    /// Run the actual renderer and return the resulting project changelog.
    ///
    /// # Specification
    /// - provides: generated Markdown from the fixture's tagged history.
    /// - fails: surfaces process or renderer errors, or an unreadable output.
    /// - panics: none.
    ///
    /// # Errors
    /// - Returns process, renderer-status, or read errors.
    ///
    /// # Adequacy
    /// - hypothesis: L2 compares review and landed-squash bytes; L3 checks
    ///   selected clauses against expected sections and subjects.
    /// - witness: `tests::review_matches_landed_squash`
    fn render(
        repo: &Path,
        event: Event,
    ) -> Result<String, Failure>
    {
        let event_name = match event {
            | Event::PullRequest => "pull_request",
            | Event::Push => "push",
        };
        let result = Command::new(env!("CARGO_BIN_EXE_moveref-changelog"))
            .current_dir(repo)
            .env("GITHUB_EVENT_NAME", event_name)
            .env("GITHUB_EVENT_PATH", repo.join("event.json"))
            .output()?;
        if !result.status.success() {
            return Err(format!(
                "changelog render failed ({}): {}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            )
            .into());
        }
        Ok(fs::read_to_string(repo.join("CHANGELOG.md"))?)
    }

    #[test]
    fn review_matches_landed_squash() -> Result<(), Failure>
    {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let repo =
            std::env::temp_dir().join(format!("moveref-changelog-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&repo)?;
        fs::write(repo.join("cliff.toml"), include_str!("../../../cliff.toml"))?;
        git(&repo, GitArgs(&["init", "-q", "-b", "main"]))?;
        git(&repo, GitArgs(&["config", "user.name", "Release fixture"]))?;
        git(
            &repo,
            GitArgs(&["config", "user.email", "fixture@example.invalid"]),
        )?;
        commit(&repo, "seed", "chore(repo): seed")?;
        git(&repo, GitArgs(&["tag", "v1.0.0"]))?;
        commit(&repo, "parent", "fix(core): deployed parent")?;
        let base = git(&repo, GitArgs(&["rev-parse", "HEAD"]))?;
        git(&repo, GitArgs(&["switch", "-qc", "review"]))?;
        commit(&repo, "experiment", "feat(core): transient experiment")?;
        commit(&repo, "adjustment", "fix(core): transient adjustment")?;
        let event = serde_json::json!({
            "pull_request": {
                "number": 17_u64,
                "title": "feat(core): add documented move",
                "base": { "sha": base.trim() }
            }
        });
        fs::write(repo.join("event.json"), serde_json::to_vec(&event)?)?;

        let review = render(&repo, Event::PullRequest)?;
        assert!(review.lines().any(|line| line == "## Unreleased"));
        assert!(
            review
                .lines()
                .any(|line| line == "- _(core)_ Add documented move (#17)")
        );
        assert!(
            review
                .lines()
                .any(|line| line == "- _(core)_ Deployed parent")
        );
        assert!(review.contains("## 1.0.0 - "));
        assert!(!review.contains("Transient experiment"));
        assert!(!review.contains("Transient adjustment"));

        let changelog = repo.join("CHANGELOG.md");
        let file = fs::File::options().write(true).open(&changelog)?;
        file.set_times(fs::FileTimes::new().set_modified(UNIX_EPOCH))?;
        render(&repo, Event::PullRequest)?;
        assert_eq!(fs::metadata(&changelog)?.modified()?, UNIX_EPOCH);

        git(&repo, GitArgs(&["switch", "-q", "main"]))?;
        commit(&repo, "squashed", "feat(core): add documented move (#17)")?;
        let landed = render(&repo, Event::Push)?;
        assert_eq!(review, landed, "review and landed histories differ");
        fs::write(&changelog, "stale\n")?;
        assert_eq!(render(&repo, Event::Push)?, landed);

        let config = repo.join("cliff.toml");
        let hidden = repo.join("cliff.off");
        fs::rename(&config, &hidden)?;
        fs::write(&config, "[changelog\n")?;
        let invalid = Command::new(env!("CARGO_BIN_EXE_moveref-changelog"))
            .current_dir(&repo)
            .env("GITHUB_EVENT_NAME", "push")
            .output()?;
        assert!(!invalid.status.success());
        assert!(
            String::from_utf8_lossy(&invalid.stderr).contains("git-cliff failed"),
            "{}",
            String::from_utf8_lossy(&invalid.stderr)
        );
        assert_eq!(fs::read_to_string(&changelog)?, landed);
        fs::remove_file(&config)?;
        fs::rename(hidden, config)?;

        git(&repo, GitArgs(&["switch", "-q", "review"]))?;
        let bad_base = git(&repo, GitArgs(&["rev-parse", "main"]))?;
        let stale_event = serde_json::json!({
            "pull_request": {
                "number": 17_u64,
                "title": "feat(core): add documented move",
                "base": { "sha": bad_base.trim() }
            }
        });
        fs::write(repo.join("event.json"), serde_json::to_vec(&stale_event)?)?;
        let stale = Command::new(env!("CARGO_BIN_EXE_moveref-changelog"))
            .current_dir(&repo)
            .env("GITHUB_EVENT_NAME", "pull_request")
            .env("GITHUB_EVENT_PATH", repo.join("event.json"))
            .output()?;
        assert!(!stale.status.success());
        assert!(
            String::from_utf8_lossy(&stale.stderr).contains(&format!(
                "pull request base {} is not an ancestor of HEAD",
                bad_base.trim()
            )),
            "{}",
            String::from_utf8_lossy(&stale.stderr)
        );
        assert_eq!(fs::read_to_string(&changelog)?, landed);
        fs::remove_dir_all(repo)?;
        Ok(())
    }
}
