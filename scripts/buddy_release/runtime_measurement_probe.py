#!/usr/bin/env python3
"""Collect repeatable process and PTY-first-frame evidence for a Buddy binary."""

import argparse
import base64
import hashlib
import json
import os
import pty
import re
import selectors
import signal
import struct
import subprocess
import sys
import time
from pathlib import Path


ANSI_SEQUENCE = re.compile(rb"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07]*(?:\x07|\x1b\\))")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--codex-home", type=Path, required=True)
    parser.add_argument("--warm-runs", type=int, default=5)
    parser.add_argument("--tui-timeout-seconds", type=float, default=8.0)
    return parser.parse_args()


def run_version(binary: Path, environment: dict[str, str]) -> dict[str, object]:
    started = time.perf_counter_ns()
    completed = subprocess.run(
        [str(binary), "--version"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
        env=environment,
        timeout=30,
    )
    return {
        "elapsed_ms": (time.perf_counter_ns() - started) / 1_000_000,
        "returncode": completed.returncode,
    }


def rss_kib(pid: int) -> int | None:
    completed = subprocess.run(
        ["ps", "-o", "rss=", "-p", str(pid)],
        capture_output=True,
        check=False,
        text=True,
    )
    if completed.returncode != 0:
        return None
    try:
        return int(completed.stdout.strip())
    except ValueError:
        return None


def descendant_processes(pid: int) -> list[dict[str, object]]:
    completed = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,comm="],
        capture_output=True,
        check=False,
        text=True,
    )
    if completed.returncode != 0:
        return []
    processes: list[tuple[int, int, str]] = []
    for line in completed.stdout.splitlines():
        fields = line.strip().split(maxsplit=2)
        if len(fields) != 3:
            continue
        try:
            processes.append((int(fields[0]), int(fields[1]), fields[2]))
        except ValueError:
            continue

    descendants: list[dict[str, object]] = []
    parents = {pid}
    while parents and len(descendants) < 64:
        children = [process for process in processes if process[1] in parents]
        parents = {child_pid for child_pid, _, _ in children}
        descendants.extend(
            {"pid": child_pid, "ppid": parent_pid, "command": command}
            for child_pid, parent_pid, command in children
        )
    return descendants[:64]


def wait_for_exit(pid: int, timeout_seconds: float) -> int | None:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        exited, status = os.waitpid(pid, os.WNOHANG)
        if exited == pid:
            return status
        time.sleep(0.02)
    return None


def terminate(pid: int) -> int | None:
    try:
        process_group = os.getpgid(pid)
    except ProcessLookupError:
        return None
    signal_target = -process_group if process_group == pid else pid
    try:
        os.kill(signal_target, signal.SIGINT)
    except ProcessLookupError:
        return None
    status = wait_for_exit(pid, 1.0)
    if status is not None:
        return status
    try:
        os.kill(signal_target, signal.SIGTERM)
    except ProcessLookupError:
        return None
    status = wait_for_exit(pid, 1.0)
    if status is not None:
        return status
    try:
        os.kill(signal_target, signal.SIGKILL)
    except ProcessLookupError:
        return None
    return wait_for_exit(pid, 1.0)


def frame_evidence(captured: bytes) -> tuple[bool, int]:
    rendered_text = ANSI_SEQUENCE.sub(b"", captured)
    rendered_text = bytes(
        byte for byte in rendered_text if byte in (9, 10, 13) or 32 <= byte <= 126
    )
    has_screen_control = any(
        marker in captured
        for marker in (b"\x1b[?1049h", b"\x1b[2J", b"\x1b[H", b"\x1b[1;1H")
    )
    return has_screen_control, len(rendered_text.strip())


