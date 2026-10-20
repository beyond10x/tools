//! harness's `local.rs` tests for write, edit, links and new paths, carried here unchanged in
//! substance; renamed where the API moved (`file_write` became `write` over a [`WriteInput`],
//! `file_edit` became `edit` over an [`EditInput`], `lands` became [`Editor::lands`]) and asserting
//! the contract error name where harness asserted a word of the message.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use tempfile::{TempDir, tempdir};
use tool_contract::conformance::{Outcome, Toolset};

use super::*;

fn workspace() -> TempDir {
    tempdir().expect("a temporary directory")
}

fn writing(root: &Path) -> Editor {
    Editor::new(root).expect("the workspace opens")
}

fn write_input(path: &str, text: &str) -> WriteInput {
    WriteInput {
        path: path.to_owned(),
        text: text.to_owned(),
        expected_sha256: None,
    }
}

fn edit_input(path: &str, old: &str, new: &str) -> EditInput {
    EditInput {
        path: path.to_owned(),
        old: old.to_owned(),
        new: new.to_owned(),
        expected_sha256: None,
    }
}

// --- from harness local.rs `mod tests` ------------------------------------------------------------

#[test]
fn a_link_that_leads_nowhere_is_not_a_door_out_of_the_workspace() {
    // `Path::exists` follows links, so a dangling one used to look absent: the walk climbed to
    // the workspace root, containment passed, and the write created the file outside.
    let outside = workspace();
    let inside = workspace();
    let escaped = outside.path().join("escaped.txt");
    symlink(&escaped, inside.path().join("link")).expect("a link");

    let refusal = writing(inside.path())
        .write(&write_input("link", "owned"))
        .expect_err("refused");
    assert_eq!(refusal.name(), "code.IsSymlink", "{refusal}");
    assert!(refusal.to_string().contains("link"), "{refusal}");
    assert!(!escaped.exists(), "the write followed the link out");
}

#[test]
fn a_link_to_a_file_outside_the_workspace_does_not_get_to_overwrite_it() {
    let outside = workspace();
    let inside = workspace();
    let victim = outside.path().join("victim.txt");
    fs::write(&victim, "original").expect("the file is written");
    symlink(&victim, inside.path().join("link")).expect("a link");

    let refusal = writing(inside.path())
        .write(&write_input("link", "owned"))
        .expect_err("refused");
    assert_eq!(refusal.name(), "code.OutsideWorkspace", "{refusal}");
    assert_eq!(
        fs::read_to_string(&victim).expect("the file is read"),
        "original"
    );
}

#[test]
fn a_link_that_stays_inside_the_workspace_is_still_a_way_to_reach_its_target() {
    // Refusing every link would break a workspace that uses one internally. The boundary is
    // where the link *lands*, so one that lands inside is followed and the write goes to the
    // target, under the target's own name.
    let inside = workspace();
    let target = inside.path().join("real.txt");
    fs::write(&target, "before").expect("the file is written");
    symlink(&target, inside.path().join("link")).expect("a link");

    let value = writing(inside.path())
        .write(&write_input("link", "after"))
        .expect("the write lands");
    assert_eq!(value.path, "real.txt");
    assert_eq!(
        fs::read_to_string(&target).expect("the file is read"),
        "after"
    );
}

#[test]
fn where_a_path_lands_is_the_targets_own_name_and_a_new_path_is_its_own() {
    // What the scope is given to judge: the file the bytes reach, under the name it has.
    let inside = workspace();
    fs::create_dir(inside.path().join("target")).expect("the directory is made");
    fs::write(inside.path().join("target/x"), "built").expect("the file is written");
    symlink(inside.path().join("target/x"), inside.path().join("link")).expect("a link");
    let editor = writing(inside.path());

    assert_eq!(editor.lands("link").expect("lands"), "target/x");
    assert_eq!(editor.lands("./target/x").expect("lands"), "target/x");
    assert_eq!(
        editor.lands("new/dir/file.txt").expect("lands"),
        "new/dir/file.txt"
    );
    assert_eq!(
        editor.lands("../escape.txt").expect_err("refused").name(),
        "code.OutsideWorkspace",
        "outside is the write's refusal, and this answers the same"
    );
}

