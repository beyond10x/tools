//! Adversary pass 2, against the correction: a link in the middle of a path to a file outside,
//! `link/../x`, a destination whose own directory lands outside, a trailing `/` on a directory,
//! an identity edit beside a real one, overlapping counts over multi-byte text, and a file whose
//! invalid bytes follow the match. Each case asserts what `semantics.md` says.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use tempfile::{TempDir, tempdir_in};
use tool_code_editor::{DeleteInput, Edit, EditInput, Editor, MoveInput, PatchInput, WriteInput};

fn workspace() -> TempDir {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adversary-2");
    fs::create_dir_all(&base).expect("the scratch base");
    tempdir_in(base).expect("a temporary directory")
}

fn edit(path: &str, old: &str, new: &str) -> EditInput {
    EditInput {
        path: path.to_owned(),
        old: old.to_owned(),
        new: new.to_owned(),
        expected_sha256: None,
    }
}

fn patch(path: &str, edits: &[(&str, &str)]) -> PatchInput {
    PatchInput {
        path: path.to_owned(),
        edits: edits
            .iter()
            .map(|(old, new)| Edit {
                old: (*old).to_owned(),
                new: (*new).to_owned(),
            })
            .collect(),
        expected_sha256: None,
    }
}

fn write(path: &str, text: &str) -> WriteInput {
    WriteInput {
        path: path.to_owned(),
        text: text.to_owned(),
        expected_sha256: None,
    }
}

fn mv(from: &str, to: &str) -> MoveInput {
    MoveInput {
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

fn del(path: &str) -> DeleteInput {
    DeleteInput {
        path: path.to_owned(),
        expected_sha256: None,
    }
}

/// An outside tree `<outside>/victim.txt` and `<outside>/sub/deep.txt`, and a workspace whose
/// `outdir` links to `<outside>/sub`. `outdir/deep.txt` is a file outside reached through a link
/// in the middle of the path; `outdir/../victim.txt` is `link/../x` with `x` there.
fn through_a_directory_link() -> (TempDir, TempDir, PathBuf, PathBuf) {
    let outside = workspace();
    let inside = workspace();
    let sub = outside.path().join("sub");
    fs::create_dir(&sub).expect("the directory");
    let victim = outside.path().join("victim.txt");
    let deep = sub.join("deep.txt");
    fs::write(&victim, "original\n").expect("the file is written");
    fs::write(&deep, "original\n").expect("the file is written");
    symlink(&sub, inside.path().join("outdir")).expect("a link");
    (outside, inside, victim, deep)
}

#[test]
fn a_link_in_the_middle_of_a_path_to_a_file_outside_is_refused_by_every_operation() {
    let (outside, inside, victim, deep) = through_a_directory_link();
    let editor = Editor::new(inside.path()).expect("the workspace opens");
    fs::write(inside.path().join("a.txt"), "x\n").expect("the file");

    for path in [
        "outdir/deep.txt",
        "outdir/./deep.txt",
        "outdir/deep.txt/",
        "outdir/../victim.txt",
        "outdir/../missing.txt",
    ] {
        let refused = editor
            .edit(&edit(path, "original", "owned"))
            .expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "edit {path}: {refused}"
        );
        let refused = editor
            .patch(&patch(path, &[("original", "owned")]))
            .expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "patch {path}: {refused}"
        );
        let refused = editor.write(&write(path, "owned\n")).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "write {path}: {refused}"
        );
        let refused = editor.delete(&del(path)).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "delete {path}: {refused}"
        );
        let refused = editor.rename(&mv(path, "b.txt")).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "move from {path}: {refused}"
        );
        let refused = editor.rename(&mv("a.txt", path)).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "move to {path}: {refused}"
        );
    }
    assert_eq!(fs::read_to_string(&victim).expect("read"), "original\n");
    assert_eq!(fs::read_to_string(&deep).expect("read"), "original\n");
    assert!(!outside.path().join("missing.txt").exists());
    assert!(inside.path().join("a.txt").is_file());
    assert!(!inside.path().join("b.txt").exists());
}

#[test]
fn a_destination_whose_directory_lands_outside_is_refused_whether_or_not_something_is_there() {
    let (outside, inside, _victim, _deep) = through_a_directory_link();
    let editor = Editor::new(inside.path()).expect("the workspace opens");
    fs::write(inside.path().join("a.txt"), "x\n").expect("the file");

    for to in [
        "outdir/new.txt",
        "outdir/new.txt/",
        "outdir/deep.txt",
        "outdir/..",
        "outdir/../new.txt",
        "../new.txt",
    ] {
        let refused = editor.rename(&mv("a.txt", to)).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.OutsideWorkspace",
            "to {to}: {refused}"
        );
    }
    assert!(inside.path().join("a.txt").is_file());
    assert!(!outside.path().join("sub/new.txt").exists());
    assert!(!outside.path().join("new.txt").exists());
    assert!(
        !inside
            .path()
            .parent()
            .expect("a parent")
            .join("new.txt")
            .exists()
    );
}

