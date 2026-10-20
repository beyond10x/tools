//! Adversary cases for `process.run`, each driven from a sentence of
//! `contracts/process/v1alpha1/semantics.md`.
//!
//! The `adversary-*.yaml` scenarios under `contracts/process/v1alpha1/scenarios/` join the suite
//! the conformance run reads. Each also has a test here that runs it alone, so one case can be
//! run by name. The two tests after them measure what a scenario cannot: how long the call was
//! held open, and whether the descendant is still alive once it returned.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use tool_contract::conformance::{Suite, declared_properties, run};
use tool_process::{INHERITED_ENV, Process, RunInput, contract};

const DECLARED: [&str; 3] = ["sh", "printf", "b10x-conformance-absent-program"];

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

/// Runs the one scenario `name` of the suite in `contracts/`, against the configuration
/// `semantics.md` names. The scenario is given every behavioural property in `holds` in memory
/// only, so that a suite of one passes the coverage rule; nothing else about it changes.
fn alone(name: &str) {
    let mut suite = Suite::read_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/process/v1alpha1/scenarios"),
    )
    .expect("the process suite parses");
    suite.scenarios.retain(|scenario| scenario.name == name);
    assert_eq!(suite.scenarios.len(), 1, "{name} is not in the suite");
    let profile = &contract()
        .operation("process.run")
        .expect("declared")
        .profile;
    suite.scenarios[0].holds = declared_properties(profile)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let scratch = scratch();
    let report = run(&suite, contract(), scratch.path(), |root| {
        Box::new(
            Process::new(root, DECLARED.iter().map(|p| (*p).to_owned()).collect())
                .expect("the scenario workspace opens"),
        )
    });
    println!("{report}");
    assert!(report.passed(), "{report}");
}

#[test]
fn scenario_a_descendant_holding_the_output_after_its_shell_ended_is_killed_at_the_bound() {
    alone("adversary-a-descendant-holding-the-output-after-its-shell-ended-is-killed-at-the-bound");
}

#[test]
fn scenario_the_head_cut_lands_between_characters_after_a_byte_that_is_not_utf_8() {
    alone("adversary-the-head-cut-lands-between-characters-after-a-byte-that-is-not-utf-8");
}

#[test]
fn scenario_one_byte_over_the_cap_is_cut_and_the_marker_names_one_byte() {
    alone("adversary-one-byte-over-the-cap-is-cut-and-the-marker-names-one-byte");
}

#[test]
fn scenario_both_streams_over_the_cap_are_each_cut_and_omitted_bytes_is_their_sum() {
    alone("adversary-both-streams-over-the-cap-are-each-cut-and-omitted-bytes-is-their-sum");
}

/// `semantics.md`: "Both cuts land between UTF-8 characters, so neither end opens or closes on a
/// broken character." The same stream as the scenario above: one byte that is not UTF-8 first,
/// then a three-byte character across the head's cut.
#[test]
fn the_head_does_not_close_on_a_broken_character_after_a_byte_that_is_not_utf_8() {
    let inside = scratch();
    let process = Process::new(inside.path(), vec!["sh".to_owned()]).expect("the workspace opens");
    let output = process
        .run(
            &RunInput::argv([
                "sh",
                "-c",
                "a=a; b=b; i=0\n\
                 while [ \"$i\" -lt 15 ]; do a=\"$a$a\"; b=\"$b$b\"; i=$((i + 1)); done\n\
                 printf '\\351%s€%s%s' \"${a#aa}\" \"$b\" \"$b\"",
            ]),
            None,
        )
        .expect("the program runs");
    let marker = output
        .stdout
        .find("\n… ")
        .expect("the stream was cut and marked");
    let head = &output.stdout[..marker];
    assert!(
        head.starts_with('\u{fffd}'),
        "the stream's own byte: {:?}",
        &head[..8]
    );
    assert!(
        !head.ends_with('\u{fffd}'),
        "the head closes on the character the cut broke: {:?}",
        &output.stdout[marker - 8..marker + 40]
    );
}

/// `semantics.md`: the environment is "given, by name, only the variables on an allow-list that
/// are set in the caller's own environment". Every allow-listed name set here reaches the child
/// with its value unchanged; the suite held `PATH` alone, so dropping any other name from the list
/// passed it.
#[test]
fn every_allow_listed_variable_set_here_reaches_the_child_unchanged() {
    let inside = scratch();
    let process = Process::new(inside.path(), vec!["env".to_owned()]).expect("the workspace opens");
    let output = process
        .run(&RunInput::argv(["env"]), None)
        .expect("the program runs");
    let mut checked = Vec::new();
    for name in INHERITED_ENV {
        let Some(value) = std::env::var_os(name) else {
            continue;
        };
        let Some(value) = value.to_str() else {
            continue;
        };
        assert!(
            output
                .stdout
                .lines()
                .any(|line| line == format!("{name}={value}")),
            "{name} is set here and did not reach the child: {}",
            output.stdout
        );
        checked.push(*name);
    }
    println!("checked: {checked:?}");
    assert!(
        checked.contains(&"PATH") && checked.contains(&"HOME"),
        "the test process holds at least PATH and HOME: {checked:?}"
    );
}

