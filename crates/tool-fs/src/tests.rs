//! The reader's tests, ported from harness's `local.rs` `mod tests` at `798325f`: those about
//! reads, listings, globs, links and line handling, adapted to the typed API with their
//! assertions kept. The write, edit, search and process tests stayed with their code. After them,
//! the cases for what this crate added: the whole-file digest, `fs.stat`, the refusal names and
//! the workspace constructor.

use std::collections::BTreeSet;
use std::io;
use std::os::unix::fs::symlink;
use std::path::Path;

use serde_json::json;
use tempfile::TempDir;
use tool_contract::conformance::{Outcome, Toolset};

use super::*;

fn workspace() -> TempDir {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/tmp");
    fs::create_dir_all(&base).expect("the scratch directory under target/ is made");
    tempfile::tempdir_in(base).expect("a temporary directory")
}

fn reading(root: &Path) -> Fs {
    Fs::new(root).expect("the workspace opens")
}

#[test]
fn a_link_that_stays_inside_the_workspace_is_still_a_way_to_reach_its_target() {
    // Refusing every link would break a workspace that uses one internally. The boundary is
    // where the link *lands*, so one that lands inside is followed and the read answers the
    // target, under the target's own name.
    let inside = workspace();
    let target = inside.path().join("real.txt");
    fs::write(&target, "before\n").expect("the file is written");
    symlink(&target, inside.path().join("link")).expect("a link");

    let value = reading(inside.path())
        .read(&ReadInput::whole("link"))
        .expect("the read lands");
    assert_eq!(value.path, "real.txt");
    assert_eq!(value.text, "     1\tbefore\n");
}

#[test]
fn a_path_that_climbs_out_or_starts_at_the_root_is_refused() {
    let inside = workspace();
    let local = reading(inside.path());
    for path in ["../escape.txt", "/etc/b10x-escape.txt", "/"] {
        let refusal = local.read(&ReadInput::whole(path)).expect_err("refused");
        assert_eq!(refusal.name(), "fs.OutsideWorkspace", "{path}: {refusal}");
        assert_eq!(refusal.fields(), json!({ "path": path }));
        assert!(
            local
                .stat(&StatInput {
                    path: path.to_owned()
                })
                .is_err(),
            "{path} is as outside for stat as for read"
        );
    }
}

#[test]
fn a_large_file_is_bounded_before_it_is_read_rather_than_after() {
    let inside = workspace();
    fs::write(
        inside.path().join("big.txt"),
        "filler\n".repeat(30 * 1024) + "last\n",
    )
    .expect("the file is written");

    let value = reading(inside.path())
        .read(&ReadInput {
            max_bytes: Some(1024),
            ..ReadInput::whole("big.txt")
        })
        .expect("the read answers");
    assert_eq!(value.bytes, 215_045, "the whole file's size");
    assert!(value.truncated);
    assert_eq!(value.lines.total, Some(30 * 1024 + 1));
    assert_eq!(
        value.lines.to,
        1024 / 7,
        "as many whole lines as the byte ceiling holds, and not one past it"
    );
}

#[test]
fn a_first_line_over_the_requested_byte_window_is_visibly_truncated() {
    let inside = workspace();
    fs::write(inside.path().join("one.txt"), "abcdefghij\n").expect("written");
    let value = reading(inside.path())
        .read(&ReadInput {
            max_bytes: Some(4),
            ..ReadInput::whole("one.txt")
        })
        .expect("the bounded first line is answered");
    assert!(value.truncated);
    assert_eq!(value.truncated_lines, [1]);
    assert_eq!(value.text, "     1\tabc\n");
}

#[test]
fn a_line_too_long_to_answer_whole_is_cut_on_a_character_boundary_and_named() {
    // The cut is at a character count, so a three-byte character cannot be halved by the byte
    // cap that holds the line while it is read. U+FFFD in the reply would read as damage to
    // the file rather than to this answer.
    let inside = workspace();
    fs::write(inside.path().join("wide.txt"), "€".repeat(3_000)).expect("the file is written");

    let value = reading(inside.path())
        .read(&ReadInput::whole("wide.txt"))
        .expect("the read answers");
    assert_eq!(
        value.truncated_lines,
        [1],
        "the cut is named by line number"
    );
    assert!(value.truncated);
    assert_eq!(
        value.text,
        format!("     1\t{}\n", "€".repeat(MAX_READ_LINE_CHARS)),
        "cut at the character count, with no replacement character in it"
    );
}

