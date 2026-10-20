//! The search's tests, ported from harness's `local.rs` `mod tests` at harness 0.14.1: those about
//! `search`, adapted to the typed API with their assertions kept, and the search half of the two
//! that also held `find` (which `tool-fs` ported for `fs.glob`). After them, the cases for what
//! this crate corrected or added: the result cap tested on matches, the empty glob, the pipe that
//! is not read, the path that climbs out, the refusal names, the unknown input field, and the
//! contract's guidance naming the figures the code enforces.

use std::collections::BTreeSet;
use std::fs;
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

fn reading(root: &Path) -> Search {
    Search::new(root).expect("the workspace opens")
}

fn paths(output: &GrepOutput) -> Vec<&str> {
    output.matches.iter().map(|hit| hit.path.as_str()).collect()
}

fn bound() -> usize {
    usize::try_from(MAX_GREP_FILE_BYTES).expect("the search bound fits usize")
}

#[test]
fn a_match_that_was_cut_says_so() {
    // Nothing is truncated silently. A 400-character cap is fine; a cap the reader cannot see is
    // a line the model believes it has read whole.
    let inside = workspace();
    let long = format!("{}needle{}", "a".repeat(500), "b".repeat(500));
    fs::write(inside.path().join("long.txt"), format!("{long}\nneedle\n"))
        .expect("the file is written");

    let value = reading(inside.path())
        .grep(&GrepInput::new("needle"))
        .expect("the search answers");
    assert_eq!(value.matches.len(), 2);
    assert!(value.matches[0].line_truncated);
    assert_eq!(value.matches[0].text.chars().count(), MAX_MATCH_CHARS);
    assert!(!value.matches[1].line_truncated);
}

#[test]
fn a_search_names_the_large_files_it_did_not_inspect() {
    let inside = workspace();
    fs::write(
        inside.path().join("large.txt"),
        format!("needle{}", "x".repeat(bound())),
    )
    .expect("large file");
    let value = reading(inside.path())
        .grep(&GrepInput::new("needle"))
        .expect("search answers incompletely");
    assert_eq!(value.matches, []);
    assert!(value.truncated);
    assert_eq!(value.skipped_large_files, 1);
    assert_eq!(value.skipped_large_paths, ["large.txt"]);
}

#[test]
fn the_large_file_bound_is_checked_in_bytes_at_both_edges() {
    let inside = workspace();
    let limit = bound();
    for (name, total) in [("below.txt", limit - 1), ("exact.txt", limit)] {
        let prefix = "needle ";
        fs::write(
            inside.path().join(name),
            format!("{prefix}{}", "x".repeat(total - prefix.len())),
        )
        .expect("bounded file");
    }
    let prefix = "needle é";
    fs::write(
        inside.path().join("multibyte.txt"),
        format!("{prefix}{}", "x".repeat(limit - prefix.len())),
    )
    .expect("bounded multibyte file");
    fs::write(
        inside.path().join("over.txt"),
        format!("needle {}", "x".repeat(limit + 1 - "needle ".len())),
    )
    .expect("over-bound file");

    let value = reading(inside.path())
        .grep(&GrepInput::new("needle"))
        .expect("search answers and marks the skipped file");
    assert_eq!(paths(&value), ["below.txt", "exact.txt", "multibyte.txt"]);
    assert_eq!(value.skipped_large_files, 1);
    assert_eq!(value.skipped_large_paths, ["over.txt"]);
    assert!(value.truncated);
}

#[test]
fn a_search_can_be_a_regular_expression_and_a_broken_one_is_refused_by_the_regexs_own_words() {
    let inside = workspace();
    fs::write(inside.path().join("a.rs"), "fn alpha() {}\nfn beta() {}\n")
        .expect("the file is written");
    let local = reading(inside.path());

    let value = local
        .grep(&GrepInput {
            regex: Some(true),
            ..GrepInput::new(r"fn\s+(alpha|gamma)")
        })
        .expect("the search answers");
    assert_eq!(value.matches.len(), 1);
    assert_eq!(value.matches[0].line, 1);
    assert_eq!(value.regex, Some(true));

    // Literal is still the default, so a model that meant `a.b` gets `a.b`.
    let value = local
        .grep(&GrepInput::new("fn (alpha"))
        .expect("the search answers");
    assert_eq!(value.matches, []);
    assert_eq!(value.regex, None, "a literal search does not say regex");

    let refusal = local
        .grep(&GrepInput {
            regex: Some(true),
            ..GrepInput::new("fn (alpha")
        })
        .expect_err("refused");
    let refusal = refusal.to_string();
    assert!(refusal.contains("not a regular expression"), "{refusal}");
    assert!(
        refusal.contains("unclosed group"),
        "with the regex's own words: {refusal}"
    );
}

