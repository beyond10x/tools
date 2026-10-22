//! The adversary's cases for `story:declared-timeouts-are-enforced` (`fs` v1alpha2), written
//! against `contracts/fs/v1alpha2/semantics.md` § The deadline before anything was run.
//!
//! The scenario cases are `contracts/fs/v1alpha2/scenarios/adversary-deadline.yaml`; a test here
//! picks one out of the merged suite's report so that it can be run and read alone.
//!
//! The direct cases drive a real clock through the public API. The unit's own cases stop a loop
//! with `Deadline::after_checks` or with `timeout_ms: 0`, so none of them would notice a deadline
//! that is never reached for any `timeout_ms` above 0 (a `Duration::from_secs` where
//! `from_millis` was meant, say). Each direct case first shows the workload takes longer than
//! the deadline it then names, so a deadline that passes in the middle of the work is the only
//! honest answer.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tool_contract::conformance::{Report, Suite, run};
use tool_fs::{Fs, GlobInput, ListInput, ReadInput, StatInput, contract};

/// The most a call named `timeout_ms: 1` may take here: far above 1 ms so a loaded machine does
/// not fail it, and below the 1 s a seconds-for-milliseconds slip would give the call.
const NEAR_ITS_BOUND: Duration = Duration::from_millis(500);
/// The least the uncut workload must take for a 1 ms deadline to be one it has to reach.
const SLOWER_THAN_THE_BOUND: Duration = Duration::from_millis(5);

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/fs/v1alpha2")
}

fn scratch() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory under target/")
}

fn suite_report() -> Report {
    let suite = Suite::read_dir(contract_dir().join("scenarios")).expect("the fs suite parses");
    let scratch = scratch();
    run(&suite, contract(), scratch.path(), |root| {
        Box::new(Fs::new(root).expect("the scenario workspace opens"))
    })
}

fn scenario_passes(name: &str) {
    let report = suite_report();
    assert!(
        report.refused.is_empty(),
        "the suite was refused:\n{report}"
    );
    let result = report
        .scenarios
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("{name} is not a scenario of the suite"));
    assert!(result.passed, "FAIL {name}: {}", result.detail);
}

fn timed<T>(call: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let answer = call();
    (answer, started.elapsed())
}

// semantics.md § The deadline: "Path resolution and every refusal come first, so a call that
// would be refused is refused whatever its deadline."
#[test]
fn a_path_that_climbs_out_is_refused_for_the_path_whatever_its_deadline_asks() {
    scenario_passes(
        "adversary-a-path-that-climbs-out-is-refused-for-the-path-whatever-its-deadline-asks",
    );
}

// contract.yaml, `timeout_ms`: `Optional<Integer>`, "above 60000 is refused as fs.InvalidInput".
#[test]
fn a_deadline_that_is_not_a_non_negative_integer_is_not_the_operations_input() {
    scenario_passes("adversary-a-negative-deadline-is-not-the-operations-input");
    scenario_passes("adversary-a-deadline-given-as-a-string-is-not-the-operations-input");
}

// semantics.md § The deadline: "checked ... per line of a read's scan ... a slow call stops near
// its bound."
#[test]
fn a_read_with_a_deadline_of_1_ms_stops_near_it_on_a_file_that_takes_longer() {
    let dir = scratch();
    fs::write(dir.path().join("lines.txt"), "x\n".repeat(1_000_000)).expect("written");
    let local = Fs::new(dir.path()).expect("the workspace opens");

    let (whole, took) = timed(|| {
        local.read(&ReadInput {
            timeout_ms: Some(60_000),
            ..ReadInput::whole("lines.txt")
        })
    });
    let whole = whole.expect("the read answers");
    assert!(!whole.deadline_reached, "the uncut read finished");
    assert_eq!(whole.lines.total, Some(1_000_000));
    assert!(took >= SLOWER_THAN_THE_BOUND, "the workload took {took:?}");

    let (cut, took) = timed(|| {
        local.read(&ReadInput {
            timeout_ms: Some(1),
            ..ReadInput::whole("lines.txt")
        })
    });
    let cut = cut.expect("the read answers");
    assert!(cut.deadline_reached, "timeout_ms: 1 on a {took:?} read");
    assert!(took < NEAR_ITS_BOUND, "timeout_ms: 1 took {took:?}");
    assert_eq!(cut.lines.total, None);
    assert_eq!(cut.sha256, None);
    assert!(cut.truncated);
}