#[test]
fn a_trailing_slash_on_a_directory_names_the_directory_for_every_operation() {
    let inside = workspace();
    let editor = Editor::new(inside.path()).expect("the workspace opens");
    fs::create_dir(inside.path().join("dir")).expect("the directory");
    fs::write(inside.path().join("dir/keep"), "keep\n").expect("the file");
    fs::write(inside.path().join("a.txt"), "x\n").expect("the file");

    for spelling in ["dir/", "dir/.", "dir/./"] {
        let refused = editor
            .write(&write(spelling, "nope\n"))
            .expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.NotAFile",
            "write {spelling}: {refused}"
        );
        let refused = editor
            .edit(&edit(spelling, "keep", "x"))
            .expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.NotAFile",
            "edit {spelling}: {refused}"
        );
        let refused = editor.delete(&del(spelling)).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.NotAFile",
            "delete {spelling}: {refused}"
        );
        let refused = editor.rename(&mv(spelling, "other")).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.NotAFile",
            "move from {spelling}: {refused}"
        );
        let refused = editor.rename(&mv("a.txt", spelling)).expect_err("refused");
        assert_eq!(
            refused.name(),
            "code.Exists",
            "move to {spelling}: {refused}"
        );
    }
    assert_eq!(
        fs::read_to_string(inside.path().join("dir/keep")).expect("read"),
        "keep\n"
    );
    assert!(inside.path().join("dir").is_dir());
    assert!(inside.path().join("a.txt").is_file());
    assert!(!inside.path().join("other").exists());
}

#[test]
fn an_identity_edit_beside_a_real_one_is_counted_and_only_the_real_change_is_a_hunk() {
    let inside = workspace();
    let editor = Editor::new(inside.path()).expect("the workspace opens");
    fs::write(inside.path().join("a.txt"), "one\ntwo\n").expect("the file");

    let output = editor
        .patch(&patch("a.txt", &[("one", "one"), ("two", "TWO")]))
        .expect("both apply");
    assert_eq!(output.applied, 2);
    assert!(
        output.diff.starts_with("--- a.txt\n+++ a.txt\n@@"),
        "{}",
        output.diff
    );
    assert!(output.diff.contains("-two\n+TWO\n"), "{}", output.diff);
    assert!(!output.diff.contains("-one\n"), "{}", output.diff);
    assert_eq!(
        fs::read_to_string(inside.path().join("a.txt")).expect("read"),
        "one\nTWO\n"
    );

    let output = editor
        .patch(&patch("a.txt", &[("one", "one")]))
        .expect("an identity edit applies");
    assert_eq!(
        (output.applied, output.diff.as_str()),
        (1, "--- a.txt\n+++ a.txt\n")
    );
    assert_eq!(
        fs::read_to_string(inside.path().join("a.txt")).expect("read"),
        "one\nTWO\n"
    );
}

#[test]
fn overlapping_occurrences_are_counted_over_multi_byte_text() {
    let inside = workspace();
    let editor = Editor::new(inside.path()).expect("the workspace opens");

    for (text, old, count) in [
        ("日日日日\n", "日日", 3),
        ("aéaéa\n", "aéa", 2),
        ("éaéa\n", "éa", 2),
        ("日本日本語\n", "日本", 2),
        ("ééé\n", "éé", 2),
    ] {
        fs::write(inside.path().join("u.txt"), text).expect("the file");
        let refused = editor
            .edit(&edit("u.txt", old, "x"))
            .expect_err("ambiguous");
        assert_eq!(
            refused.name(),
            "code.Ambiguous",
            "{text:?}/{old:?}: {refused}"
        );
        assert_eq!(
            refused.fields()["count"],
            count,
            "{text:?}/{old:?}: {refused}"
        );
        assert_eq!(
            fs::read_to_string(inside.path().join("u.txt")).expect("read"),
            text
        );
    }

    fs::write(inside.path().join("u.txt"), "éé\n").expect("the file");
    let output = editor
        .edit(&edit("u.txt", "éé", "x"))
        .expect("one occurrence");
    assert_eq!(output.bytes, 2);
    assert_eq!(
        fs::read_to_string(inside.path().join("u.txt")).expect("read"),
        "x\n"
    );
}

#[test]
fn a_delete_of_a_link_is_refused_by_name_whatever_it_points_at() {
    // semantics.md, code.delete: "a link is `code.IsSymlink`, refused by the name the caller gave
    // however it is spelled (`link/` included), whatever it points at."
    let outside = workspace();
    let inside = workspace();
    let victim = outside.path().join("victim.txt");
    fs::write(&victim, "original\n").expect("the file is written");
    symlink(&victim, inside.path().join("link")).expect("a link");
    let editor = Editor::new(inside.path()).expect("the workspace opens");

    for spelling in ["link", "link/"] {
        let refused = editor.delete(&del(spelling)).expect_err("refused");
        assert_eq!(refused.name(), "code.IsSymlink", "{spelling}: {refused}");
    }
    assert!(victim.is_file(), "the target outside is untouched");
    assert!(inside.path().join("link").symlink_metadata().is_ok());
}

#[test]
fn a_file_whose_invalid_bytes_follow_the_match_is_still_not_text_and_unchanged() {
    let inside = workspace();
    let editor = Editor::new(inside.path()).expect("the workspace opens");
    let bytes: &[u8] = b"one\ntwo\n\xff\n";
    fs::write(inside.path().join("bin.txt"), bytes).expect("the file");

    let refused = editor
        .edit(&edit("bin.txt", "one", "uno"))
        .expect_err("refused");
    assert_eq!(refused.name(), "code.NotText", "{refused}");
    let refused = editor
        .patch(&patch("bin.txt", &[("one", "uno")]))
        .expect_err("refused");
    assert_eq!(refused.name(), "code.NotText", "{refused}");
    assert_eq!(
        fs::read(inside.path().join("bin.txt")).expect("read"),
        bytes
    );
}
