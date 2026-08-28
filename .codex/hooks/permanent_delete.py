import json
import os
import shlex
import shutil
import sys
from pathlib import Path
from typing import NoReturn


class PermanentDeleteError(ValueError):
    pass


def _deny(reason: str) -> None:
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "deny",
                    "permissionDecisionReason": reason,
                }
            }
        )
    )


def _allow_rewrite(command: str) -> None:
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "updatedInput": {"command": command},
                }
            }
        )
    )


def _tokens(command: str) -> list[str]:
    lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|<>()")
    lexer.commenters = ""
    lexer.whitespace_split = True
    try:
        return list(lexer)
    except ValueError as error:
        raise PermanentDeleteError(
            f"cannot safely parse Trash command: {error}"
        ) from error


def rewrite_trash_command(
    command: str,
    *,
    interpreter: str,
    script_path: Path,
) -> str | None:
    tokens = _tokens(command)
    if not tokens:
        return None

    shell_operators = {";", ";;", "&", "&&", "|", "||", "<", ">", "<<", ">>", "(", ")"}
    if Path(tokens[0]).name != "trash":
        command_starts = {0}
        command_starts.update(
            index + 1
            for index, token in enumerate(tokens[:-1])
            if token in shell_operators
        )
        wrapped_trash = any(
            Path(tokens[index]).name == "trash"
            or (
                tokens[index] in {"command", "sudo"}
                and index + 1 < len(tokens)
                and Path(tokens[index + 1]).name == "trash"
            )
            for index in command_starts
        )
        if wrapped_trash:
            raise PermanentDeleteError(
                "wrapped or compound Trash commands are blocked; "
                "run the deletion as a standalone command"
            )
        return None

    if any(token in shell_operators for token in tokens[1:]):
        raise PermanentDeleteError(
            "Trash commands combined with other shell operations are blocked; "
            "run the deletion as a separate command"
        )

    targets = tokens[1:]
    if targets[:1] == ["--"]:
        targets = targets[1:]
    if not targets:
        raise PermanentDeleteError("Trash command has no deletion target")
    if any(target.startswith("-") for target in targets):
        raise PermanentDeleteError(
            "Trash command options are not rewritten; use explicit workspace paths"
        )
    if any(any(character in target for character in "$*?[]{}") for target in targets):
        raise PermanentDeleteError(
            "dynamic Trash paths are not rewritten; expand them to explicit workspace paths"
        )

    return shlex.join([interpreter, str(script_path), "--delete", "--", *targets])


def _workspace_entry(raw_target: str, *, cwd: Path, workspace_root: Path) -> Path:
    expanded = Path(os.path.expanduser(raw_target))
    target = expanded if expanded.is_absolute() else cwd / expanded
    candidate = target.parent.resolve() / target.name
    root = workspace_root.resolve()
    if candidate == root:
        raise PermanentDeleteError("refusing to delete the workspace root")
    try:
        candidate.relative_to(root)
    except ValueError as error:
        raise PermanentDeleteError(
            f"refusing to delete outside the workspace: {raw_target}"
        ) from error
    return candidate


def delete_targets(targets: list[str], *, cwd: Path, workspace_root: Path) -> None:
    if not targets:
        raise PermanentDeleteError("no deletion targets were provided")

    entries = [
        _workspace_entry(target, cwd=cwd, workspace_root=workspace_root)
        for target in targets
    ]
    missing = [
        str(entry) for entry in entries if not entry.exists() and not entry.is_symlink()
    ]
    if missing:
        raise PermanentDeleteError(f"deletion target does not exist: {missing[0]}")

    for entry in entries:
        if entry.is_symlink() or entry.is_file():
            entry.unlink()
        elif entry.is_dir():
            shutil.rmtree(entry)
        else:
            raise PermanentDeleteError(f"unsupported deletion target: {entry}")
        print(f"permanently deleted: {entry}")


def _fail(message: str) -> NoReturn:
    print(message, file=sys.stderr)
    raise SystemExit(2)


def main() -> None:
    script_path = Path(__file__).resolve()
    workspace_root = script_path.parents[2]
    if sys.argv[1:2] == ["--delete"]:
        targets = sys.argv[2:]
        if targets[:1] == ["--"]:
            targets = targets[1:]
        try:
            delete_targets(targets, cwd=Path.cwd(), workspace_root=workspace_root)
        except PermanentDeleteError as error:
            _fail(str(error))
        return

    try:
        event = json.load(sys.stdin)
    except (json.JSONDecodeError, OSError) as error:
        _fail(f"failed to read PreToolUse input: {error}")

    if event.get("tool_name") != "Bash":
        return
    command = event.get("tool_input", {}).get("command")
    if not isinstance(command, str):
        _deny("shell hook received no command to inspect")
        return

    try:
        rewritten = rewrite_trash_command(
            command,
            interpreter=sys.executable,
            script_path=script_path,
        )
    except PermanentDeleteError as error:
        _deny(str(error))
        return
    if rewritten is not None:
        _allow_rewrite(rewritten)


if __name__ == "__main__":
    main()