// semantics.md § The deadline: "per entry of a listing".
#[test]
fn a_listing_with_a_deadline_of_1_ms_stops_near_it_on_a_directory_that_takes_longer() {
    let dir = scratch();
    for n in 0..50_000 {
        fs::write(dir.path().join(format!("{n:05}")), "").expect("written");
    }
    let local = Fs::new(dir.path()).expect("the workspace opens");

    let (whole, took) = timed(|| {
        local.list(&ListInput {
            timeout_ms: Some(60_000),
            ..ListInput::default()
        })
    });
    let whole = whole.expect("the listing answers");
    assert!(!whole.deadline_reached, "the uncut listing finished");
    assert_eq!(
        whole.entries.first().map(|e| e.name.as_str()),
        Some("00000")
    );
    assert!(took >= SLOWER_THAN_THE_BOUND, "the workload took {took:?}");

    let (cut, took) = timed(|| {
        local.list(&ListInput {
            timeout_ms: Some(1),
            ..ListInput::default()
        })
    });
    let cut = cut.expect("the listing answers");
    assert!(cut.deadline_reached, "timeout_ms: 1 on a {took:?} listing");
    assert!(took < NEAR_ITS_BOUND, "timeout_ms: 1 took {took:?}");
    assert!(cut.truncated);
}

// semantics.md § The deadline: "per entry ... of the walk".
#[test]
fn a_glob_with_a_deadline_of_1_ms_stops_near_it_on_a_tree_that_takes_longer() {
    let dir = scratch();
    for d in 0..100 {
        let sub = dir.path().join(format!("d{d:03}"));
        fs::create_dir(&sub).expect("created");
        for n in 0..200 {
            fs::write(sub.join(format!("f{n:03}.txt")), "").expect("written");
        }
    }
    let local = Fs::new(dir.path()).expect("the workspace opens");

    let (whole, took) = timed(|| {
        local.glob(&GlobInput {
            timeout_ms: Some(60_000),
            ..GlobInput::new("*.none")
        })
    });
    let whole = whole.expect("the glob answers");
    assert!(!whole.deadline_reached, "the uncut walk finished");
    assert!(!whole.truncated);
    assert!(took >= SLOWER_THAN_THE_BOUND, "the workload took {took:?}");

    let (cut, took) = timed(|| {
        local.glob(&GlobInput {
            timeout_ms: Some(1),
            ..GlobInput::new("*.none")
        })
    });
    let cut = cut.expect("the glob answers");
    assert!(cut.deadline_reached, "timeout_ms: 1 on a {took:?} walk");
    assert!(took < NEAR_ITS_BOUND, "timeout_ms: 1 took {took:?}");
    assert!(cut.truncated);
}

// semantics.md § The deadline: "per chunk of fs.stat's digest".
#[test]
fn a_stat_with_a_deadline_of_1_ms_stops_near_it_on_a_digest_that_takes_longer() {
    let dir = scratch();
    fs::write(dir.path().join("big.bin"), vec![7_u8; 16 * 1024 * 1024]).expect("written");
    let local = Fs::new(dir.path()).expect("the workspace opens");

    let (whole, took) = timed(|| {
        local.stat(&StatInput {
            timeout_ms: Some(60_000),
            ..StatInput::new("big.bin")
        })
    });
    let whole = whole.expect("stat answers");
    assert!(!whole.deadline_reached, "the uncut digest finished");
    assert!(whole.sha256.is_some());
    assert!(took >= SLOWER_THAN_THE_BOUND, "the workload took {took:?}");

    let (cut, took) = timed(|| {
        local.stat(&StatInput {
            timeout_ms: Some(1),
            ..StatInput::new("big.bin")
        })
    });
    let cut = cut.expect("stat answers");
    assert!(cut.deadline_reached, "timeout_ms: 1 on a {took:?} digest");
    assert!(took < NEAR_ITS_BOUND, "timeout_ms: 1 took {took:?}");
    assert_eq!(cut.sha256, None);
    assert_eq!(cut.bytes, Some(16 * 1024 * 1024));
}

