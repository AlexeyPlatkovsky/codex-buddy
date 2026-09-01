#!/usr/bin/env python3
"""Verify the Codex Buddy package, lockfile, and TUI identity use one version."""

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import sys
from typing import Callable


BUDDY_CARGO_TOML = Path("codex-rs/codex-buddy/Cargo.toml")
CARGO_LOCK = Path("codex-rs/Cargo.lock")
TUI_VERSION_RS = Path("codex-rs/tui/src/version.rs")
SEMVER_RE = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)


@dataclass(frozen=True)
class BuddyVersions:
    """Versions taken from every artifact that defines the Buddy release identity."""

    cargo_toml: str
    cargo_lock: str
    tui_display: str


def _extract_version(text: str, pattern: str, source: Path) -> str:
    match = re.search(pattern, text, flags=re.MULTILINE)
    if match is None:
        raise ValueError(f"could not find the Codex Buddy version in {source}")
    return match.group(1)


def versions_from_text(
    cargo_toml: str, cargo_lock: str, tui_version_rs: str
) -> BuddyVersions:
    """Parse Buddy's release version from the three version-bearing source files."""

    return BuddyVersions(
        cargo_toml=_extract_version(
            cargo_toml,
            r'^version = "([^"]+)"$',
            BUDDY_CARGO_TOML,
        ),
        cargo_lock=_extract_version(
            cargo_lock,
            r'(?ms)^\[\[package\]\]\nname = "codex-buddy"\nversion = "([^"]+)"$',
            CARGO_LOCK,
        ),
        tui_display=_extract_version(
            tui_version_rs,
            r'(?ms)^#\[cfg\(feature = "buddy-branding"\)\]\n'
            r'pub\(crate\) const PRODUCT_DISPLAY_VERSION: &str = "([^"]+)";$',
            TUI_VERSION_RS,
        ),
    )


def validate_versions(versions: BuddyVersions) -> list[str]:
    """Return every version policy violation so a release fix is one edit cycle."""

    errors = []
    for source, version in (
        (BUDDY_CARGO_TOML, versions.cargo_toml),
        (CARGO_LOCK, versions.cargo_lock),
        (TUI_VERSION_RS, versions.tui_display),
    ):
        if SEMVER_RE.fullmatch(version) is None:
            errors.append(f"{source} has a non-SemVer Buddy version: {version!r}")

    distinct_versions = {versions.cargo_toml, versions.cargo_lock, versions.tui_display}
    if len(distinct_versions) != 1:
        errors.append(
            "Codex Buddy versions must match: "
            f"Cargo.toml={versions.cargo_toml}, "
            f"Cargo.lock={versions.cargo_lock}, "
            f"TUI={versions.tui_display}"
        )
    return errors


def validate_version_bump(previous: str, current: str) -> list[str]:
    """Require the checked Buddy version to be newer than a local baseline."""

    previous_match = SEMVER_RE.fullmatch(previous)
    current_match = SEMVER_RE.fullmatch(current)
    if previous_match is None or current_match is None:
        return ["Codex Buddy versions must be valid SemVer before comparing them"]

    previous_core = tuple(
        map(
            int, previous.split("-", maxsplit=1)[0].split("+", maxsplit=1)[0].split(".")
        )
    )
    current_core = tuple(
        map(int, current.split("-", maxsplit=1)[0].split("+", maxsplit=1)[0].split("."))
    )
    if current_core <= previous_core:
        return [
            "Codex Buddy version must increase for every completed task: "
            f"baseline={previous}, current={current}"
        ]
    return []


def staged_reader(repo_root: Path) -> Callable[[Path], str]:
    """Return a reader for the Git index, which is what a commit actually records."""

    def read(path: Path) -> str:
        result = subprocess.run(
            ["git", "-C", str(repo_root), "show", f":{path.as_posix()}"],
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise ValueError(f"could not read staged {path}: {result.stderr.strip()}")
        return result.stdout

    return read


def working_tree_reader(repo_root: Path) -> Callable[[Path], str]:
    """Return a reader for CI and direct local validation."""

    return lambda path: (repo_root / path).read_text(encoding="utf-8")


def version_from_git_ref(repo_root: Path, git_ref: str) -> str:
    """Read the Codex Buddy package version from one local Git revision."""

    result = subprocess.run(
        ["git", "-C", str(repo_root), "show", f"{git_ref}:{BUDDY_CARGO_TOML}"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise ValueError(
            f"could not read {BUDDY_CARGO_TOML} from {git_ref}: {result.stderr.strip()}"
        )
    return _extract_version(result.stdout, r'^version = "([^"]+)"$', BUDDY_CARGO_TOML)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--staged",
        action="store_true",
        help="validate the Git index instead of the working tree",
    )
    parser.add_argument(
        "--require-bump-from-ref",
        metavar="REF",
        help="require the checked version to be newer than this local Git revision",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parents[2]
    read = staged_reader(repo_root) if args.staged else working_tree_reader(repo_root)
    try:
        versions = versions_from_text(
            read(BUDDY_CARGO_TOML),
            read(CARGO_LOCK),
            read(TUI_VERSION_RS),
        )
    except (OSError, ValueError) as error:
        print(f"Buddy version check failed: {error}", file=sys.stderr)
        return 1

    errors = validate_versions(versions)
    if args.require_bump_from_ref is not None:
        try:
            previous_version = version_from_git_ref(
                repo_root, args.require_bump_from_ref
            )
        except ValueError as error:
            print(f"Buddy version check failed: {error}", file=sys.stderr)
            return 1
        errors.extend(validate_version_bump(previous_version, versions.cargo_toml))
    if errors:
        print("Buddy version check failed:", file=sys.stderr)
        print("\n".join(f"- {error}" for error in errors), file=sys.stderr)
        return 1

    print(f"Codex Buddy version {versions.cargo_toml} is consistent.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