#[test]
fn a_read_answers_numbered_lines_and_says_which_window_of_the_file_they_are() {
    // The shape a model can quote back to an edit: the number is a prefix it strips, and a
    // window is never mistaken for the file because `lines.total` is there.
    let inside = workspace();
    fs::write(inside.path().join("five.rs"), "a\nb\nc\nd\ne\n").expect("the file is written");
    let local = reading(inside.path());

    let value = local
        .read(&ReadInput::lines("five.rs", 2, 2))
        .expect("the read answers");
    assert_eq!(value.text, "     2\tb\n     3\tc\n");
    assert_eq!(
        value.lines,
        Window {
            from: 2,
            to: 3,
            total: Some(5)
        }
    );
    assert!(value.truncated, "line 5 is not in it");

    let value = local
        .read(&ReadInput::lines("five.rs", 4, 99))
        .expect("the read answers");
    assert_eq!(value.text, "     4\td\n     5\te\n");
    assert_eq!(
        value.lines,
        Window {
            from: 4,
            to: 5,
            total: Some(5)
        }
    );
    assert!(
        !value.truncated,
        "a window that ends at the last line has reached the end"
    );
}

#[test]
fn a_window_that_starts_past_the_end_is_refused_with_the_number_of_lines_there_are() {
    let inside = workspace();
    fs::write(inside.path().join("three.txt"), "a\nb\nc\n").expect("the file is written");
    let local = reading(inside.path());

    let refusal = local
        .read(&ReadInput::lines("three.txt", 9, 10))
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.InvalidWindow");
    let message = refusal.to_string();
    assert!(message.contains("has 3 lines"), "{message}");
    assert!(message.contains("line 9"), "{message}");

    let refusal = local
        .read(&ReadInput {
            offset: Some(0),
            ..ReadInput::whole("three.txt")
        })
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.InvalidWindow");
    assert!(refusal.to_string().contains("numbered from 1"), "{refusal}");

    // An empty file read from its first line is the file, not a window past its end.
    fs::write(inside.path().join("empty.txt"), "").expect("the file is written");
    let value = local
        .read(&ReadInput::whole("empty.txt"))
        .expect("the read answers");
    assert_eq!(value.text, "");
    assert_eq!(
        value.lines,
        Window {
            from: 1,
            to: 0,
            total: Some(0)
        }
    );
    assert!(!value.truncated);
    assert_eq!(
        value.sha256.as_deref(),
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        "the digest of nothing is the well-known one"
    );
}

#[test]
fn a_link_to_a_directory_is_listed_as_neither_a_file_nor_a_directory() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "x").expect("the file is written");
    fs::create_dir(inside.path().join("sub")).expect("the directory is made");
    symlink(inside.path().join("sub"), inside.path().join("zlink")).expect("a link");

    let value = reading(inside.path())
        .list(&ListInput::default())
        .expect("the listing answers");
    let kinds: Vec<Kind> = value.entries.iter().map(|entry| entry.kind).collect();
    assert_eq!(kinds, [Kind::File, Kind::Directory, Kind::Symlink]);
    assert_eq!(value.entries[0].bytes, Some(1));
    assert_eq!(value.entries[2].bytes, None);
    assert_eq!(value.path, "", "the root is the empty relative path");
    assert!(!value.truncated);
}

#[test]
fn glob_answers_workspace_relative_paths_and_never_the_machines_own_output() {
    let inside = workspace();
    fs::create_dir_all(inside.path().join("crates/x/src")).expect("the directories are made");
    fs::create_dir_all(inside.path().join("target/debug")).expect("the directories are made");
    fs::write(inside.path().join("crates/x/src/lib.rs"), "x").expect("the file is written");
    fs::write(inside.path().join("top.rs"), "x").expect("the file is written");
    fs::write(inside.path().join("target/debug/built.rs"), "x").expect("the file is written");
    let local = reading(inside.path());

    // No separator in the glob, so it is the file's own name, anywhere in the tree: the rule a
    // person means by `*.rs`.
    let value = local
        .glob(&GlobInput::new("*.rs"))
        .expect("the glob answers");
    assert_eq!(
        value.paths,
        ["crates/x/src/lib.rs", "top.rs"],
        "and `target` is machine output, never an answer"
    );
    assert!(!value.truncated);

    // A separator makes it the whole path, and `*` does not cross one.
    let value = local
        .glob(&GlobInput::new("crates/**/*.rs"))
        .expect("the glob answers");
    assert_eq!(value.paths, ["crates/x/src/lib.rs"]);

    let value = local
        .glob(&GlobInput {
            max_results: Some(1),
            ..GlobInput::new("*.rs")
        })
        .expect("the glob answers");
    assert_eq!(value.paths.len(), 1);
    assert!(value.truncated, "a cut list says it was cut");

    let refusal = local
        .glob(&GlobInput::new("crates/["))
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.InvalidGlob");
    assert!(refusal.to_string().contains("not a glob"), "{refusal}");
    assert_eq!(refusal.fields(), json!({ "glob": "crates/[" }));
}