#[test]
fn a_new_file_under_directories_that_do_not_exist_yet_is_refused_without_side_effects() {
    let inside = workspace();
    let refusal = writing(inside.path())
        .write(&write_input("new/dir/file.txt", "hello"))
        .expect_err("a missing parent is refused");
    assert_eq!(refusal.name(), "code.ParentMissing", "{refusal}");
    assert!(
        refusal
            .to_string()
            .contains("parent directory that does not exist"),
        "{refusal}"
    );
    assert!(!inside.path().join("new").exists());
}

#[test]
fn a_path_that_climbs_out_or_starts_at_the_root_is_refused() {
    let inside = workspace();
    let editor = writing(inside.path());
    assert_eq!(
        editor
            .write(&write_input("../escape.txt", "owned"))
            .expect_err("refused")
            .name(),
        "code.OutsideWorkspace"
    );
    assert_eq!(
        editor
            .write(&write_input("/etc/b10x-escape.txt", "owned"))
            .expect_err("refused")
            .name(),
        "code.OutsideWorkspace"
    );
    let sibling = inside
        .path()
        .parent()
        .expect("a temporary directory has a parent")
        .join("escape.txt");
    assert!(!sibling.exists(), "the write escaped upwards");
}

// --- from harness tests.rs ------------------------------------------------------------------------

#[test]
fn a_write_refuses_missing_parents() {
    let dir = workspace();
    let editor = writing(dir.path());

    let refusal = editor
        .write(&write_input("docs/notes/one.md", "written\n"))
        .expect_err("the absent parent is refused");
    assert_eq!(refusal.name(), "code.ParentMissing", "{refusal}");
    assert!(
        refusal
            .to_string()
            .contains("parent directory that does not exist"),
        "{refusal}"
    );
    assert!(!dir.path().join("docs").exists());
}

#[test]
fn a_write_to_a_path_that_does_not_exist_yet_is_still_held_inside_the_tree() {
    // `resolve` cannot answer here: the file is not there to canonicalise. So this is the one
    // place a second containment rule lives, and it is the one a caller reaches for to escape.
    let dir = workspace();
    let editor = writing(dir.path());

    for outside in ["../escaped.txt", "src/../../escaped.txt"] {
        let refused = editor
            .write(&write_input(outside, "nope"))
            .expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "{outside}: {refused}"
        );
    }

    let link = dir.path().join("link");
    symlink("/etc", &link).expect("a link out of the tree");
    let refused = editor
        .write(&write_input("link/escaped.txt", "nope"))
        .expect_err("refused");
    assert_eq!(refused.name(), "code.OutsideWorkspace", "{refused}");
    assert!(
        refused
            .to_string()
            .contains("resolves outside the workspace"),
        "through a symlink, by where it lands: {refused}"
    );
}

#[test]
fn an_edit_that_matches_twice_changes_nothing_and_says_how_many() {
    let dir = workspace();
    let editor = writing(dir.path());
    editor
        .write(&write_input("twice.rs", "let x = 1;\nlet x = 1;\n"))
        .expect("the file");

    let refused = editor
        .edit(&edit_input("twice.rs", "let x = 1;", "let x = 2;"))
        .expect_err("refused");
    assert_eq!(refused.name(), "code.Ambiguous", "{refused}");
    assert!(refused.to_string().contains("appears 2 times"), "{refused}");
    assert_eq!(refused.fields(), json!({ "index": 0, "count": 2 }));
    assert_eq!(
        fs::read_to_string(dir.path().join("twice.rs")).expect("on disk"),
        "let x = 1;\nlet x = 1;\n",
        "and the file is exactly as it was"
    );

    let refused = editor
        .edit(&edit_input("twice.rs", "absent", "x"))
        .expect_err("refused");
    assert_eq!(refused.name(), "code.NoMatch", "{refused}");
    assert!(refused.to_string().contains("appears nowhere"), "{refused}");

    editor
        .edit(&edit_input(
            "twice.rs",
            "let x = 1;\nlet",
            "let x = 2;\nlet",
        ))
        .expect("one place, named by its surroundings");
    assert_eq!(
        fs::read_to_string(dir.path().join("twice.rs")).expect("on disk"),
        "let x = 2;\nlet x = 1;\n"
    );
}

// --- new here -------------------------------------------------------------------------------------

#[test]
fn an_empty_old_is_refused_before_the_file_is_looked_at() {
    let dir = workspace();
    let editor = writing(dir.path());
    let refused = editor
        .edit(&edit_input("missing.rs", "", "x"))
        .expect_err("refused");
    assert_eq!(refused.name(), "code.EmptyOld", "{refused}");
}

