use super::*;
use pretty_assertions::assert_eq;

#[test]
fn rounds_context_usage_to_thousands() {
    assert_eq!(format_rounded_tokens(499), "<1K");
    assert_eq!(format_rounded_tokens(500), "1K");
    assert_eq!(format_rounded_tokens(12_500), "13K");
}
