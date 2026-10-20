//! The adversary's case for the guidance figures (wave w3, `story:toolset-search`).
//!
//! `tests::the_contracts_guidance_names_every_figure_the_search_enforces` checks that the
//! guidance contains each figure as a substring. Two of the six cannot fail that way: `5` is in
//! `1048576` and `20` is in `200`, so the guidance could say "capped at 6" and "names the first
//! 21" and the test would stay green. This one checks each figure in the phrase that states it.

use tool_search::{
    MAX_GREP_FILE_BYTES, MAX_GREP_RESULTS, MAX_MATCH_CHARS, MAX_SEARCH_CONTEXT, MAX_SKIPPED_PATHS,
    contract,
};

#[test]
fn the_guidance_states_each_figure_in_the_phrase_that_names_it() {
    let op = contract()
        .operation("search.grep")
        .expect("the contract declares search.grep");
    let guidance = op.guidance.as_deref().expect("guidance");
    for phrase in [
        format!("capped at {MAX_SEARCH_CONTEXT};"),
        format!("a larger ask answers {MAX_SEARCH_CONTEXT} and says so"),
        format!("names the first {MAX_SKIPPED_PATHS}."),
        format!("At most {MAX_GREP_RESULTS} matches"),
        format!("A line over {MAX_MATCH_CHARS} characters"),
        format!("({MAX_GREP_FILE_BYTES} bytes)"),
        format!("at depth {}", tool_fs::walk::MAX_WALK_DEPTH),
    ] {
        assert!(
            guidance.contains(&phrase),
            "{phrase:?} is not in: {guidance}"
        );
    }
}