def tui_first_frame(
    binary: Path, timeout_seconds: float, environment: dict[str, str]
) -> dict[str, object]:
    pid, master_fd = pty.fork()
    if pid == 0:
        os.environ.clear()
        os.environ.update(environment)
        os.environ["TERM"] = "xterm-256color"
        os.execv(str(binary), [str(binary)])

    captured = bytearray()
    first_output_ns: int | None = None
    first_verified_frame_ns: int | None = None
    first_frame_rss_kib: int | None = None
    descendants_at_first_output: list[dict[str, object]] = []
    started = time.perf_counter_ns()
    selector = selectors.DefaultSelector()
    selector.register(master_fd, selectors.EVENT_READ)
    try:
        try:
            import fcntl
            import termios

            fcntl.ioctl(
                master_fd, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0)
            )
        except OSError:
            pass

        deadline = time.monotonic() + timeout_seconds
        while time.monotonic() < deadline:
            events = selector.select(timeout=min(0.1, deadline - time.monotonic()))
            if not events:
                continue
            try:
                data = os.read(master_fd, 4096 - len(captured))
            except OSError:
                break
            if not data:
                break
            if first_output_ns is None:
                first_output_ns = time.perf_counter_ns()
                first_frame_rss_kib = rss_kib(pid)
                descendants_at_first_output = descendant_processes(pid)
            captured.extend(data)
            if first_verified_frame_ns is None:
                has_screen_control, rendered_text_bytes = frame_evidence(
                    bytes(captured)
                )
                if has_screen_control and rendered_text_bytes >= 20:
                    first_verified_frame_ns = time.perf_counter_ns()
            if len(captured) == 4096:
                break
    finally:
        selector.close()
        descendants_before_exit = descendant_processes(pid)
        exit_status = terminate(pid)
        os.close(master_fd)

    has_ansi_screen_control, substantive_text_bytes = frame_evidence(bytes(captured))

    result: dict[str, object] = {
        "pty": True,
        "timeout_seconds": timeout_seconds,
        "captured_bytes": len(captured),
        "capture_sha256": hashlib.sha256(captured).hexdigest(),
        "capture_prefix_base64": base64.b64encode(captured[:128]).decode("ascii"),
        "exit_status": exit_status,
        "first_output_elapsed_ms": None,
        "first_verified_frame_elapsed_ms": None,
        "has_ansi_screen_control": has_ansi_screen_control,
        "substantive_rendered_text_bytes": substantive_text_bytes,
        "frame_verified": False,
        "frame_verification_limitation": (
            "A frame requires both ANSI screen control and at least 20 bytes of rendered text."
        ),
        "rss_kib_at_first_output": first_frame_rss_kib,
        "idle_memory_kib": None,
        "idle_memory_limitation": (
            "No stable idle-ready signal is available locally; RSS is sampled at first PTY output "
            "and is not reported as idle memory."
        ),
        "runtime_process_observation": {
            "descendants_at_first_output": descendants_at_first_output,
            "descendants_before_exit": descendants_before_exit,
            "cap": 64,
            "limitation": (
                "Two bounded child-process snapshots are a lower bound; transient, reparented, "
                "and in-process services plus a model first-turn scenario require separate "
                "instrumentation."
            ),
        },
    }
    if first_output_ns is not None:
        result["first_output_elapsed_ms"] = (first_output_ns - started) / 1_000_000
    if first_verified_frame_ns is not None:
        result["frame_verified"] = True
        result["frame_verification_limitation"] = None
        result["first_verified_frame_elapsed_ms"] = (
            first_verified_frame_ns - started
        ) / 1_000_000
    return result


def main() -> None:
    args = parse_args()
    binary = args.binary.resolve()
    codex_home = args.codex_home.resolve()
    if not binary.is_file() or binary.is_symlink():
        raise SystemExit(f"refusing probe: binary is not a regular file: {binary}")
    if not codex_home.is_dir() or codex_home.is_symlink():
        raise SystemExit(
            f"refusing probe: CODEX_HOME is not a regular directory: {codex_home}"
        )
    if args.warm_runs < 1 or args.tui_timeout_seconds <= 0:
        raise SystemExit("warm runs and TUI timeout must be positive")

    environment = os.environ.copy()
    environment["CODEX_HOME"] = str(codex_home)
    for credential_variable in ("OPENAI_API_KEY", "CODEX_API_KEY"):
        environment.pop(credential_variable, None)
    first_process_launch = run_version(binary, environment)
    warm_runs = [run_version(binary, environment) for _ in range(args.warm_runs)]
    print(
        json.dumps(
            {
                "process_startup": {
                    "command": [str(binary), "--version"],
                    "scope": "parser/process launch only; it does not establish a TUI frame",
                    "true_cold_start_ms": None,
                    "cold_start_limitation": (
                        "No privileged OS cache eviction was used; the first measured launch is "
                        "reported separately but is not labeled cold."
                    ),
                    "first_measured_process_launch": first_process_launch,
                    "warm_cache_process_runs": warm_runs,
                },
                "probe_environment": {
                    "codex_home": str(codex_home),
                    "credential_variables_removed": ["OPENAI_API_KEY", "CODEX_API_KEY"],
                },
                "tui_first_frame": tui_first_frame(
                    binary, args.tui_timeout_seconds, environment
                ),
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
