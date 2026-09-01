import importlib.util
from pathlib import Path
import unittest


SCRIPT_PATH = Path(__file__).with_name("check_buddy_version.py")
SPEC = importlib.util.spec_from_file_location("check_buddy_version", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
CHECK_BUDDY_VERSION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK_BUDDY_VERSION)


class CheckBuddyVersionTests(unittest.TestCase):
    def test_matching_versions_are_valid(self) -> None:
        versions = CHECK_BUDDY_VERSION.versions_from_text(
            '[package]\nname = "codex-buddy"\nversion = "1.0.1"\n',
            '[[package]]\nname = "codex-buddy"\nversion = "1.0.1"\n',
            '#[cfg(feature = "buddy-branding")]\n'
            'pub(crate) const PRODUCT_DISPLAY_VERSION: &str = "1.0.1";\n',
        )

        self.assertEqual(CHECK_BUDDY_VERSION.validate_versions(versions), [])

    def test_mismatched_display_version_is_rejected(self) -> None:
        versions = CHECK_BUDDY_VERSION.BuddyVersions(
            cargo_toml="1.0.1",
            cargo_lock="1.0.1",
            tui_display="1.0.0",
        )

        self.assertEqual(
            CHECK_BUDDY_VERSION.validate_versions(versions),
            [
                "Codex Buddy versions must match: "
                "Cargo.toml=1.0.1, Cargo.lock=1.0.1, TUI=1.0.0"
            ],
        )

    def test_non_semver_version_is_rejected(self) -> None:
        versions = CHECK_BUDDY_VERSION.BuddyVersions(
            cargo_toml="fresh",
            cargo_lock="fresh",
            tui_display="fresh",
        )

        self.assertEqual(
            CHECK_BUDDY_VERSION.validate_versions(versions),
            [
                "codex-rs/codex-buddy/Cargo.toml has a non-SemVer Buddy version: 'fresh'",
                "codex-rs/Cargo.lock has a non-SemVer Buddy version: 'fresh'",
                "codex-rs/tui/src/version.rs has a non-SemVer Buddy version: 'fresh'",
            ],
        )

    def test_version_bump_must_exceed_the_local_baseline(self) -> None:
        self.assertEqual(
            CHECK_BUDDY_VERSION.validate_version_bump("1.0.1", "1.0.1"),
            [
                "Codex Buddy version must increase for every completed task: "
                "baseline=1.0.1, current=1.0.1"
            ],
        )
        self.assertEqual(
            CHECK_BUDDY_VERSION.validate_version_bump("1.0.1", "1.0.2"), []
        )