#[test]
fn a_walk_stopped_by_the_depth_bound_says_so_and_one_that_reached_the_bottom_says_it_did_not() {
    // `fs.glob` says it lists every match under the workspace. It stops descending at
    // `MAX_WALK_DEPTH`, and the flag is the only thing that says so.
    let deep = workspace();
    let mut path = deep.path().to_path_buf();
    for level in 0..14 {
        path = path.join(format!("d{level}"));
    }
    fs::create_dir_all(&path).expect("the directories are made");
    fs::write(path.join("buried.txt"), "needle\n").expect("the file is written");
    let local = reading(deep.path());

    let value = local
        .glob(&GlobInput::new("*.txt"))
        .expect("the glob answers");
    assert!(value.depth_bound_reached);
    assert!(
        value.paths.is_empty(),
        "the file is past the bound, and the flag is the only thing that says so"
    );

    let shallow = workspace();
    fs::create_dir_all(shallow.path().join("a/b")).expect("the directories are made");
    fs::write(shallow.path().join("a/b/near.txt"), "needle\n").expect("the file is written");
    let local = reading(shallow.path());

    let value = local
        .glob(&GlobInput::new("*.txt"))
        .expect("the glob answers");
    assert!(!value.depth_bound_reached);
    assert_eq!(value.paths, ["a/b/near.txt"]);
}

#[test]
fn counting_lines_stops_at_a_bound_and_the_reply_says_where_rather_than_answering_a_part_count() {
    // Reading to the end to count lines is a full scan of a multi-gigabyte artefact on every
    // read. Past the bound the count is not known, so it is `None`, never the part of it this
    // read happened to reach, which would be read as the whole file's. The digest goes with it.
    let inside = workspace();
    let line = format!("{}\n", "x".repeat(1_023));
    fs::write(inside.path().join("huge.txt"), line.repeat(17 * 1_024))
        .expect("the file is written");
    let local = reading(inside.path());

    let value = local
        .read(&ReadInput::lines("huge.txt", 1, 2))
        .expect("the read answers");
    assert_eq!(value.lines.total, None, "absence, not a count");
    assert_eq!(
        value.lines_counted_to,
        Some(MAX_LINE_COUNT_BYTES / 1_024 + 1),
        "the last line the scan reached: the one whose end took it past the bound"
    );
    assert!(value.truncated);
    assert_eq!(
        value.bytes,
        17 * 1_024 * 1_024,
        "the size is metadata's and is known whatever the scan did"
    );
    assert!(
        value.note.as_deref().expect("a note").contains("not known"),
        "{:?}",
        value.note
    );
    assert_eq!(
        value.sha256, None,
        "a digest of the part that was scanned would read as the whole file's"
    );

    // And a window past where the scan reached is refused by that, not by a line count nobody
    // finished.
    let refusal = local
        .read(&ReadInput::lines("huge.txt", 20_000, 5))
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.InvalidWindow");
    let message = refusal.to_string();
    assert!(message.contains("stops counting lines"), "{message}");
    assert!(message.contains("line 20000"), "{message}");
}

