//! Adversary cases for `process.run`, third pass: what the run takes as the program's end now that
//! it asks `waitid` without reaping, instead of `Child::try_wait`.
//!
//! `contract.yaml` says `exit` is "null when it was ended by a signal"; nothing in the suite has a
//! program end by a signal of its own before the bound, or stop itself, so what the new `waitid`
//! answer is matched against is held only here.

#![cfg(unix)]

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tool_process::{Process, RunInput, RunOutput};

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

/// Runs `script` under `sh -c` with `timeout_ms` on another thread and answers the reply, or
/// `None` when the call has not answered within `limit`.
fn answered_within(
    root: &std::path::Path,
    script: &str,
    timeout_ms: u64,
    limit: Duration,
) -> Option<RunOutput> {
    let process = Process::new(root, vec!["sh".to_owned()]).expect("the workspace opens");
    let input = RunInput::argv(["sh", "-c", script]).within(timeout_ms);
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let _ = send.send(process.run(&input, None).expect("the program runs"));
    });
    receive.recv_timeout(limit).ok()
}

/// A program that kills itself is ended, and the run answers when it is, with no exit code and not
/// timed out: `waitid` reports a death by a signal as an end like an exit.
#[test]
fn a_program_ended_by_its_own_signal_is_answered_at_once_and_not_as_timed_out() {
    let inside = scratch();
    let started = Instant::now();
    let output = answered_within(
        inside.path(),
        "kill -KILL $$",
        3_000,
        Duration::from_secs(10),
    )
    .expect("the run answers");
    let took = started.elapsed();
    assert!(
        !output.timed_out,
        "a program dead by its own signal was waited on to the bound: {output:?}"
    );
    assert_eq!(output.exit, None, "ended by a signal: {output:?}");
    assert!(
        took < Duration::from_millis(1_500),
        "answered after {took:?}, not when the program died"
    );
}

/// A program that stops itself has not ended: with both streams closed it is still the run, so it
/// is killed at the bound and the reply says so. A stop taken as an end would leave the run waiting
/// to reap a process that never exits.
#[test]
fn a_program_that_stops_itself_is_not_ended_and_is_killed_at_the_bound() {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;

    let inside = scratch();
    let answer = answered_within(
        inside.path(),
        "echo $$ > pid; exec >&- 2>&-; kill -STOP $$",
        300,
        Duration::from_secs(10),
    );
    let Some(output) = answer else {
        if let Some(pid) = std::fs::read_to_string(inside.path().join("pid"))
            .ok()
            .and_then(|text| text.trim().parse::<i32>().ok())
        {
            let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
        }
        panic!("the run did not answer within 10 s of a 300 ms bound: a stop was taken as an end");
    };
    assert!(output.timed_out, "{output:?}");
    assert_eq!(output.exit, None, "killed at the bound: {output:?}");
}