#[test]
fn a_glob_narrows_a_search_to_the_files_it_names_and_context_comes_back_numbered() {
    let inside = workspace();
    fs::create_dir(inside.path().join("src")).expect("the directory is made");
    fs::write(inside.path().join("src/one.rs"), "a\nb\nneedle\nd\ne\n")
        .expect("the file is written");
    fs::write(inside.path().join("notes.md"), "needle\n").expect("the file is written");
    let local = reading(inside.path());

    let value = local
        .grep(&GrepInput {
            glob: Some("*.rs".to_owned()),
            context: Some(1),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(value.matches.len(), 1, "the markdown file is not a `*.rs`");
    assert_eq!(value.matches[0].path, "src/one.rs");
    assert_eq!(
        serde_json::to_value(&value.matches[0].before).expect("serialises"),
        json!([{"line": 2, "text": "b", "line_truncated": false}])
    );
    assert_eq!(
        serde_json::to_value(&value.matches[0].after).expect("serialises"),
        json!([{"line": 4, "text": "d", "line_truncated": false}])
    );

    // A glob with a separator in it is matched against the whole workspace-relative path.
    let value = local
        .grep(&GrepInput {
            glob: Some("src/**".to_owned()),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(value.matches.len(), 1);
    assert!(
        serde_json::to_value(&value.matches[0])
            .expect("serialises")
            .get("before")
            .is_none(),
        "context is answered only where it was asked for"
    );
}

#[test]
fn a_search_stopped_by_the_depth_bound_says_so_and_one_that_reached_the_bottom_says_it_did_not() {
    // `search` says it reads every file. It stops descending at the walk's depth bound, and used
    // to answer `truncated: false` over the part of the tree it had seen.
    let deep = workspace();
    let mut path = deep.path().to_path_buf();
    for level in 0..14 {
        path = path.join(format!("d{level}"));
    }
    fs::create_dir_all(&path).expect("the directories are made");
    fs::write(path.join("buried.txt"), "needle\n").expect("the file is written");

    let value = reading(deep.path())
        .grep(&GrepInput::new("needle"))
        .expect("the search answers");
    assert!(value.depth_bound_reached);
    assert_eq!(
        value.matches,
        [],
        "the file is past the bound, and the flag is the only thing that says so"
    );

    let shallow = workspace();
    fs::create_dir_all(shallow.path().join("a/b")).expect("the directories are made");
    fs::write(shallow.path().join("a/b/near.txt"), "needle\n").expect("the file is written");
    let value = reading(shallow.path())
        .grep(&GrepInput::new("needle"))
        .expect("the search answers");
    assert!(!value.depth_bound_reached);
    assert_eq!(paths(&value), ["a/b/near.txt"]);
}

#[test]
fn a_link_out_of_the_workspace_is_not_a_way_for_search_to_read_what_is_outside() {
    // The containment re-check inside the walk is the boundary this stands on: `is_dir` and
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

    let value = reading(inside.path())
        .grep(&GrepInput::new("needle"))
        .expect("the search answers");
    assert_eq!(paths(&value), ["own.txt"], "and nothing outside was read");

    let refusal = reading(inside.path())
        .grep(&GrepInput {
            path: Some("out".to_owned()),
            ..GrepInput::new("needle")
        })
        .expect_err("a search started through the link is refused");
    assert_eq!(refusal.name(), "search.OutsideWorkspace");
}

#[test]
fn a_regular_expression_too_large_for_this_builds_limit_is_refused_by_the_crates_own_words() {
    // The pattern comes from the model, and the crate's default ceiling is ten megabytes of this
    // process's memory for one search.
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "aaa\n").expect("the file is written");

    let refusal = reading(inside.path())
        .grep(&GrepInput {
            regex: Some(true),
            ..GrepInput::new("(a{1000}){1000}")
        })
        .expect_err("refused")
        .to_string();
    assert!(refusal.contains("not a regular expression"), "{refusal}");
    assert!(
        refusal.contains("size limit"),
        "with the crate's own words: {refusal}"
    );
}

#[test]
fn a_capped_context_is_echoed_not_applied_silently() {
    let inside = workspace();
    fs::write(
        inside.path().join("a.txt"),
        "1\n2\n3\n4\n5\n6\n7\n8\nneedle\n",
    )
    .expect("the file is written");
    let local = reading(inside.path());

    let value = local
        .grep(&GrepInput {
            context: Some(40),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(value.context, Some(MAX_SEARCH_CONTEXT));
    assert!(
        value.note.as_deref().expect("a note").contains("40"),
        "{:?}",
        value.note
    );
    assert_eq!(
        value.matches[0].before.as_ref().expect("before").len(),
        usize::try_from(MAX_SEARCH_CONTEXT).expect("five fits"),
        "and the matches carry the figure the reply echoed"
    );

    // Asking for what the cap allows is not a cap, so nothing is echoed.
    let value = local
        .grep(&GrepInput {
            context: Some(2),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(value.context, None);
    assert_eq!(value.note, None);
}

#[test]
fn an_empty_pattern_is_refused_by_name() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "needle\n").expect("the file is written");
    let refusal = reading(inside.path())
        .grep(&GrepInput::new(""))
        .expect_err("refused");
    assert_eq!(refusal.name(), "search.InvalidPattern");
    assert!(
        refusal.to_string().contains("`pattern` is required"),
        "{refusal}"
    );
}

#[test]
fn an_empty_glob_is_refused_by_name_rather_than_matching_no_file() {
    // harness compiled an empty glob into a filter that matched no file and answered an empty
    // list, which reads as *the text is not in the tree*. `fs.glob` refuses an empty glob, and
    // this is the same refusal.
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "needle\n").expect("the file is written");
    let refusal = reading(inside.path())
        .grep(&GrepInput {
            glob: Some(String::new()),
            ..GrepInput::new("needle")
        })
        .expect_err("refused");
    assert_eq!(refusal.name(), "search.InvalidGlob");
    assert_eq!(refusal.fields(), json!({ "glob": "" }));

    let refusal = reading(inside.path())
        .grep(&GrepInput {
            glob: Some("src/[".to_owned()),
            ..GrepInput::new("needle")
        })
        .expect_err("refused");
    assert_eq!(refusal.name(), "search.InvalidGlob");
    assert!(refusal.to_string().contains("not a glob"), "{refusal}");
}

#[test]
fn exactly_max_results_matches_is_a_complete_answer() {
    // harness tested the cap before every file and every line, so exactly `max_results` matches
    // followed by anything at all answered `truncated: true`. `truncated` says a match past the
    // cap exists, as `fs.glob`'s does since wave w2.
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "needle\nneedle\nother\n").expect("written");
    fs::write(inside.path().join("b.txt"), "other\n").expect("written");
    let local = reading(inside.path());

    let value = local
        .grep(&GrepInput {
            max_results: Some(2),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(value.matches.len(), 2);
    assert!(!value.truncated, "nothing past the cap matched");

    let value = local
        .grep(&GrepInput {
            max_results: Some(1),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(value.matches.len(), 1);
    assert!(value.truncated, "the second match is past the cap");
}

#[test]
fn a_search_stops_at_the_first_match_past_the_cap_and_counts_only_what_it_met() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "needle\n").expect("written");
    fs::write(
        inside.path().join("b.txt"),
        format!("needle{}", "x".repeat(bound())),
    )
    .expect("written");
    fs::write(inside.path().join("c.txt"), "needle\n").expect("written");
    fs::write(
        inside.path().join("d.txt"),
        format!("needle{}", "x".repeat(bound())),
    )
    .expect("written");

    let value = reading(inside.path())
        .grep(&GrepInput {
            max_results: Some(1),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(paths(&value), ["a.txt"]);
    assert!(value.truncated);
    assert_eq!(
        value.skipped_large_paths,
        ["b.txt"],
        "b.txt was met before the match past the cap in c.txt; d.txt was never reached"
    );
    assert_eq!(value.skipped_large_files, 1);
}

#[test]
fn only_the_first_twenty_large_files_are_named_and_every_one_is_counted() {
    let inside = workspace();
    let large = format!("needle{}", "x".repeat(bound()));
    for index in 0..22 {
        fs::write(inside.path().join(format!("f{index:02}.txt")), &large).expect("written");
    }
    let value = reading(inside.path())
        .grep(&GrepInput::new("needle"))
        .expect("the search answers");
    assert_eq!(value.skipped_large_files, 22);
    assert_eq!(value.skipped_large_paths.len(), MAX_SKIPPED_PATHS);
    assert_eq!(value.skipped_large_paths[0], "f00.txt");
    assert_eq!(value.skipped_large_paths[19], "f19.txt");
    assert!(value.truncated);
}

#[test]
fn a_named_pipe_in_the_tree_is_passed_over_rather_than_read() {
    // Opening a pipe for reading blocks until a writer opens it, and there is none. harness read
    // every entry the walk reached that was not a directory, so one pipe in the tree held the
    // call open for good; only regular files are read now.
    let inside = workspace();
    fs::write(inside.path().join("own.txt"), "needle\n").expect("the file is written");
    let made = std::process::Command::new("mkfifo")
        .arg(inside.path().join("pipe"))
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "the pipe is made");
    let search = reading(inside.path());
    let (answer, answered) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = answer.send(search.grep(&GrepInput::new("needle")));
    });
    let value = answered
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the search answers rather than blocking on the pipe")
        .expect("the search answers");
    assert_eq!(paths(&value), ["own.txt"]);
}

#[test]
fn an_entry_with_a_skipped_name_is_not_searched_whatever_kind_of_entry_it_is() {
    // Correction round 1, F1. The walk checks the skipped names on directories only, so a
    // worktree's `.git` file was searched while the guidance said `.git` is skipped. The rule is
    // the entry's own name, for every kind of entry the walk can meet: a directory, a file, a
    // link to a file and a link to a directory.
    let inside = workspace();
    let root = inside.path();
    fs::write(root.join("notes.md"), "needle\n").expect("written");
    fs::write(
        root.join(".git"),
        "gitdir: ../main/.git/worktrees/x needle\n",
    )
    .expect("written");
    fs::write(root.join("target"), "needle\n").expect("written");
    fs::create_dir_all(root.join("node_modules/m")).expect("made");
    fs::write(root.join("node_modules/m/index.js"), "needle\n").expect("written");
    fs::create_dir_all(root.join("real")).expect("made");
    fs::write(root.join("real/inner.txt"), "needle\n").expect("written");
    symlink("notes.md", root.join(".venv")).expect("a link to a file");
    symlink("real", root.join("__pycache__")).expect("a link to a directory");
    for skipped in tool_fs::walk::SKIPPED {
        assert!(
            root.join(skipped).symlink_metadata().is_ok(),
            "{skipped} is in the tree"
        );
    }

    let value = reading(root)
        .grep(&GrepInput::new("needle"))
        .expect("the search answers");
    assert_eq!(paths(&value), ["notes.md", "real/inner.txt"]);
    assert!(!value.truncated);
}

#[test]
fn a_path_that_names_a_skipped_entry_is_searched_because_it_was_asked_for() {
    // The skipped names keep machine output out of a search of the tree; a caller who names one
    // as `path` asked for it, as `fs.glob` from a `target` directory walks that directory.
    let inside = workspace();
    fs::write(inside.path().join(".git"), "needle\n").expect("written");
    fs::create_dir_all(inside.path().join("target/debug")).expect("made");
    fs::write(inside.path().join("target/debug/built.txt"), "needle\n").expect("written");
    let local = reading(inside.path());
    for (path, found) in [(".git", ".git"), ("target", "target/debug/built.txt")] {
        let value = local
            .grep(&GrepInput {
                path: Some(path.to_owned()),
                ..GrepInput::new("needle")
            })
            .expect("the search answers");
        assert_eq!(paths(&value), [found], "{path}");
    }
}

#[test]
fn a_file_that_holds_a_nul_byte_is_binary_and_is_not_searched() {
    // Correction round 1, F2. semantics.md says a binary file is not searched, and a file that is
    // valid UTF-8 with NUL bytes in it was searched and its NUL bytes answered. A NUL byte makes
    // a file binary, wherever it is, and a binary file is passed over without a word, as one that
    // is not UTF-8 always was. Other control bytes do not.
    let inside = workspace();
    fs::write(inside.path().join("blob.dat"), "needle\0\0\0\x01\x02\n").expect("written");
    fs::write(
        inside.path().join("late.dat"),
        format!("needle\n{}\0\n", "x".repeat(5000)),
    )
    .expect("written");
    fs::write(inside.path().join("bell.txt"), "needle\x07\n").expect("written");
    fs::write(inside.path().join("text.txt"), "needle\n").expect("written");
    let local = reading(inside.path());

    let value = local
        .grep(&GrepInput::new("needle"))
        .expect("the search answers");
    assert_eq!(paths(&value), ["bell.txt", "text.txt"]);
    assert!(!value.truncated, "a binary file is not a cut in the answer");
    assert_eq!(value.skipped_large_files, 0);

    let value = local
        .grep(&GrepInput {
            path: Some("blob.dat".to_owned()),
            ..GrepInput::new("needle")
        })
        .expect("the search answers");
    assert_eq!(value.matches, [], "named by path, it is still binary");
}

#[test]
fn a_path_that_climbs_out_is_outside_whether_or_not_anything_is_there() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "needle\n").expect("written");
    let local = reading(inside.path());
    for path in ["..", "../nowhere-at-all", "/"] {
        let refusal = local
            .grep(&GrepInput {
                path: Some(path.to_owned()),
                ..GrepInput::new("needle")
            })
            .expect_err("refused");
        assert_eq!(refusal.name(), "search.OutsideWorkspace", "{path}");
        assert_eq!(refusal.fields(), json!({ "path": path }));
    }
    for path in ["nowhere", "a.txt/x"] {
        let refusal = local
            .grep(&GrepInput {
                path: Some(path.to_owned()),
                ..GrepInput::new("needle")
            })
            .expect_err("refused");
        assert_eq!(refusal.name(), "search.NotFound", "{path}");
    }
}

#[test]
fn every_refusal_is_named_by_the_contract_and_carries_its_declared_fields() {
    // The class, checked rather than listed in prose: a refusal whose name the contract does not
    // declare, or whose fields differ from the declaration, would fail every scenario that hit
    // it. Every variant is constructed here; a new one has to be added to this list.
    let gone = || io::Error::new(io::ErrorKind::NotFound, "gone");
    let broken_pattern = search_expression("(", true).expect_err("a broken expression");
    assert!(matches!(broken_pattern, SearchError::InvalidPattern { .. }));
    let broken_glob = compile_glob("[").expect_err("a broken glob");
    assert!(matches!(broken_glob, SearchError::InvalidGlob { .. }));
    let every = [
        SearchError::Workspace {
            path: "w".to_owned(),
            source: gone(),
        },
        SearchError::WorkspaceNotADirectory {
            path: "w".to_owned(),
        },
        SearchError::EmptyPattern,
        broken_pattern,
        SearchError::EmptyGlob,
        broken_glob,
        SearchError::NotFound {
            path: "p".to_owned(),
            source: gone(),
        },
        SearchError::OutsideWorkspace {
            path: "p".to_owned(),
        },
        SearchError::Io {
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
fn an_input_the_operation_does_not_declare_is_invalid_input_through_the_toolset() {
    // semantics.md: an unknown field is `search.InvalidInput`. The suite format cannot hold this
    // (the runner refuses an unknown `call.input` key at coverage), so the crate holds it here,
    // beside the missing field and the wrong type the suite does hold.
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "needle\n").expect("the file is written");
    let local = reading(inside.path());
    for input in [
        json!({ "pattern": "needle", "case_insensitive": true }),
        json!({ "pattern": "needle", "options": { "regex": true } }),
        json!({}),
        json!({ "pattern": "needle", "max_results": "ten" }),
    ] {
        match local.invoke("search.grep", &input) {
            Outcome::Error { name, fields, .. } => {
                assert_eq!(name, "search.InvalidInput", "{input}");
                assert_eq!(fields, json!({}), "{input}");
            }
            Outcome::Ok(output) => panic!("{input} answered {output}"),
        }
    }
    assert!(
        matches!(
            local.invoke("search.grep", &json!({ "pattern": "needle" })),
            Outcome::Ok(_)
        ),
        "the same input without the unknown field is answered"
    );
}

#[test]
fn the_contracts_guidance_names_every_figure_the_search_enforces() {
    // A bound the model cannot read is one it cannot work around, so each figure is in the
    // guidance; and the walk's figures are `tool-fs`'s, so a change there shows here.
    let op = contract()
        .operation("search.grep")
        .expect("the contract declares search.grep");
    let guidance = op.guidance.as_deref().expect("guidance");
    for skipped in tool_fs::walk::SKIPPED {
        assert!(guidance.contains(skipped), "{skipped}: {guidance}");
    }
    for figure in [
        tool_fs::walk::MAX_WALK_DEPTH.to_string(),
        MAX_GREP_RESULTS.to_string(),
        MAX_GREP_FILE_BYTES.to_string(),
        MAX_SEARCH_CONTEXT.to_string(),
        MAX_MATCH_CHARS.to_string(),
        MAX_SKIPPED_PATHS.to_string(),
    ] {
        assert!(guidance.contains(&figure), "{figure}: {guidance}");
    }
}

#[test]
fn the_workspace_must_be_a_directory() {
    let inside = workspace();
    fs::write(inside.path().join("a.txt"), "x\n").expect("written");
    let refusal = Search::new(inside.path().join("a.txt")).expect_err("a file is not a root");
    assert_eq!(refusal.name(), "search.Io");
    let refusal = Search::new(inside.path().join("missing")).expect_err("nothing is not a root");
    assert_eq!(refusal.name(), "search.Io");
    assert_eq!(
        reading(inside.path()).root(),
        inside.path().canonicalize().expect("canonical")
    );
}