#[test]
fn a_link_out_of_the_workspace_is_not_a_way_for_glob_to_read_what_is_outside() {
    // The containment re-check inside `walk` is the boundary a glob stands on: `is_dir` and
    // `read_dir` both follow links, so a link to a directory outside would be walked and its
    // files answered under workspace-relative names.
    let outside = workspace();
    fs::create_dir(outside.path().join("private")).expect("the directory is made");
    fs::write(outside.path().join("private/secret.txt"), "needle\n").expect("the file is written");
    fs::write(outside.path().join("loose.txt"), "needle\n").expect("the file is written");

    let inside = workspace();
    fs::write(inside.path().join("own.txt"), "needle\n").expect("the file is written");
    symlink(outside.path(), inside.path().join("out")).expect("a link to a directory");
    symlink(
        outside.path().join("loose.txt"),
        inside.path().join("out.txt"),
    )
    .expect("a link to a file");
    let local = reading(inside.path());

    let value = local
        .glob(&GlobInput::new("*.txt"))
        .expect("the glob answers");
    assert_eq!(
        value.paths,
        ["own.txt"],
        "neither the linked directory's files nor the linked file itself"
    );

    let refusal = local
        .read(&ReadInput::whole("out.txt"))
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.OutsideWorkspace");
    let refusal = local
        .list(&ListInput {
            path: Some("out".to_owned()),
        })
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.OutsideWorkspace");
}

#[test]
fn a_crlf_file_reads_with_the_carriage_returns_dropped() {
    // Keeping the `\r` made a `\r` quoted back to an edit match nothing in a file the model had
    // just been handed.
    let inside = workspace();
    fs::write(inside.path().join("dos.txt"), "alpha\r\nbeta\r\n").expect("the file is written");

    let value = reading(inside.path())
        .read(&ReadInput::whole("dos.txt"))
        .expect("the read answers");
    assert_eq!(value.text, "     1\talpha\n     2\tbeta\n");
    assert!(
        value.truncated_lines.is_empty(),
        "a dropped `\\r` is not a cut line"
    );
    assert!(!value.truncated);
    assert_eq!(
        value.lines,
        Window {
            from: 1,
            to: 2,
            total: Some(2)
        }
    );
}

#[test]
fn an_empty_glob_is_refused_by_name() {
    // An empty glob matched nothing and answered an empty list, which reads as *there are no
    // such files* rather than as *you named no pattern*.
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "x\n").expect("the file is written");
    let refusal = reading(inside.path())
        .glob(&GlobInput::new(""))
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.InvalidGlob");
    assert!(
        refusal.to_string().contains("`glob` is required"),
        "{refusal}"
    );
    assert_eq!(refusal.fields(), json!({ "glob": "" }));
}

#[test]
fn a_read_carries_the_digest_of_the_whole_file_and_not_of_the_window() {
    let inside = workspace();
    let body = "one\ntwo\nthree\n";
    fs::write(inside.path().join("a.txt"), body).expect("the file is written");
    let local = reading(inside.path());

    let whole = local
        .read(&ReadInput::whole("a.txt"))
        .expect("the read answers");
    let window = local
        .read(&ReadInput::lines("a.txt", 2, 1))
        .expect("the read answers");
    let expected = hex(&Sha256::digest(body.as_bytes()));
    assert_eq!(whole.sha256.as_deref(), Some(expected.as_str()));
    assert_eq!(
        window.sha256.as_deref(),
        Some(expected.as_str()),
        "the window's digest is the file's"
    );
    assert_ne!(
        expected,
        hex(&Sha256::digest(window.text.as_bytes())),
        "and not the digest of the text it answered"
    );
    assert_eq!(window.text, "     2\ttwo\n");
}

#[test]
fn a_byte_window_over_the_ceiling_is_capped_at_the_ceiling() {
    let inside = workspace();
    fs::write(inside.path().join("big.txt"), "filler\n".repeat(40_000))
        .expect("the file is written");

    let value = reading(inside.path())
        .read(&ReadInput {
            max_bytes: Some(300_000),
            ..ReadInput::whole("big.txt")
        })
        .expect("the read answers");
    assert_eq!(value.bytes, 280_000);
    assert!(value.truncated);
    assert_eq!(
        value.lines.to,
        MAX_READ_BYTES_CEILING / 7,
        "the ceiling, not the ask, closes the window"
    );
    assert_eq!(value.lines.total, Some(40_000));
}

