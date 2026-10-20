//! Adversary cases: a link to a file *outside* the workspace, named with a trailing `/` (or a
//! trailing `/.`), must still be refused by where it lands. The conformance suite cannot build
//! this fixture, because the victim has to live outside the scenario's workspace.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use tempfile::{TempDir, tempdir_in};
use tool_code_editor::{Edit, EditInput, Editor, PatchInput};

fn workspace() -> TempDir {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adversary");
    fs::create_dir_all(&base).expect("the scratch base");
    tempdir_in(base).expect("a temporary directory")
}

/// A workspace holding `link`, which points at a file outside it holding `original\n`.
fn link_out() -> (TempDir, TempDir, std::path::PathBuf) {
    let outside = workspace();
    let inside = workspace();
    let victim = outside.path().join("victim.txt");
    fs::write(&victim, "original\n").expect("the file is written");
    symlink(&victim, inside.path().join("link")).expect("a link");
    (outside, inside, victim)
}

#[test]
fn an_edit_through_a_link_out_of_the_workspace_spelled_with_a_trailing_slash_is_refused() {
    let (_outside, inside, victim) = link_out();
    let editor = Editor::new(inside.path()).expect("the workspace opens");

    for spelling in ["link/", "link/."] {
        let result = editor.edit(&EditInput {
            path: spelling.to_owned(),
            old: "original".to_owned(),
            new: "owned".to_owned(),
            expected_sha256: None,
        });
        assert_eq!(
            fs::read_to_string(&victim).expect("the file is read"),
            "original\n",
            "{spelling}: the edit followed the link out of the workspace"
        );
        let refusal = result.expect_err("a link that lands outside is refused");
        assert_eq!(
            refusal.name(),
            "code.OutsideWorkspace",
            "{spelling}: {refusal}"
        );
    }
}

#[test]
fn a_patch_through_a_link_out_of_the_workspace_spelled_with_a_trailing_slash_is_refused() {
    let (_outside, inside, victim) = link_out();
    let editor = Editor::new(inside.path()).expect("the workspace opens");

    let result = editor.patch(&PatchInput {
        path: "link/".to_owned(),
        edits: vec![Edit {
            old: "original".to_owned(),
            new: "owned".to_owned(),
        }],
        expected_sha256: None,
    });
    assert_eq!(
        fs::read_to_string(&victim).expect("the file is read"),
        "original\n",
        "the patch followed the link out of the workspace"
    );
    let refusal = result.expect_err("a link that lands outside is refused");
    assert_eq!(refusal.name(), "code.OutsideWorkspace", "{refusal}");
}
