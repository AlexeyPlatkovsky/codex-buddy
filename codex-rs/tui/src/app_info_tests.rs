use super::*;
use pretty_assertions::assert_eq;

#[test]
fn connector_mention_slug_matches_existing_tui_syntax() {
    assert_eq!(
        ["Google Calendar", "  ", "GitHub.com"].map(connector_mention_slug_from_name),
        ["google-calendar", "app", "github-com"]
    );
}