#[test]
fn a_path_that_does_not_exist_is_not_found_and_a_dangling_link_is_too() {
    let inside = workspace();
    symlink(
        inside.path().join("nowhere.txt"),
        inside.path().join("dangling"),
    )
    .expect("a link");
    let local = reading(inside.path());

    for path in ["missing.txt", "dangling", "missing/../a.txt"] {
        let refusal = local.read(&ReadInput::whole(path)).expect_err("refused");
        assert_eq!(refusal.name(), "fs.NotFound", "{path}: {refusal}");
        assert_eq!(refusal.fields(), json!({ "path": path }));
    }
    let refusal = local
        .list(&ListInput {
            path: Some("missing".to_owned()),
        })
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.NotFound");
    let refusal = local
        .glob(&GlobInput {
            path: Some("missing".to_owned()),
            ..GlobInput::new("*")
        })
        .expect_err("refused");
    assert_eq!(refusal.name(), "fs.NotFound");
}

#[test]
fn stat_answers_an_entrys_own_kind() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "one\ntwo\n").expect("the file is written");
    fs::create_dir(inside.path().join("sub")).expect("the directory is made");
    symlink(inside.path().join("a.txt"), inside.path().join("link")).expect("a link");
    let local = reading(inside.path());

    let file = local
        .stat(&StatInput {
            path: "./a.txt".to_owned(),
        })
        .expect("stat answers");
    assert_eq!(
        file,
        StatOutput {
            path: "a.txt".to_owned(),
            kind: Kind::File,
            bytes: Some(8),
            sha256: Some(hex(&Sha256::digest(b"one\ntwo\n"))),
        }
    );

    let directory = local
        .stat(&StatInput {
            path: "sub".to_owned(),
        })
        .expect("stat answers");
    assert_eq!(
        directory,
        StatOutput {
            path: "sub".to_owned(),
            kind: Kind::Directory,
            bytes: None,
            sha256: None,
        }
    );

    let link = local
        .stat(&StatInput {
            path: "link".to_owned(),
        })
        .expect("stat answers");
    assert_eq!(
        link,
        StatOutput {
            path: "link".to_owned(),
            kind: Kind::Symlink,
            bytes: None,
            sha256: None,
        },
        "a link is a link under its own name, whatever it points at"
    );

    let root = local
        .stat(&StatInput {
            path: ".".to_owned(),
        })
        .expect("stat answers");
    assert_eq!(root.kind, Kind::Directory);
    assert_eq!(root.path, "");
}

#[test]
fn stat_digests_a_file_of_at_most_the_counting_bound_and_not_one_over_it() {
    // "Over 16 MiB" is more than 16,777,216 bytes: a file of exactly that many is counted and
    // digested whole by a read, so stat digests it too; one byte more and neither does.
    let inside = workspace();
    let line = format!("{}\n", "x".repeat(1_023));
    fs::write(inside.path().join("at.txt"), line.repeat(16 * 1_024)).expect("the file is written");
    fs::write(
        inside.path().join("over.txt"),
        line.repeat(16 * 1_024) + "x",
    )
    .expect("the file is written");
    let local = reading(inside.path());

    let at = local
        .stat(&StatInput {
            path: "at.txt".to_owned(),
        })
        .expect("stat answers");
    assert_eq!(at.kind, Kind::File);
    assert_eq!(at.bytes, Some(MAX_LINE_COUNT_BYTES));
    assert!(
        at.sha256.is_some(),
        "a file of exactly the bound is digested"
    );
    let read = local
        .read(&ReadInput::lines("at.txt", 1, 1))
        .expect("the read answers");
    assert_eq!(read.lines.total, Some(16 * 1_024), "and counted whole");
    assert_eq!(read.sha256, at.sha256, "read and stat digest the same file");

    let over = local
        .stat(&StatInput {
            path: "over.txt".to_owned(),
        })
        .expect("stat answers");
    assert_eq!(over.bytes, Some(MAX_LINE_COUNT_BYTES + 1));
    assert_eq!(over.sha256, None, "one byte over the bound is over 16 MiB");
    let read = local
        .read(&ReadInput::lines("over.txt", 1, 1))
        .expect("the read answers");
    assert_eq!(read.lines.total, None);
    assert_eq!(read.sha256, None);
}

#[test]
fn the_workspace_must_be_a_directory() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "x").expect("the file is written");

    let refusal = Fs::new(inside.path().join("a.txt")).expect_err("refused");
    assert_eq!(refusal.name(), "fs.NotADirectory");
    assert!(refusal.to_string().starts_with("workspace `"), "{refusal}");

    let refusal = Fs::new(inside.path().join("missing")).expect_err("refused");
    assert_eq!(refusal.name(), "fs.Io");
    assert!(refusal.to_string().starts_with("workspace `"), "{refusal}");

    let opened = reading(inside.path());
    assert_eq!(
        opened.root(),
        inside.path().canonicalize().expect("canonical")
    );
}

