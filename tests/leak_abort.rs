#![cfg(all(unix, not(miri)))]

#[cfg(test)]
mod tests
{
    use std::os::unix::process::ExitStatusExt as _;
    use std::process::Command;

    #[test]
    #[expect(
        clippy::mem_forget,
        reason = "exercise the production abort on leaked ownership"
    )]
    fn forgetting_a_move_ref_aborts_the_process()
    {
        const CHILD: &str = "MOVEREF_ABORT_TEST_CHILD";
        if std::env::var_os(CHILD).is_some() {
            moveref::bind!(moved = &move 7_i32);
            core::mem::forget(moved);
            return;
        }

        let executable = std::env::current_exe()
            .unwrap_or_else(|error| panic!("cannot locate the test executable: {error}"));
        let output = Command::new(executable)
            .args(["--exact", "tests::forgetting_a_move_ref_aborts_the_process"])
            .env(CHILD, "1")
            .output()
            .unwrap_or_else(|error| panic!("cannot launch the abort witness: {error}"));
        assert_eq!(
            output.status.signal(),
            Some(6_i32),
            "leaked MoveRef must terminate with SIGABRT: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