#[test]
fn a_stale_digest_refuses_every_operation_that_changes_a_file_and_changes_nothing() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "old\n").expect("the file");
    let stale = Some("0".repeat(64));
    let actual = sha256_hex(b"old\n");

    let refused = editor
        .write(&WriteInput {
            path: "a.txt".into(),
            text: "new\n".into(),
            expected_sha256: stale.clone(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.PreconditionFailed", "{refused}");
    assert_eq!(
        refused.fields(),
        json!({ "expected": "0".repeat(64), "actual": actual })
    );

    let refused = editor
        .edit(&EditInput {
            path: "a.txt".into(),
            old: "old".into(),
            new: "new".into(),
            expected_sha256: stale.clone(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.PreconditionFailed", "{refused}");

    let refused = editor
        .patch(&PatchInput {
            path: "a.txt".into(),
            edits: vec![Edit {
                old: "old".into(),
                new: "new".into(),
            }],
            expected_sha256: stale.clone(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.PreconditionFailed", "{refused}");

    let refused = editor
        .delete(&DeleteInput {
            path: "a.txt".into(),
            expected_sha256: stale,
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.PreconditionFailed", "{refused}");

    assert_eq!(
        fs::read_to_string(dir.path().join("a.txt")).expect("on disk"),
        "old\n"
    );
}

#[test]
fn a_digest_given_for_a_file_that_is_not_there_is_a_mismatch_with_no_actual() {
    let dir = workspace();
    let editor = writing(dir.path());
    let refused = editor
        .write(&WriteInput {
            path: "new.txt".into(),
            text: "x\n".into(),
            expected_sha256: Some("0".repeat(64)),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.PreconditionFailed", "{refused}");
    assert_eq!(refused.fields()["actual"], Value::Null);
    assert!(!dir.path().join("new.txt").exists());
}

#[test]
fn a_patch_resolves_every_edit_against_the_original_and_refuses_overlaps_whole() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "abcd\n").expect("the file");

    let refused = editor
        .patch(&PatchInput {
            path: "a.txt".into(),
            edits: vec![
                Edit {
                    old: "abc".into(),
                    new: "x".into(),
                },
                Edit {
                    old: "bcd".into(),
                    new: "y".into(),
                },
            ],
            expected_sha256: None,
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.Overlap", "{refused}");
    assert_eq!(refused.fields(), json!({ "first": 0, "second": 1 }));
    assert_eq!(
        fs::read_to_string(dir.path().join("a.txt")).expect("on disk"),
        "abcd\n"
    );

    fs::write(dir.path().join("a.txt"), "a\n").expect("the file");
    let refused = editor
        .patch(&PatchInput {
            path: "a.txt".into(),
            edits: vec![
                Edit {
                    old: "a".into(),
                    new: "b".into(),
                },
                Edit {
                    old: "b".into(),
                    new: "c".into(),
                },
            ],
            expected_sha256: None,
        })
        .expect_err(
            "`b` appears nowhere in the original, however the file would read after the first",
        );
    assert_eq!(refused.name(), "code.NoMatch", "{refused}");
    assert_eq!(refused.fields(), json!({ "index": 1 }));
    assert_eq!(
        fs::read_to_string(dir.path().join("a.txt")).expect("on disk"),
        "a\n"
    );
}

#[test]
fn a_patch_applies_disjoint_edits_from_the_end_so_offsets_hold_and_answers_the_count() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "alpha\nbeta\ngamma\n").expect("the file");

    let output = editor
        .patch(&PatchInput {
            path: "a.txt".into(),
            edits: vec![
                Edit {
                    old: "gamma".into(),
                    new: "GAMMA-LONGER".into(),
                },
                Edit {
                    old: "alpha".into(),
                    new: "A".into(),
                },
            ],
            expected_sha256: None,
        })
        .expect("two disjoint edits apply");
    assert_eq!(output.applied, 2);
    assert_eq!(
        fs::read_to_string(dir.path().join("a.txt")).expect("on disk"),
        "A\nbeta\nGAMMA-LONGER\n"
    );
    assert_eq!(output.sha256, sha256_hex(b"A\nbeta\nGAMMA-LONGER\n"));
    assert!(
        output.diff.starts_with("--- a.txt\n+++ a.txt\n"),
        "{}",
        output.diff
    );
    assert!(output.diff.contains("-alpha\n+A\n"), "{}", output.diff);
    assert!(
        output.diff.contains("-gamma\n+GAMMA-LONGER\n"),
        "{}",
        output.diff
    );
}

#[test]
fn an_edit_answers_a_unified_diff_naming_the_workspace_relative_path_twice() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "one\ntwo\n").expect("the file");

    let output = editor
        .edit(&edit_input("./a.txt", "one", "uno"))
        .expect("one match");
    assert_eq!(output.path, "a.txt");
    assert_eq!(output.bytes, 8);
    assert_eq!(
        output.diff,
        "--- a.txt\n+++ a.txt\n@@ -1,2 +1,2 @@\n-one\n+uno\n two\n"
    );
}

#[test]
fn a_move_takes_the_file_to_a_contained_new_path_and_never_replaces_what_is_there() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "x\n").expect("the file");
    fs::write(dir.path().join("taken.txt"), "y\n").expect("the file");

    let refused = editor
        .rename(&MoveInput {
            from: "a.txt".into(),
            to: "taken.txt".into(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.Exists", "{refused}");

    let refused = editor
        .rename(&MoveInput {
            from: "a.txt".into(),
            to: "../a.txt".into(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.OutsideWorkspace", "{refused}");

    let refused = editor
        .rename(&MoveInput {
            from: "a.txt".into(),
            to: "new/a.txt".into(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.ParentMissing", "{refused}");

    let refused = editor
        .rename(&MoveInput {
            from: "missing.txt".into(),
            to: "b.txt".into(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.NotFound", "{refused}");

    let output = editor
        .rename(&MoveInput {
            from: "a.txt".into(),
            to: "b.txt".into(),
        })
        .expect("moved");
    assert_eq!(
        (output.from.as_str(), output.to.as_str()),
        ("a.txt", "b.txt")
    );
    assert!(!dir.path().join("a.txt").exists());
    assert_eq!(
        fs::read_to_string(dir.path().join("b.txt")).expect("on disk"),
        "x\n"
    );
}

#[test]
fn a_delete_removes_a_regular_file_only() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "x\n").expect("the file");
    fs::create_dir(dir.path().join("d")).expect("the directory");
    symlink(dir.path().join("a.txt"), dir.path().join("link")).expect("a link");

    let refused = editor
        .delete(&DeleteInput {
            path: "d".into(),
            expected_sha256: None,
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.NotAFile", "{refused}");

    let refused = editor
        .delete(&DeleteInput {
            path: "link".into(),
            expected_sha256: None,
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.IsSymlink", "{refused}");
    assert!(dir.path().join("link").symlink_metadata().is_ok());
    assert!(dir.path().join("a.txt").exists());

    editor
        .delete(&DeleteInput {
            path: "a.txt".into(),
            expected_sha256: Some(sha256_hex(b"x\n")),
        })
        .expect("deleted");
    assert!(!dir.path().join("a.txt").exists());
}

#[test]
fn invoke_maps_an_input_that_does_not_parse_and_an_unknown_operation_to_named_errors() {
    let dir = workspace();
    let editor = writing(dir.path());
    let Outcome::Error { name, .. } = editor.invoke("code.write", &json!({ "path": "a" })) else {
        panic!("a missing field is an error");
    };
    assert_eq!(name, "code.InvalidInput");
    let Outcome::Error { name, .. } = editor.invoke("code.read", &json!({})) else {
        panic!("an unknown operation is an error");
    };
    assert_eq!(name, "code.UnknownOperation");
}

// --- correction round 1 ---------------------------------------------------------------------------

#[test]
fn a_file_that_is_not_utf8_text_is_refused_as_not_text_and_left_alone() {
    let dir = workspace();
    let editor = writing(dir.path());
    let bytes: &[u8] = &[b'o', b'n', b'e', 0xff, 0xfe, b'\n'];
    fs::write(dir.path().join("bin.dat"), bytes).expect("the file");

    let refused = editor
        .edit(&edit_input("bin.dat", "one", "uno"))
        .expect_err("refused");
    assert_eq!(refused.name(), "code.NotText", "{refused}");
    assert_eq!(refused.fields(), json!({ "path": "bin.dat" }));

    let refused = editor
        .patch(&PatchInput {
            path: "bin.dat".into(),
            edits: vec![Edit {
                old: "one".into(),
                new: "uno".into(),
            }],
            expected_sha256: None,
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.NotText", "{refused}");
    assert_eq!(
        fs::read(dir.path().join("bin.dat")).expect("on disk"),
        bytes,
        "and the file is exactly as it was"
    );
}

#[test]
fn occurrences_are_counted_overlapping_unlike_harness_0_13_3() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "aaa\n").expect("the file");

    let refused = editor
        .edit(&edit_input("a.txt", "aa", "b"))
        .expect_err("`aa` fits at two places in `aaa`");
    assert_eq!(refused.name(), "code.Ambiguous", "{refused}");
    assert_eq!(refused.fields(), json!({ "index": 0, "count": 2 }));
    assert_eq!(
        fs::read_to_string(dir.path().join("a.txt")).expect("on disk"),
        "aaa\n"
    );

    fs::write(dir.path().join("u.txt"), "ééé\n").expect("the file");
    let refused = editor
        .edit(&edit_input("u.txt", "éé", "x"))
        .expect_err("the same over multi-byte characters, advancing on a character boundary");
    assert_eq!(refused.fields(), json!({ "index": 0, "count": 2 }));
}

#[test]
fn a_trailing_slash_or_dot_names_the_same_entry_and_a_link_so_spelled_is_judged_by_where_it_lands()
{
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("real.txt"), "before\n").expect("the file");
    symlink("real.txt", dir.path().join("link")).expect("a link");

    for spelling in ["link/", "link/.", "link/./", "./link/"] {
        assert_eq!(
            editor.lands(spelling).expect("lands"),
            "real.txt",
            "{spelling}"
        );
    }
    let refused = editor
        .delete(&DeleteInput {
            path: "link/".into(),
            expected_sha256: None,
        })
        .expect_err("refused by name");
    assert_eq!(refused.name(), "code.IsSymlink", "{refused}");
    assert_eq!(refused.fields(), json!({ "path": "link/" }));
    assert!(dir.path().join("link").symlink_metadata().is_ok());

    let output = editor
        .edit(&edit_input("link/.", "before", "after"))
        .expect("lands on the target");
    assert_eq!(output.path, "real.txt");
    assert_eq!(
        fs::read_to_string(dir.path().join("real.txt")).expect("on disk"),
        "after\n"
    );
}

#[test]
fn a_move_onto_anything_that_is_there_is_refused_as_exists_once_the_destination_is_contained() {
    // The workspace is a subdirectory of this test's own temporary directory, so the "outside"
    // this test writes to is private to it and no other run can race on it.
    let dir = workspace();
    let root = dir.path().join("ws");
    fs::create_dir(&root).expect("the workspace directory");
    let editor = writing(&root);
    fs::write(root.join("a.txt"), "x\n").expect("the file");
    symlink("missing.txt", root.join("dangling")).expect("a link");
    fs::create_dir(root.join("d")).expect("the directory");

    for taken in ["dangling", "d", "d/"] {
        let refused = editor
            .rename(&MoveInput {
                from: "a.txt".into(),
                to: taken.into(),
            })
            .expect_err("refused");
        assert_eq!(refused.name(), "code.Exists", "{taken}: {refused}");
        assert_eq!(refused.fields(), json!({ "path": taken }));
    }

    // Containment first: a destination that climbs out is refused as outside, whether or not
    // something is there, so nothing about the outside is answered.
    fs::write(dir.path().join("marker.txt"), "x\n").expect("a file outside the workspace");
    let refused = editor
        .rename(&MoveInput {
            from: "a.txt".into(),
            to: "../marker.txt".into(),
        })
        .expect_err("refused");
    assert_eq!(refused.name(), "code.OutsideWorkspace", "{refused}");
    assert!(root.join("a.txt").exists());
}

#[test]
fn an_edit_whose_new_equals_old_answers_the_header_lines_and_a_patch_of_none_answers_nothing() {
    let dir = workspace();
    let editor = writing(dir.path());
    fs::write(dir.path().join("a.txt"), "one\ntwo\n").expect("the file");

    let output = editor
        .edit(&edit_input("a.txt", "one", "one"))
        .expect("one match, replaced with itself");
    assert_eq!(output.diff, "--- a.txt\n+++ a.txt\n");

    let output = editor
        .patch(&PatchInput {
            path: "a.txt".into(),
            edits: vec![],
            expected_sha256: None,
        })
        .expect("nothing to apply");
    assert_eq!((output.applied, output.diff.as_str()), (0, ""));
}