#[test]
fn every_refusal_is_named_by_the_contract_and_carries_its_declared_fields() {
    // The class, checked rather than listed in prose: a refusal whose name the contract does not
    // declare, or whose fields differ from the declaration, would fail every scenario that hit
    // it. Every variant is constructed here; a new one has to be added to this list.
    let gone = || io::Error::new(io::ErrorKind::NotFound, "gone");
    let broken = globset::Glob::new("[").expect_err("a broken glob");
    let every = [
        FsError::Workspace {
            path: "w".to_owned(),
            source: gone(),
        },
        FsError::WorkspaceNotADirectory {
            path: "w".to_owned(),
        },
        FsError::NotFound {
            path: "p".to_owned(),
            source: gone(),
        },
        FsError::NotAFile {
            path: "p".to_owned(),
        },
        FsError::NotADirectory {
            path: "p".to_owned(),
        },
        FsError::OutsideWorkspace {
            path: "p".to_owned(),
        },
        FsError::OffsetZero {
            path: "p".to_owned(),
        },
        FsError::LimitZero {
            path: "p".to_owned(),
        },
        FsError::PastEnd {
            path: "p".to_owned(),
            offset: 9,
            total: 3,
        },
        FsError::PastScan {
            path: "p".to_owned(),
            offset: 9,
            total: 3,
        },
        FsError::EmptyGlob,
        FsError::InvalidGlob {
            glob: "[".to_owned(),
            source: broken,
        },
        FsError::Io {
            path: "p".to_owned(),
            source: gone(),
        },
    ];
    for error in every {
        let declared = contract()
            .error_decl(error.name())
            .unwrap_or_else(|| panic!("{} is not declared by the contract", error.name()));
        let fields = error.fields();
        let returned: BTreeSet<String> = fields
            .as_object()
            .expect("fields are an object")
            .keys()
            .cloned()
            .collect();
        let expected: BTreeSet<String> = declared.fields.iter().map(|f| f.name.clone()).collect();
        assert_eq!(returned, expected, "{}", error.name());
    }
}

#[test]
fn a_mutant_is_only_what_it_was_asked_to_be() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "one\ntwo\nthree\n").expect("the file is written");

    let intact = reading(inside.path());
    assert!(!intact.mutated_by(Mutation::NoReadCap));

    let page = Fs::mutated(inside.path(), Mutation::PageDigest).expect("opens");
    assert!(page.mutated_by(Mutation::PageDigest));
    assert!(!page.mutated_by(Mutation::NoContainment));
    let value = page
        .read(&ReadInput::lines("a.txt", 2, 1))
        .expect("the read answers");
    assert_eq!(
        value.sha256.as_deref(),
        Some(hex(&Sha256::digest(value.text.as_bytes())).as_str()),
        "the mutant digests the page"
    );
    assert!(value.truncated, "and nothing else about it changed");
}

#[test]
fn an_input_the_operation_does_not_declare_is_invalid_input_through_the_toolset() {
    // semantics.md: an unknown field is `fs.InvalidInput`. The suite format cannot hold this
    // (the runner refuses an unknown `call.input` key at coverage), so the crate holds it here,
    // beside the missing field and the wrong type the suite does hold.
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "x\n").expect("the file is written");
    let local = reading(inside.path());
    for (operation, input) in [
        ("fs.read", json!({ "path": "a.txt", "colour": "red" })),
        ("fs.list", json!({ "path": ".", "depth": 1 })),
        ("fs.glob", json!({ "glob": "*", "recursive": true })),
        ("fs.stat", json!({ "path": "a.txt", "follow": false })),
        ("fs.read", json!({})),
        ("fs.read", json!({ "path": "a.txt", "offset": "two" })),
    ] {
        match local.invoke(operation, &input) {
            Outcome::Error { name, fields, .. } => {
                assert_eq!(name, "fs.InvalidInput", "{operation} {input}");
                assert_eq!(fields, json!({}), "{operation} {input}");
            }
            Outcome::Ok(output) => panic!("{operation} {input} answered {output}"),
        }
    }
    assert!(
        matches!(
            local.invoke("fs.read", &json!({ "path": "a.txt" })),
            Outcome::Ok(_)
        ),
        "the same input without the unknown field is answered"
    );
}

