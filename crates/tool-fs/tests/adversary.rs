//! The adversary's cases for the `fs` v1alpha1 unit, each written against one sentence of
//! `contracts/fs/v1alpha1/contract.yaml` or `semantics.md` before anything was run.
//!
//! The scenario cases are `contracts/fs/v1alpha1/scenarios/adversary-*.yaml`; a test here picks
//! one out of the merged suite's report so that one case can be run and read alone. The direct
//! cases need a file too large for a fixture or an input the suite format does not express.

use std::fs;
use std::path::{Path, PathBuf};

use tool_contract::conformance::{Report, Suite, run};
use tool_fs::{Fs, ReadInput, StatInput, contract};

/// The counting bound as the contract states it: 16 MiB.
const COUNTING_BOUND: u64 = 16 * 1024 * 1024;

fn contract_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/fs/v1alpha1")
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

fn scenario_passes(report: &Report, name: &str) {
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

fn workspace_with(name: &str, bytes: &[u8]) -> (tempfile::TempDir, Fs) {
    let dir = scratch();
    fs::write(dir.path().join(name), bytes).expect("the file is written");
    let local = Fs::new(dir.path()).expect("the workspace opens");
    (dir, local)
}

// contract.yaml, fs.glob output `truncated`: "More paths matched than were answered."
#[test]
fn a_glob_with_exactly_max_results_matches_is_not_truncated() {
    scenario_passes(
        &suite_report(),
        "adversary-a-glob-with-exactly-max-results-matches-is-not-truncated",
    );
}

// semantics.md, Paths and the boundary: "A path that lands inside the root and does not exist is
// `fs.NotFound`".
#[test]
fn a_path_through_a_file_does_not_exist_and_is_not_found() {
    scenario_passes(
        &suite_report(),
        "adversary-a-path-through-a-file-does-not-exist-and-is-not-found",
    );
}

// Every other adversary scenario pins a sentence or a constant no earlier scenario held: the depth
// bound at exactly 12, all five skipped directories, the 2000-character cut, the byte ceiling at
// exactly its value, the last line as a window and the line after it, a missing input field, a
// link that leads nowhere, a link to a directory inside the workspace.
#[test]
fn the_other_adversary_scenarios_hold() {
    let report = suite_report();
    assert!(
        report.refused.is_empty(),
        "the suite was refused:\n{report}"
    );
    let red = [
        "adversary-a-glob-with-exactly-max-results-matches-is-not-truncated",
        "adversary-a-path-through-a-file-does-not-exist-and-is-not-found",
    ];
    let others: Vec<_> = report
        .scenarios
        .iter()
        .filter(|s| s.name.starts_with("adversary-") && !red.contains(&s.name.as_str()))
        .collect();
    assert_eq!(
        others.len(),
        21,
        "the adversary scenarios all ran: {report}"
    );
    for result in others {
        assert!(result.passed, "FAIL {}: {}", result.name, result.detail);
    }
}

// contract.yaml, fs.read guidance: "On a file over 16 MiB the line count stops: lines.total and
// sha256 are null"; fs.stat guidance: "sha256 only for a file under 16 MiB". A file one byte
// under 16 MiB is under the bound for both, and the scan consumed every byte of it.
#[test]
fn a_file_one_byte_under_the_counting_bound_is_counted_and_digested_whole() {
    // 16383 lines of 1024 bytes, then 1023 bytes with no newline after them: 16 MiB - 1.
    let mut body = format!("{}\n", "x".repeat(1_023))
        .repeat(16 * 1_024 - 1)
        .into_bytes();
    body.extend(std::iter::repeat_n(b'x', 1_023));
    assert_eq!(
        u64::try_from(body.len()).expect("the length fits"),
        COUNTING_BOUND - 1
    );
    let (_dir, local) = workspace_with("under.txt", &body);

    let described = local
        .stat(&StatInput {
            path: "under.txt".to_owned(),
        })
        .expect("stat answers");
    assert!(
        described.sha256.is_some(),
        "stat digests a file under 16 MiB"
    );
    let read = local
        .read(&ReadInput::lines("under.txt", 1, 1))
        .expect("the read answers");
    assert_eq!(read.bytes, COUNTING_BOUND - 1);
    assert_eq!(
        read.lines.total,
        Some(16 * 1_024),
        "a file under 16 MiB has a known line count; lines_counted_to {:?}, note {:?}",
        read.lines_counted_to,
        read.note
    );
    assert_eq!(
        read.sha256, described.sha256,
        "read and stat digest the same file"
    );
}

// contract.yaml, fs.Window `to`: "one below from when nothing was, which is only an empty file
// read from line 1".
#[test]
fn a_window_one_below_its_start_is_only_an_empty_file_read_from_line_one() {
    let (_dir, local) = workspace_with("three.txt", b"a\nb\nc\n");
    for offset in [1_u64, 2, 9] {
        match local.read(&ReadInput::lines("three.txt", offset, 0)) {
            // Refusing `limit: 0` is within the contract, and since the second pass it is the
            // answer: `fs.InvalidWindow` naming `limit`, judged before an offset past the end is.
            Err(refusal) => {
                assert_eq!(
                    refusal.name(),
                    "fs.InvalidWindow",
                    "offset {offset}: {refusal}"
                );
                assert!(
                    refusal.to_string().contains("`limit`"),
                    "offset {offset}: {refusal}"
                );
            }
            // An answer has to be within the contract too.
            Ok(out) => {
                if out.lines.to + 1 == out.lines.from {
                    assert_eq!(
                        (out.lines.from, out.lines.total),
                        (1, Some(0)),
                        "limit 0 at offset {offset} answered lines {:?} with text {:?}",
                        out.lines,
                        out.text
                    );
                }
            }
        }
    }
}

// contract.yaml, fs.read guidance: "On a file over 16 MiB (more than 16777216 bytes) the line
// count stops: lines.total and sha256 are null"; fs.stat guidance: "sha256 only for a file of at
// most 16 MiB". The counting bound is tested between lines, so a file over it that holds no
// newline is one line and is scanned whole inside the window; the two sentences then disagree on
// the same file.
#[test]
fn a_file_over_the_counting_bound_with_no_newline_is_over_it_for_read_as_well_as_for_stat() {
    let body = vec![b'x'; usize::try_from(COUNTING_BOUND + 1).expect("the length fits")];
    let (_dir, local) = workspace_with("blob.bin", &body);

    let described = local
        .stat(&StatInput {
            path: "blob.bin".to_owned(),
        })
        .expect("stat answers");
    let read = local
        .read(&ReadInput::whole("blob.bin"))
        .expect("the read answers");
    assert_eq!(read.bytes, COUNTING_BOUND + 1);
    assert_eq!(
        (
            read.lines.total,
            read.sha256.is_some(),
            described.sha256.is_some()
        ),
        (None, false, false),
        "over 16 MiB: (lines.total, read digests, stat digests); lines_counted_to {:?}, \
         truncated_lines {:?}",
        read.lines_counted_to,
        read.truncated_lines
    );
}