// Pass 2. The correction round moved the `timeout_ms` refusal after every refusal of the input
// (wave w4 D3). `adversary-deadline-order.yaml` holds each input refusal of each operation with
// `timeout_ms` 60001 and with 0; its names start `adversary2-` so the count of
// `tests/adversary.rs` is not this file's to move.
#[test]
fn every_input_refusal_of_every_operation_wins_over_the_deadline_at_60001_and_at_0() {
    let report = suite_report();
    assert!(
        report.refused.is_empty(),
        "the suite was refused:\n{report}"
    );
    let ours: Vec<_> = report
        .scenarios
        .iter()
        .filter(|s| s.name.starts_with("adversary2-order-"))
        .collect();
    assert_eq!(ours.len(), 28, "every order scenario ran");
    let failed: Vec<String> = ours
        .iter()
        .filter(|s| !s.passed)
        .map(|s| format!("{}: {}", s.name, s.detail))
        .collect();
    assert!(failed.is_empty(), "{failed:#?}");
}

// Pass 2. The correction round has the counting bound mark a line it stopped in as not whole
// "unless the file ends there or its newline is the next byte" (`line.rs`). `Line::whole` says
// why a dropped `\r` must not make a line read as cut: "a reply that called that line cut would
// name every line of a CRLF file". So a line the bound stopped in right before its `\r\n` is as
// whole as the same line stopped right before its `\n`.
#[test]
fn a_line_the_counting_bound_stops_before_its_crlf_is_as_whole_as_one_before_its_lf() {
    // 16383 lines of 1 KiB leave the scan 1025 bytes of budget: the 1025 bytes of line 16384,
    // and the bound falls right after them.
    let prefix = format!("{}\n", "a".repeat(1_023)).repeat(16_383);
    let line = "b".repeat(1_025);
    let dir = scratch();
    fs::write(dir.path().join("lf.txt"), format!("{prefix}{line}\ntail\n")).expect("written");
    fs::write(
        dir.path().join("crlf.txt"),
        format!("{prefix}{line}\r\ntail\r\n"),
    )
    .expect("written");
    let local = Fs::new(dir.path()).expect("the workspace opens");

    let lf = local
        .read(&ReadInput::lines("lf.txt", 16_384, 1))
        .expect("the read answers");
    let crlf = local
        .read(&ReadInput::lines("crlf.txt", 16_384, 1))
        .expect("the read answers");
    assert_eq!(
        lf.lines_counted_to,
        Some(16_384),
        "the bound stopped at line 16384"
    );
    assert_eq!(crlf.lines_counted_to, Some(16_384));
    assert_eq!(lf.text, format!(" 16384\t{line}\n"));
    assert_eq!(crlf.text, lf.text, "the same line, answered the same");
    assert_eq!(
        lf.truncated_lines, [0_u64; 0],
        "line 16384 is whole in the LF file"
    );
    assert_eq!(
        crlf.truncated_lines, lf.truncated_lines,
        "line 16384 is named cut in the CRLF file only"
    );
}

// Pass 2, the same change seen from the other side: `line.rs` says "A `\r` before the newline is
// dropped ... A `\r` a model quotes back to an edit matches nothing in a file it was just handed,
// so the reader does not hand one out." Where the bound falls between a line's `\r` and its `\n`,
// the newline is the next byte, so the line counts as whole, and its `\r` is still in the text.
#[test]
fn a_line_the_counting_bound_stops_between_its_cr_and_its_lf_hands_out_no_cr() {
    // 16383 lines of 1 KiB leave 1025 bytes of budget: 1024 bytes of line 16384 and its `\r`.
    let prefix = format!("{}\n", "a".repeat(1_023)).repeat(16_383);
    let line = "b".repeat(1_024);
    let dir = scratch();
    fs::write(
        dir.path().join("crlf.txt"),
        format!("{prefix}{line}\r\ntail\r\n"),
    )
    .expect("written");
    let local = Fs::new(dir.path()).expect("the workspace opens");
    let read = local
        .read(&ReadInput::lines("crlf.txt", 16_384, 1))
        .expect("the read answers");
    assert_eq!(
        read.lines_counted_to,
        Some(16_384),
        "the bound stopped at line 16384"
    );
    assert!(
        !read.text.contains('\r'),
        "a `\\r` is handed out: {:?}",
        &read.text[read.text.len() - 4..]
    );
}