#[test]
fn limit_zero_names_no_line_and_is_refused() {
    // contract.yaml, fs.Window `to`: one below `from` is only an empty file read from line 1. A
    // `limit` of 0 would answer that shape on any file, so it is refused instead.
    let inside = workspace();
    fs::write(inside.path().join("three.txt"), "a\nb\nc\n").expect("the file is written");
    let local = reading(inside.path());
    for offset in [1, 2, 3] {
        let refusal = local
            .read(&ReadInput::lines("three.txt", offset, 0))
            .expect_err("refused");
        assert_eq!(refusal.name(), "fs.InvalidWindow");
        assert!(refusal.to_string().contains("`limit`"), "{refusal}");
        assert_eq!(refusal.fields(), json!({ "path": "three.txt" }));
    }
}

#[test]
fn a_glob_with_exactly_max_results_matches_is_a_complete_answer() {
    // harness 0.13.3 tested the cap on every visited file, so one match at `max_results: 1`
    // followed by a file the glob did not name answered `truncated: true`.
    let inside = workspace();
    fs::write(inside.path().join("a.rs"), "x").expect("the file is written");
    fs::write(inside.path().join("b.txt"), "x").expect("the file is written");
    fs::write(inside.path().join("c.rs"), "x").expect("the file is written");
    let local = reading(inside.path());

    let value = local
        .glob(&GlobInput {
            max_results: Some(2),
            ..GlobInput::new("*.rs")
        })
        .expect("the glob answers");
    assert_eq!(value.paths, ["a.rs", "c.rs"]);
    assert!(
        !value.truncated,
        "two matches at a cap of two is the whole answer"
    );

    let value = local
        .glob(&GlobInput {
            max_results: Some(1),
            ..GlobInput::new("*.rs")
        })
        .expect("the glob answers");
    assert_eq!(value.paths, ["a.rs"]);
    assert!(
        value.truncated,
        "a second match past the cap is what truncated names"
    );
}

#[test]
fn a_capped_glob_stops_walking_once_the_match_past_the_cap_is_found() {
    // The visitor's stop returned from the current directory only, so every parent kept
    // iterating and canonicalised the rest of the tree after the answer was already complete.
    let inside = workspace();
    for outer in 0..10 {
        for inner in 0..10 {
            let directory = inside.path().join(format!("d{outer}/s{inner}"));
            fs::create_dir_all(&directory).expect("the directories are made");
            for file in 0..10 {
                fs::write(directory.join(format!("f{file}.txt")), "x")
                    .expect("the file is written");
            }
        }
    }
    let local = reading(inside.path());

    walk::VISITED.with(|visited| visited.set(0));
    let value = local
        .glob(&GlobInput {
            max_results: Some(1),
            ..GlobInput::new("*.txt")
        })
        .expect("the glob answers");
    let visited = walk::VISITED.with(std::cell::Cell::get);
    println!("the walk visited {visited} of 1,110 entries");

    assert_eq!(value.paths, ["d0/s0/f0.txt"]);
    assert!(value.truncated, "a second match past the cap");
    assert!(!value.depth_bound_reached);
    assert!(
        visited < 10,
        "the walk visited {visited} of 1,110 entries for one answered path"
    );
}

#[test]
fn a_capped_glob_reports_the_depth_bound_only_where_it_bit_before_the_cap() {
    // Once the cap is passed nothing more is walked, so a directory past the bound that comes
    // after the stop is never reached and is not reported; one that came before it still is.
    fn nested(under: &Path) -> std::path::PathBuf {
        (0..14).fold(under.to_path_buf(), |path, level| {
            path.join(format!("d{level}"))
        })
    }

    let after = workspace();
    fs::create_dir_all(after.path().join("a")).expect("the directory is made");
    fs::write(after.path().join("a/1.txt"), "x").expect("the file is written");
    fs::write(after.path().join("a/2.txt"), "x").expect("the file is written");
    let deep = nested(&after.path().join("z"));
    fs::create_dir_all(&deep).expect("the directories are made");
    fs::write(deep.join("deep.txt"), "x").expect("the file is written");

    let value = reading(after.path())
        .glob(&GlobInput {
            max_results: Some(1),
            ..GlobInput::new("*.txt")
        })
        .expect("the glob answers");
    assert_eq!(value.paths, ["a/1.txt"]);
    assert!(value.truncated);
    assert!(
        !value.depth_bound_reached,
        "the deep directory comes after the stop and is never walked"
    );

    let before = workspace();
    let deep = nested(&before.path().join("a"));
    fs::create_dir_all(&deep).expect("the directories are made");
    fs::write(deep.join("deep.txt"), "x").expect("the file is written");
    fs::create_dir_all(before.path().join("b")).expect("the directory is made");
    fs::write(before.path().join("b/1.txt"), "x").expect("the file is written");
    fs::write(before.path().join("b/2.txt"), "x").expect("the file is written");

    let value = reading(before.path())
        .glob(&GlobInput {
            max_results: Some(1),
            ..GlobInput::new("*.txt")
        })
        .expect("the glob answers");
    assert_eq!(value.paths, ["b/1.txt"]);
    assert!(value.truncated);
    assert!(
        value.depth_bound_reached,
        "the bound bit before the stop, so the answer above it is a partial one"
    );
}

