import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPT_PATH = Path(__file__).with_name("check_buddy_version.py")
SPEC = importlib.util.spec_from_file_location("check_buddy_version", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
CHECK_BUDDY_VERSION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK_BUDDY_VERSION)


class CheckBuddyVersionTests(unittest.TestCase):
    def test_cli_rejects_a_pin_that_differs_from_the_actual_codex_version(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            script = root / "scripts/buddy_release/check_buddy_version.py"
            script.parent.mkdir(parents=True)
            shutil.copyfile(SCRIPT_PATH, script)
            files = {
                "scripts/buddy_release/upstream-version.txt": "0.157.1\n",
                "codex-rs/Cargo.toml": '[workspace.package]\nversion = "0.158.0"\n',
                "codex-rs/codex-buddy/Cargo.toml": 'version = "1.157.1"\n',
                "codex-rs/Cargo.lock": (
                    '[[package]]\nname = "codex-buddy"\nversion = "1.157.1"\n'
                ),
                "codex-rs/tui/src/version.rs": (
                    '#[cfg(feature = "buddy-branding")]\n'
                    'pub(crate) const PRODUCT_DISPLAY_VERSION: &str = "1.157.1";\n'
                ),
            }
            for name, contents in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(contents, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(script)],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 1)
            self.assertIn(
                "Pinned upstream version 0.157.1 differs from Codex workspace version 0.158.0",
                result.stderr,
            )

    def test_upstream_release_mapping(self) -> None:
        for upstream, buddy in (
            ("0.157.1", "1.157.1"),
            ("0.158.0", "1.158.0"),
        ):
            with self.subTest(upstream=upstream):
                self.assertEqual(
                    CHECK_BUDDY_VERSION.validate_upstream_version(buddy, upstream), []
                )

    def test_unrelated_buddy_release_is_rejected(self) -> None:
        for buddy in ("1.157.2", "1.156.1", "2.157.1"):
            with self.subTest(buddy=buddy):
                self.assertEqual(
                    CHECK_BUDDY_VERSION.validate_upstream_version(buddy, "0.157.1"),
                    [
                        f"Codex Buddy must track upstream 0.157.1 as 1.157.1, got {buddy}"
                    ],
                )

    def test_invalid_upstream_release_is_rejected(self) -> None:
        for upstream in ("0.157.1-alpha.1", "1.157.1", "latest"):
            with self.subTest(upstream=upstream):
                self.assertEqual(
                    CHECK_BUDDY_VERSION.validate_upstream_version("1.157.1", upstream),
                    ["Upstream version must be a stable 0.x.x release"],
                )

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
                "Codex Buddy version must increase for an upstream release sync: "
                "baseline=1.0.1, current=1.0.1"
            ],
        )
        self.assertEqual(
            CHECK_BUDDY_VERSION.validate_version_bump("1.0.1", "1.0.2"), []
        )
