//! Adversary cases for `process.run`, second pass: the correction's own sentences in
//! `contracts/process/v1alpha1/semantics.md`, driven against the code that answers them.
//!
//! The `adversary-*-2.yaml` scenarios join the suite the conformance run reads; each also has a
//! test here that runs it alone, so one case can be run by name.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use tool_contract::conformance::{Suite, declared_properties, run};
use tool_process::{Process, RunInput, contract};

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

/// `semantics.md`: "When the bound is reached first, the **whole group** is killed, whether or not
/// the program itself has already ended". The descendant writes after the bound and inside the
/// half second the run then waits, so only a kill keeps its word out of the reply; the suite's
/// own scenario for this case has its descendant write after the settle, where a run that never
/// killed it answers the same.
#[test]
fn scenario_a_descendant_left_by_a_shell_that_ended_is_killed_at_the_bound_not_only_left_behind() {
    alone(
        "adversary-a-descendant-left-by-a-shell-that-ended-is-killed-at-the-bound-not-only-left-behind",
    );
}

/// `semantics.md`: "Only a character the cut itself broke moves it". The head's last byte is one
/// that is not UTF-8 and opens no character with what follows it in the stream.
#[test]
fn scenario_a_byte_that_is_not_utf_8_closing_the_head_is_answered_and_not_moved() {
    alone("adversary-a-byte-that-is-not-utf-8-closing-the-head-is-answered-and-not-moved");
}

/// The same sentence at the other end: the tail's first byte is a stray continuation byte that
/// follows a whole character in the stream, so the cut broke nothing.
#[test]
fn scenario_a_byte_that_is_not_utf_8_opening_the_tail_is_answered_and_not_moved() {
    alone("adversary-a-byte-that-is-not-utf-8-opening-the-tail-is-answered-and-not-moved");
}

/// `semantics.md`: "The run then waits briefly, at most half a second, for the streams to end and
/// the group to be gone". With what the run has left as the bound and a process outside the group
/// holding both streams, the call answers within that bound plus the half second.
#[cfg(unix)]
#[test]
fn a_run_answers_within_what_it_had_left_and_half_a_second_when_a_process_outside_the_group_holds_the_output()
 {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    let inside = scratch();
    let process = Process::new(inside.path(), vec!["sh".to_owned()]).expect("the workspace opens");
    let left = Duration::from_millis(200);

    let started = Instant::now();
    let output = process
        .run(
            &RunInput::argv([
                "sh",
                "-c",
                "set -m; sleep 3 & printf '%s' \"$!\" > escaped.pid; exec sleep 30",
            ]),
            Some(left),
        )
        .expect("the program runs");
    let held = started.elapsed();
    let pid: i32 = fs::read_to_string(inside.path().join("escaped.pid"))
        .expect("the script wrote the pid")
        .trim()
        .parse()
        .expect("a pid");
    let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);

    assert!(output.timed_out, "{output:?}");
    assert_eq!(output.timeout_ms, 200, "{output:?}");
    assert!(
        held < left + Duration::from_millis(500 + 250),
        "the call answered {held:?} after it started, with {left:?} left: {output:?}"
    );
}