#[test]
fn a_bad_window_is_judged_after_the_path() {
    // The refusal a governor reads is the path's: a window nobody could answer on a path outside
    // the workspace, absent, or not a file is refused for the path, not for the window.
    let inside = workspace();
    fs::create_dir(inside.path().join("sub")).expect("the directory is made");
    let local = reading(inside.path());
    for (path, expected) in [
        ("../x", "fs.OutsideWorkspace"),
        ("missing.txt", "fs.NotFound"),
        ("sub", "fs.NotAFile"),
    ] {
        let refusal = local
            .read(&ReadInput::lines(path, 1, 0))
            .expect_err("refused");
        assert_eq!(refusal.name(), expected, "{path} with limit 0: {refusal}");
        let refusal = local
            .read(&ReadInput {
                offset: Some(0),
                ..ReadInput::whole(path)
            })
            .expect_err("refused");
        assert_eq!(refusal.name(), expected, "{path} with offset 0: {refusal}");
    }
}

#[test]
fn the_first_line_cut_leaves_room_for_a_separator_only_where_the_line_has_one() {
    let inside = workspace();
    fs::write(inside.path().join("bare.txt"), "abcdefghij").expect("the file is written");
    fs::write(inside.path().join("ended.txt"), "abcdefghij\n").expect("the file is written");
    let local = reading(inside.path());

    let bare = local
        .read(&ReadInput {
            max_bytes: Some(4),
            ..ReadInput::whole("bare.txt")
        })
        .expect("the read answers");
    assert_eq!(
        bare.text, "     1\tabcd\n",
        "no separator, so all four bytes are the line's"
    );
    assert!(bare.truncated);
    assert_eq!(bare.truncated_lines, [1]);

    let ended = local
        .read(&ReadInput {
            max_bytes: Some(4),
            ..ReadInput::whole("ended.txt")
        })
        .expect("the read answers");
    assert_eq!(
        ended.text, "     1\tabc\n",
        "one byte of the four is the separator"
    );
    assert_eq!(ended.truncated_lines, [1]);
}

#[test]
fn a_single_line_over_the_counting_bound_stops_the_scan_inside_the_window() {
    // The bound is on bytes read, not on lines seen: a file over 16 MiB that holds no newline is
    // one line, and it is over the bound for a read exactly as it is for stat.
    let inside = workspace();
    let body = vec![b'y'; usize::try_from(MAX_LINE_COUNT_BYTES + 1).expect("fits")];
    fs::write(inside.path().join("blob.bin"), &body).expect("the file is written");
    let local = reading(inside.path());

    let read = local
        .read(&ReadInput::whole("blob.bin"))
        .expect("the read answers");
    assert_eq!(read.bytes, MAX_LINE_COUNT_BYTES + 1);
    assert_eq!(read.lines.total, None);
    assert_eq!(read.sha256, None);
    assert_eq!(read.lines_counted_to, Some(1));
    assert!(read.truncated);
    assert_eq!(
        read.truncated_lines,
        [1],
        "the one line is over the ceiling too"
    );
    assert!(
        read.text.starts_with("     1\tyyyy"),
        "{:?}",
        &read.text[..16]
    );
    let stat = local
        .stat(&StatInput {
            path: "blob.bin".to_owned(),
        })
        .expect("stat answers");
    assert_eq!(stat.sha256, None, "stat agrees");
}
