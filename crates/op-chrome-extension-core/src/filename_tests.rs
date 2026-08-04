//! Tests for [`crate::filename`] — download-name sanitisation of an
//! attacker-controlled page title.

use crate::filename::snapshot_filename;

#[test]
fn keeps_an_ordinary_title_readable() {
    assert_eq!(
        snapshot_filename("Example Domain"),
        "Example Domain-snapshot.json"
    );
    assert_eq!(snapshot_filename("周报 2026"), "周报 2026-snapshot.json");
}

#[test]
fn falls_back_when_the_title_sanitises_to_nothing() {
    for title in ["", "   ", "...", "///", "\u{0}\u{1}", "..", "."] {
        assert_eq!(
            snapshot_filename(title),
            "page-snapshot.json",
            "title {title:?}"
        );
    }
}

#[test]
fn no_title_can_produce_a_parent_directory_component() {
    // `chrome.downloads` rejects a name containing `..` outright, which would
    // turn a capture into an opaque failure.
    for title in [
        "../../etc/passwd",
        "..\\..\\windows",
        "a/../../b",
        "....//....//x",
    ] {
        let name = snapshot_filename(title);
        assert!(!name.contains(".."), "{title:?} produced {name:?}");
        assert!(!name.contains('/'), "{title:?} produced {name:?}");
        assert!(!name.contains('\\'), "{title:?} produced {name:?}");
    }
}

#[test]
fn strips_path_separators_and_reserved_characters() {
    assert_eq!(
        snapshot_filename("a/b\\c:d*e?f\"g<h>i|j%k"),
        "a b c d e f g h i j k-snapshot.json"
    );
}

#[test]
fn strips_control_characters_including_newlines_and_nul() {
    assert_eq!(
        snapshot_filename("a\u{0}b\nc\rd\te"),
        "a b c d e-snapshot.json"
    );
    // A control run collapses to a single space, like the JS `+` quantifier.
    assert_eq!(snapshot_filename("a\u{1}\u{2}\u{3}b"), "a b-snapshot.json");
}

#[test]
fn never_produces_a_dot_file() {
    assert_eq!(snapshot_filename(".bashrc"), "bashrc-snapshot.json");
    assert_eq!(snapshot_filename("   .hidden"), "hidden-snapshot.json");
}

#[test]
fn collapses_dot_runs_but_keeps_a_single_dot() {
    assert_eq!(snapshot_filename("report.v2"), "report.v2-snapshot.json");
    assert_eq!(snapshot_filename("report...v2"), "report.v2-snapshot.json");
}

#[test]
fn caps_the_stem_and_trims_what_the_cut_leaves_behind() {
    let long = "x".repeat(200);
    assert_eq!(
        snapshot_filename(&long),
        format!("{}-snapshot.json", "x".repeat(80))
    );

    // The 80th unit lands on a space, which Windows would silently drop.
    let with_space = format!("{} tail", "y".repeat(79));
    assert_eq!(
        snapshot_filename(&with_space),
        format!("{}-snapshot.json", "y".repeat(79))
    );

    // …and on a dot.
    let with_dot = format!("{}.tail", "z".repeat(79));
    assert_eq!(
        snapshot_filename(&with_dot),
        format!("{}-snapshot.json", "z".repeat(79))
    );
}

#[test]
fn measures_the_cap_in_utf16_code_units_like_js_does() {
    // 40 astral characters are 80 UTF-16 code units — exactly the cap.
    let emoji = "😀".repeat(50);
    let name = snapshot_filename(&emoji);
    let stem = name.trim_end_matches("-snapshot.json");
    assert_eq!(stem.chars().count(), 40);
}

#[test]
fn collapses_whitespace_runs_including_the_byte_order_mark() {
    assert_eq!(snapshot_filename("a \t\n b"), "a b-snapshot.json");
    assert_eq!(
        snapshot_filename("\u{feff}title\u{feff}"),
        "title-snapshot.json"
    );
    assert_eq!(snapshot_filename("a\u{a0}\u{3000}b"), "a b-snapshot.json");
}