/// The pid a script wrote to `file` in the workspace, and whether that process is still alive.
/// A live one is killed here, so no case leaves a process behind.
#[cfg(unix)]
fn reap(root: &Path, file: &str) -> (i32, bool) {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    let text = fs::read_to_string(root.join(file)).expect("the script wrote the pid");
    let pid: i32 = text.trim().parse().expect("a pid");
    let alive = kill(Pid::from_raw(pid), None).is_ok();
    if alive {
        let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
    }
    (pid, alive)
}

/// `semantics.md`: "at the bound the **whole group** is killed, so a descendant that inherited
/// the output pipes, such as `sleep` started in the background by a shell, does not outlive the
/// call or hold it open." Here the shell ends by itself at once and its background `sleep`, in
/// the same process group, keeps both pipes open past the bound.
#[cfg(unix)]
#[test]
fn a_descendant_in_the_group_that_outlives_its_shell_does_not_hold_the_call_past_the_bound() {
    let inside = scratch();
    let process = Process::new(inside.path(), vec!["sh".to_owned()]).expect("the workspace opens");

    let started = Instant::now();
    let output = process
        .run(
            &RunInput::argv([
                "sh",
                "-c",
                "sleep 3 & printf '%s' \"$!\" > descendant.pid; printf started",
            ])
            .within(300),
            None,
        )
        .expect("the program runs");
    let held = started.elapsed();
    let (pid, alive) = reap(inside.path(), "descendant.pid");

    assert!(
        held < Duration::from_millis(1_500),
        "the call was held {held:?} against a bound of {} ms, by a descendant in the run's own \
         process group: {output:?}",
        output.timeout_ms
    );
    assert!(
        !alive,
        "descendant {pid} was still alive after the call returned"
    );
}

/// The same sentence, for a descendant that left the group: a shell with job control on
/// (`set -m`, POSIX) starts each job in a process group of its own, so the group kill at the
/// bound does not reach it, and it keeps both pipes open.
#[cfg(unix)]
#[test]
fn a_descendant_that_left_the_group_does_not_hold_the_call_past_the_bound() {
    let inside = scratch();
    let process = Process::new(inside.path(), vec!["sh".to_owned()]).expect("the workspace opens");

    let started = Instant::now();
    let output = process
        .run(
            &RunInput::argv([
                "sh",
                "-c",
                "set -m; sleep 3 & printf '%s' \"$!\" > escaped.pid; exec sleep 30",
            ])
            .within(300),
            None,
        )
        .expect("the program runs");
    let held = started.elapsed();
    let _ = reap(inside.path(), "escaped.pid");

    assert!(output.timed_out, "{output:?}");
    assert!(
        held < Duration::from_millis(1_500),
        "the call was held {held:?} against a bound of {} ms, by a descendant outside the \
         run's process group: {output:?}",
        output.timeout_ms
    );
}

/// `semantics.md`: "a workspace root that cannot be opened or is no longer a directory is
/// `process.WorkspaceUnavailable` naming its `path`, checked before a start, so a removed
/// workspace does not read as a missing program." Here the root is still a directory but can no
/// longer be entered: its search permission was taken away after it opened.
#[cfg(unix)]
#[test]
fn a_workspace_that_can_no_longer_be_entered_is_unavailable_and_not_a_spawn_failure() {
    use std::os::unix::fs::PermissionsExt as _;

    let inside = scratch();
    let root = inside.path().join("closed");
    fs::create_dir(&root).expect("the directory is made");
    let process = Process::new(&root, vec!["sh".to_owned()]).expect("the workspace opens");
    fs::set_permissions(&root, fs::Permissions::from_mode(0o600))
        .expect("the search permission is taken away");
    let probe = std::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .current_dir(&root)
        .status();
    let answer = process.run(&RunInput::argv(["sh", "-c", "exit 0"]), None);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
        .expect("the search permission is given back");

    assert!(
        probe.is_err(),
        "precondition: this user can still enter a directory it has no search permission on \
         (root?), so the state under test was not built: {probe:?}"
    );
    let refused = answer.expect_err("refused");
    assert_eq!(refused.name(), "process.WorkspaceUnavailable", "{refused}");
}
