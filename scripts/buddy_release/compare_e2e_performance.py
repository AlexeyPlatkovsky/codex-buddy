#!/usr/bin/env python3
"""Compare deterministic Codex Buddy startup and first-turn performance.

The controller intentionally lives in the current checkout: it builds the requested revisions in
isolated worktrees and drives them against one local, deterministic Responses SSE server. This
keeps external model latency, credentials, and network variability out of the release evidence.
"""

import argparse
import base64
import fcntl
import hashlib
import http.server
import json
import math
import os
import platform
import pty
import random
import selectors
import shutil
import signal
import socket
import statistics
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TypeVar

from typing_extensions import Self

SCHEMA_VERSION = 1
CAPTURE_LIMIT_BYTES = 64 * 1024
DEFAULT_SCENARIOS = (
    "parser_startup",
    "tui_startup",
    "headless_first_turn",
    "interactive_first_turn",
)
PROMPT = "Reply with exactly BENCHMARK_REPLY."
RESPONSE_TEXT = "BENCHMARK_REPLY"
ResponsesServerType = TypeVar("ResponsesServerType", bound="ResponsesServer")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default="92529d95fd")
    parser.add_argument(
        "--baseline-binary",
        type=Path,
        help="Use an existing executable as the baseline instead of building --baseline.",
    )
    parser.add_argument("--current", default="HEAD")
    parser.add_argument(
        "--current-model-instructions-file",
        type=Path,
        help="Replace Buddy's root instructions with this file for the current samples only.",
    )
    parser.add_argument("--target")
    parser.add_argument("--warmups", type=int, default=5)
    parser.add_argument("--warm-pairs", type=int, default=30)
    parser.add_argument("--cold-pairs", type=int, default=10)
    parser.add_argument("--seed", type=int, default=20_260_829)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--minimum-free-gib", type=float, default=40.0)
    parser.add_argument("--scenario", action="append", choices=DEFAULT_SCENARIOS)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--enable-cold-cache-eviction",
        action="store_true",
        help="Allow Linux root-only page-cache eviction before cold samples.",
    )
    parser.add_argument(
        "--require-ideal-host",
        action="store_true",
        help="Reject hosts without the Linux controls needed for release evidence.",
    )
    return parser.parse_args()


def command_output(command: list[str], *, cwd: Path | None = None) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def require_regular_directory(path: Path, description: str) -> None:
    if not path.is_dir() or path.is_symlink():
        raise RuntimeError(f"{description} must be a regular directory: {path}")


def require_regular_file(path: Path, description: str) -> None:
    if not path.is_file() or path.is_symlink():
        raise RuntimeError(f"{description} must be a regular file: {path}")


def resolve_revision(repo_root: Path, revision: str) -> str:
    return command_output(
        ["git", "rev-parse", "--verify", f"{revision}^{{commit}}"], cwd=repo_root
    )


def rust_host_target() -> str:
    for line in command_output(["rustc", "-vV"]).splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    raise RuntimeError("rustc -vV did not report a host target")


def active_build_processes() -> list[str]:
    processes: list[str] = []
    for name in ("cargo", "cargo-nextest", "rustc", "just", "bazel", "bazelisk"):
        completed = subprocess.run(
            ["pgrep", "-x", name],
            capture_output=True,
            check=False,
            text=True,
        )
        if completed.returncode == 0:
            processes.append(
                f"{name}: {completed.stdout.strip().replace(chr(10), ',')}"
            )
    return processes


def free_bytes(path: Path) -> int:
    return shutil.disk_usage(path).free


def cpu_governors() -> list[str]:
    governors = sorted(
        Path("/sys/devices/system/cpu").glob("cpu[0-9]*/cpufreq/scaling_governor")
    )
    return sorted(
        {governor.read_text().strip() for governor in governors if governor.is_file()}
    )


def host_controls() -> dict[str, Any]:
    load_average = os.getloadavg()[0] if hasattr(os, "getloadavg") else None
    governors = cpu_governors()
    linux = sys.platform.startswith("linux")
    return {
        "platform": platform.platform(),
        "python": sys.version,
        "cpu_count": os.cpu_count(),
        "load_average_1m": load_average,
        "cpu_governors": governors,
        "effective_uid": os.geteuid() if hasattr(os, "geteuid") else None,
        "ideal_release_host": linux
        and bool(governors)
        and governors == ["performance"]
        and load_average is not None
        and load_average <= max(1.0, (os.cpu_count() or 1) * 0.25),
        "limitations": (
            "A dedicated runner, fixed power mode, and an idle system are operational controls; "
            "this script records available host evidence but cannot establish exclusive ownership."
        ),
    }


def loopback_available() -> bool:
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
            listener.bind(("127.0.0.1", 0))
    except OSError:
        return False
    return True


def cold_cache_status(enabled: bool) -> dict[str, Any]:
    cache_path = Path("/proc/sys/vm/drop_caches")
    available = (
        enabled
        and sys.platform.startswith("linux")
        and hasattr(os, "geteuid")
        and os.geteuid() == 0
        and cache_path.is_file()
        and os.access(cache_path, os.W_OK)
    )
    if available:
        reason = None
    elif not enabled:
        reason = "requires explicit --enable-cold-cache-eviction"
    else:
        reason = "requires a Linux root runner with writable /proc/sys/vm/drop_caches"
    return {"available": available, "reason_unavailable": reason}


def evict_page_cache() -> None:
    subprocess.run(["sync"], check=True)
    Path("/proc/sys/vm/drop_caches").write_text("3\n")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def bounded_append(buffer: bytearray, chunk: bytes) -> None:
    remaining = CAPTURE_LIMIT_BYTES - len(buffer)
    if remaining > 0:
        buffer.extend(chunk[:remaining])


def decoded_capture(buffer: bytearray) -> dict[str, Any]:
    captured = bytes(buffer)
    return {
        "bytes_captured": len(captured),
        "truncated": len(captured) == CAPTURE_LIMIT_BYTES,
        "sha256": sha256_bytes(captured),
        "prefix_base64": base64.b64encode(captured[:512]).decode("ascii"),
    }


def approximate_tokens(value: bytes) -> int:
    """Return a deterministic payload-size estimate, not model tokenizer usage."""
    return math.ceil(len(value) / 4)


def request_payload_summary(request_body: bytes) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "body_bytes": len(request_body),
        "approximate_tokens": approximate_tokens(request_body),
        "body_sha256": sha256_bytes(request_body),
    }
    try:
        payload = json.loads(request_body)
    except json.JSONDecodeError:
        summary["json"] = "invalid"
        return summary
    if not isinstance(payload, dict):
        summary["json"] = type(payload).__name__
        return summary
    sections: dict[str, Any] = {}
    for key, value in payload.items():
        encoded = json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()
        sections[key] = {
            "json_bytes": len(encoded),
            "approximate_tokens": approximate_tokens(encoded),
            "sha256": sha256_bytes(encoded),
        }
    summary["json"] = "object"
    summary["sections"] = sections
    return summary


def sse_fixture() -> bytes:
    events = (
        {
            "type": "response.created",
            "response": {"id": "benchmark-response"},
        },
        {
            "type": "response.output_item.done",
            "item": {
                "type": "message",
                "role": "assistant",
                "id": "benchmark-message",
                "content": [{"type": "output_text", "text": RESPONSE_TEXT}],
            },
        },
        {
            "type": "response.completed",
            "response": {
                "id": "benchmark-response",
                "usage": {
                    "input_tokens": 0,
                    "input_tokens_details": None,
                    "output_tokens": 0,
                    "output_tokens_details": None,
                    "total_tokens": 0,
                },
            },
        },
    )
    return b"".join(
        f"event: {event['type']}\ndata: {json.dumps(event, separators=(',', ':'))}\n\n".encode()
        for event in events
    )


class ResponsesServer:
    def __init__(self) -> None:
        self.requests: list[dict[str, Any]] = []
        self.first_sse_ns: int | None = None
        self.completed_sse_ns: int | None = None
        self._lock = threading.Lock()
        fixture = sse_fixture()

        recorder = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_POST(self) -> None:
                content_length = int(self.headers.get("content-length", "0"))
                request_body = self.rfile.read(content_length)
                with recorder._lock:
                    recorder.requests.append(
                        {
                            "arrival_ns": time.perf_counter_ns(),
                            "path": self.path,
                            "payload": request_payload_summary(request_body),
                        }
                    )
                self.send_response(200)
                self.send_header("content-type", "text/event-stream")
                self.send_header("cache-control", "no-cache")
                self.send_header("content-length", str(len(fixture)))
                self.end_headers()
                with recorder._lock:
                    recorder.first_sse_ns = time.perf_counter_ns()
                self.wfile.write(fixture)
                self.wfile.flush()
                with recorder._lock:
                    recorder.completed_sse_ns = time.perf_counter_ns()

            def log_message(self, _format: str, *_args: object) -> None:
                return

        self._server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)

    @property
    def base_url(self) -> str:
        host, port = self._server.server_address
        return f"http://{host}:{port}"

    def __enter__(self) -> Self:
        self._thread.start()
        return self

    def __exit__(self, _type: object, _value: object, _traceback: object) -> None:
        self._server.shutdown()
        self._server.server_close()
        self._thread.join(timeout=5)

    def evidence(self) -> dict[str, Any]:
        with self._lock:
            return {
                "request_count": len(self.requests),
                "requests": list(self.requests),
                "first_sse_ns": self.first_sse_ns,
                "completed_sse_ns": self.completed_sse_ns,
                "fixture_sha256": sha256_bytes(sse_fixture()),
            }


def isolated_environment(codex_home: Path) -> dict[str, str]:
    environment = os.environ.copy()
    for variable in (
        "OPENAI_API_KEY",
        "CODEX_API_KEY",
        "CODEX_ACCESS_TOKEN",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ):
        environment.pop(variable, None)
    environment["CODEX_HOME"] = str(codex_home)
    environment["NO_PROXY"] = "127.0.0.1,localhost"
    environment["no_proxy"] = "127.0.0.1,localhost"
    environment["TERM"] = "xterm-256color"
    return environment


def write_config(
    codex_home: Path,
    server_url: str,
    model_instructions_file: Path | None = None,
) -> None:
    codex_home.mkdir(mode=0o700)
    model_instructions_config = ""
    if model_instructions_file is not None:
        require_regular_file(model_instructions_file, "model instructions file")
        copied_instructions = codex_home / "model_instructions.md"
        shutil.copyfile(model_instructions_file, copied_instructions)
        copied_instructions.chmod(0o600)
        model_instructions_config = (
            f'model_instructions_file = "{copied_instructions}"\n'
        )
    config = f'''model = "benchmark-model"
model_provider = "benchmark"
approval_policy = "never"
sandbox_mode = "read-only"
analytics.enabled = false
{model_instructions_config}

[model_providers.benchmark]
name = "Deterministic benchmark Responses provider"
base_url = "{server_url}/v1"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
'''
    (codex_home / "config.toml").write_text(config)


def process_rss_kib(pid: int) -> int | None:
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


def terminate_process_group(pid: int) -> None:
    try:
        process_group = os.getpgid(pid)
    except ProcessLookupError:
        return
    target = -process_group if process_group == pid else pid
    for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGKILL):
        try:
            os.kill(target, sig)
        except ProcessLookupError:
            return
        deadline = time.monotonic() + 1
        while time.monotonic() < deadline:
            try:
                exited, _status = os.waitpid(pid, os.WNOHANG)
            except ChildProcessError:
                return
            if exited == pid:
                return
            time.sleep(0.02)


def command_measurement(
    command: list[str],
    environment: dict[str, str],
    cwd: Path,
    server: ResponsesServer | None,
    timeout_seconds: float,
    marker: bytes | None,
) -> dict[str, Any]:
    started_ns = time.perf_counter_ns()
    process = subprocess.Popen(
        command,
        cwd=cwd,
        env=environment,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    stdout = bytearray()
    stderr = bytearray()
    first_marker_ns: int | None = None
    peak_rss_kib: int | None = None
    timed_out = False
    selector = selectors.DefaultSelector()
    assert process.stdout is not None
    assert process.stderr is not None
    selector.register(process.stdout, selectors.EVENT_READ, "stdout")
    selector.register(process.stderr, selectors.EVENT_READ, "stderr")
    try:
        while process.poll() is None:
            for key, _events in selector.select(timeout=0.05):
                chunk = os.read(key.fileobj.fileno(), 4096)
                if key.data == "stdout":
                    bounded_append(stdout, chunk)
                    if (
                        marker is not None
                        and first_marker_ns is None
                        and marker in stdout
                    ):
                        first_marker_ns = time.perf_counter_ns()
                else:
                    bounded_append(stderr, chunk)
            rss = process_rss_kib(process.pid)
            if rss is not None:
                peak_rss_kib = max(peak_rss_kib or rss, rss)
            if time.perf_counter_ns() - started_ns > timeout_seconds * 1_000_000_000:
                timed_out = True
                process.terminate()
                break
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        timed_out = True
        terminate_process_group(process.pid)
    finally:
        selector.close()
    for stream, buffer in ((process.stdout, stdout), (process.stderr, stderr)):
        if stream is not None:
            remainder = stream.read()
            if remainder:
                bounded_append(buffer, remainder)
    evidence = server.evidence() if server is not None else {"request_count": 0}
    success = (
        not timed_out
        and process.returncode == 0
        and (marker is None or marker in stdout)
        and (server is None or evidence["request_count"] == 1)
    )
    return {
        "success": success,
        "returncode": process.returncode,
        "timed_out": timed_out,
        "metrics_ms": {
            "completion_elapsed_ms": (time.perf_counter_ns() - started_ns) / 1_000_000,
            "request_elapsed_ms": elapsed_ms(evidence.get("requests", []), started_ns),
            "first_sse_elapsed_ms": elapsed_ms(
                evidence.get("first_sse_ns"), started_ns
            ),
            "assistant_output_elapsed_ms": elapsed_ms(first_marker_ns, started_ns),
        },
        "peak_root_rss_kib": peak_rss_kib,
        "stdout": decoded_capture(stdout),
        "stderr": decoded_capture(stderr),
        "server": evidence,
    }


def elapsed_ms(
    timestamp: int | list[dict[str, Any]] | None, started_ns: int
) -> float | None:
    if isinstance(timestamp, list):
        timestamp = timestamp[0]["arrival_ns"] if timestamp else None
    return None if timestamp is None else (timestamp - started_ns) / 1_000_000


def frame_evidence(captured: bytes) -> bool:
    controls = (b"\x1b[?1049h", b"\x1b[2J", b"\x1b[H", b"\x1b[1;1H")
    rendered = bytes(
        byte for byte in captured if byte in (9, 10, 13) or 32 <= byte <= 126
    )
    return (
        any(control in captured for control in controls) and len(rendered.strip()) >= 20
    )


def tui_measurement(
    command: list[str],
    environment: dict[str, str],
    cwd: Path,
    server: ResponsesServer,
    timeout_seconds: float,
    turn_input: bytes | None,
) -> dict[str, Any]:
    started_ns = time.perf_counter_ns()
    pid, master_fd = pty.fork()
    if pid == 0:
        os.chdir(cwd)
        os.environ.clear()
        os.environ.update(environment)
        os.execve(command[0], command, environment)

    fcntl.ioctl(master_fd, termios.TIOCSWINSZ, struct.pack("HHHH", 48, 160, 0, 0))

    captured = bytearray()
    first_frame_ns: int | None = None
    first_marker_ns: int | None = None
    peak_rss_kib: int | None = None
    timed_out = False
    submitted_turn = False
    selector = selectors.DefaultSelector()
    selector.register(master_fd, selectors.EVENT_READ)
    exit_status: int | None = None
    try:
        deadline = time.monotonic() + timeout_seconds
        while time.monotonic() < deadline:
            events = selector.select(timeout=0.05)
            for _key, _events in events:
                try:
                    chunk = os.read(master_fd, 4096)
                except OSError:
                    chunk = b""
                bounded_append(captured, chunk)
            if first_frame_ns is None and frame_evidence(bytes(captured)):
                first_frame_ns = time.perf_counter_ns()
            if (
                first_frame_ns is not None
                and turn_input is not None
                and not submitted_turn
            ):
                os.write(master_fd, turn_input)
                submitted_turn = True
            if (
                turn_input is not None
                and first_marker_ns is None
                and RESPONSE_TEXT.encode() in captured
            ):
                first_marker_ns = time.perf_counter_ns()
            rss = process_rss_kib(pid)
            if rss is not None:
                peak_rss_kib = max(peak_rss_kib or rss, rss)
            evidence = server.evidence()
            ready = first_frame_ns is not None and (
                turn_input is None
                or (
                    submitted_turn
                    and first_marker_ns is not None
                    and evidence["request_count"] == 1
                    and evidence["completed_sse_ns"] is not None
                )
            )
            if ready:
                break
            exited, exit_status = os.waitpid(pid, os.WNOHANG)
            if exited == pid:
                break
        else:
            timed_out = True
    finally:
        selector.close()
        terminate_process_group(pid)
        os.close(master_fd)
    evidence = server.evidence()
    success = (
        not timed_out
        and first_frame_ns is not None
        and (
            turn_input is None
            or (
                submitted_turn
                and first_marker_ns is not None
                and evidence["request_count"] == 1
                and evidence["completed_sse_ns"] is not None
            )
        )
    )
    return {
        "success": success,
        "returncode": exit_status,
        "timed_out": timed_out,
        "metrics_ms": {
            "first_frame_elapsed_ms": elapsed_ms(first_frame_ns, started_ns),
            "request_elapsed_ms": elapsed_ms(evidence.get("requests", []), started_ns),
            "first_sse_elapsed_ms": elapsed_ms(
                evidence.get("first_sse_ns"), started_ns
            ),
            "assistant_output_elapsed_ms": elapsed_ms(first_marker_ns, started_ns),
        },
        "peak_root_rss_kib": peak_rss_kib,
        "terminal": decoded_capture(captured),
        "server": evidence,
    }


@dataclass(frozen=True)
class RevisionBinary:
    label: str
    revision: str
    binary: Path
    sha256: str


def build_revision(
    repo_root: Path,
    measurement_root: Path,
    label: str,
    revision: str,
    target: str,
) -> RevisionBinary:
    worktree = measurement_root / f"{label}-worktree"
    target_dir = measurement_root / f"{label}-target"
    subprocess.run(
        ["git", "worktree", "add", "--detach", str(worktree), revision],
        cwd=repo_root,
        check=True,
    )
    require_regular_directory(worktree, f"{label} worktree")
    build_environment = os.environ.copy()
    build_environment["CARGO_INCREMENTAL"] = "0"
    build_environment["CARGO_TARGET_DIR"] = str(target_dir)
    subprocess.run(
        [
            "cargo",
            "build",
            "--locked",
            "--release",
            "-p",
            "codex-buddy",
            "--target",
            target,
        ],
        cwd=worktree / "codex-rs",
        env=build_environment,
        check=True,
    )
    suffix = ".exe" if "windows" in target else ""
    binary = target_dir / target / "release" / f"codex-buddy{suffix}"
    require_regular_file(binary, f"{label} release binary")
    return RevisionBinary(
        label=label,
        revision=revision,
        binary=binary,
        sha256=sha256_file(binary),
    )


def sample(
    revision: RevisionBinary,
    scenario: str,
    root: Path,
    ordinal: int,
    timeout_seconds: float,
    model_instructions_file: Path | None,
) -> dict[str, Any]:
    sample_root = root / f"sample-{ordinal:04d}-{revision.label}-{scenario}"
    sample_root.mkdir(mode=0o700)
    codex_home = sample_root / "codex-home"
    cwd = sample_root / "workspace"
    cwd.mkdir(mode=0o700)
    environment = isolated_environment(codex_home)
    if scenario == "parser_startup":
        result = command_measurement(
            [str(revision.binary), "--version"],
            environment,
            cwd,
            None,
            timeout_seconds,
            None,
        )
    else:
        with ResponsesServer() as server:
            write_config(codex_home, server.base_url, model_instructions_file)
            if scenario == "headless_first_turn":
                command = [
                    str(revision.binary),
                    "exec",
                    "--skip-git-repo-check",
                    "--color",
                    "never",
                    PROMPT,
                ]
                result = command_measurement(
                    command,
                    environment,
                    cwd,
                    server,
                    timeout_seconds,
                    RESPONSE_TEXT.encode(),
                )
            elif scenario == "tui_startup":
                result = tui_measurement(
                    [str(revision.binary)],
                    environment,
                    cwd,
                    server,
                    timeout_seconds,
                    None,
                )
            elif scenario == "interactive_first_turn":
                result = tui_measurement(
                    [str(revision.binary)],
                    environment,
                    cwd,
                    server,
                    timeout_seconds,
                    f"{PROMPT}\r".encode(),
                )
            else:
                raise ValueError(f"unknown scenario: {scenario}")
    return {
        "revision_label": revision.label,
        "revision": revision.revision,
        "scenario": scenario,
        "sample_root": str(sample_root),
        **result,
    }


def balanced_order(seed: int, pairs: int) -> list[tuple[str, str]]:
    randomizer = random.Random(seed)
    return [
        ("baseline", "current")
        if randomizer.randrange(2) == 0
        else ("current", "baseline")
        for _ in range(pairs)
    ]


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        raise ValueError("cannot calculate a percentile without values")
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (position - lower)


def bootstrap_interval(
    values: list[float], seed: int, resamples: int = 2_000
) -> tuple[float, float]:
    randomizer = random.Random(seed)
    means = sorted(
        statistics.fmean(randomizer.choice(values) for _ in values)
        for _ in range(resamples)
    )
    return percentile(means, 0.025), percentile(means, 0.975)


def paired_summary(
    samples: list[dict[str, Any]],
    scenario: str,
    metric: str,
    phase: str,
    seed: int,
) -> dict[str, Any]:
    paired: dict[int, dict[str, float]] = {}
    for sample_result in samples:
        if (
            sample_result["scenario"] != scenario
            or sample_result["phase"] != phase
            or not sample_result["success"]
        ):
            continue
        value = sample_result["metrics_ms"].get(metric)
        if value is not None:
            paired.setdefault(sample_result["pair"], {})[
                sample_result["revision_label"]
            ] = value
    deltas = [
        ((entry["current"] - entry["baseline"]) / entry["baseline"]) * 100
        for entry in paired.values()
        if set(entry) == {"baseline", "current"} and entry["baseline"] > 0
    ]
    if not deltas:
        return {"status": "inconclusive", "paired_samples": 0}
    confidence_low, confidence_high = bootstrap_interval(deltas, seed)
    status = (
        "improved"
        if confidence_high < 0
        else "non_regressing"
        if confidence_high <= 5
        else "inconclusive"
    )
    return {
        "status": status,
        "paired_samples": len(deltas),
        "median_delta_percent": statistics.median(deltas),
        "p90_delta_percent": percentile(deltas, 0.9),
        "p95_delta_percent": percentile(deltas, 0.95),
        "mad_delta_percent": statistics.median(
            abs(delta - statistics.median(deltas)) for delta in deltas
        ),
        "bootstrap_95_percent_ci": [confidence_low, confidence_high],
        "acceptance": "upper confidence bound must be <= 5% for non-regression",
    }


def cleanup_measurement_root(repo_root: Path, root: Path, delete_guard: Path) -> bool:
    if root.parent != repo_root or not root.name.startswith(".buddy_e2e_performance."):
        raise RuntimeError(f"refusing cleanup outside the measurement root: {root}")
    if root.exists() and not root.is_symlink():
        subprocess.run(
            [sys.executable, str(delete_guard), "--delete", "--", str(root)],
            cwd=repo_root,
            check=True,
        )
    subprocess.run(["git", "worktree", "prune"], cwd=repo_root, check=True)
    return not root.exists()


def write_report(report: dict[str, Any], output: Path | None) -> None:
    encoded = json.dumps(report, indent=2, sort_keys=True)
    if output is None:
        print(encoded)
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(f"{encoded}\n")


def main() -> int:
    args = parse_args()
    if args.warmups < 0 or args.warm_pairs < 1 or args.cold_pairs < 0:
        raise SystemExit(
            "warmups must be non-negative, warm pairs must be positive, and cold pairs non-negative"
        )
    if args.timeout_seconds <= 0 or args.minimum_free_gib <= 0:
        raise SystemExit("timeout and minimum free space must be positive")
    scenarios = tuple(args.scenario or DEFAULT_SCENARIOS)
    repo_root = Path(__file__).resolve().parents[2]
    require_regular_directory(repo_root, "repository root")
    require_regular_directory(repo_root / "codex-rs", "codex-rs")
    delete_guard = repo_root / ".codex/hooks/permanent_delete.py"
    require_regular_file(delete_guard, "permanent delete guard")
    if repo_root.is_symlink() or (repo_root / "codex-rs").is_symlink():
        raise SystemExit("refusing measurement from a symlinked repository path")
    external_baseline_binary: Path | None = None
    if args.baseline_binary is None:
        baseline = resolve_revision(repo_root, args.baseline)
    else:
        external_baseline_binary = args.baseline_binary.expanduser().resolve()
        require_regular_file(external_baseline_binary, "external baseline binary")
        if not os.access(external_baseline_binary, os.X_OK):
            raise SystemExit(
                f"external baseline binary is not executable: {external_baseline_binary}"
            )
        baseline = f"external:{external_baseline_binary}"
    current = resolve_revision(repo_root, args.current)
    current_model_instructions_file = args.current_model_instructions_file
    if current_model_instructions_file is not None:
        current_model_instructions_file = current_model_instructions_file.resolve()
        require_regular_file(
            current_model_instructions_file, "current model instructions file"
        )
    target = args.target or rust_host_target()
    if "/" in target or ".." in target:
        raise SystemExit(f"refusing invalid target triple: {target}")
    controls = host_controls()
    cold_status = cold_cache_status(args.enable_cold_cache_eviction)
    report: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "mode": "dry-run" if args.dry_run else "measurement",
        "baseline": baseline,
        "current": current,
        "target": target,
        "run": {
            "warmups": args.warmups,
            "warm_pairs": args.warm_pairs,
            "cold_pairs_requested": args.cold_pairs,
            "seed": args.seed,
            "scenarios": scenarios,
            "timeout_seconds": args.timeout_seconds,
            "minimum_free_gib": args.minimum_free_gib,
        },
        "host_controls": controls,
        "cold_cache": cold_status,
        "loopback_server_available": loopback_available(),
        "environment": {
            "credentials_removed": [
                "OPENAI_API_KEY",
                "CODEX_API_KEY",
                "CODEX_ACCESS_TOKEN",
            ],
            "external_proxies_removed": True,
            "model_server": "deterministic loopback Responses SSE",
        },
    }
    if current_model_instructions_file is not None:
        report["instruction_overrides"] = {
            "current": {
                "source_path": str(current_model_instructions_file),
                "source_bytes": current_model_instructions_file.stat().st_size,
                "sha256": sha256_file(current_model_instructions_file),
            }
        }
    if external_baseline_binary is not None:
        report["external_baseline"] = {
            "path": str(external_baseline_binary),
            "version": command_output([str(external_baseline_binary), "--version"]),
        }
    if args.require_ideal_host and not controls["ideal_release_host"]:
        raise SystemExit("host does not meet --require-ideal-host controls")
    if args.dry_run:
        report["classification"] = "dry_run"
        report["note"] = (
            "No worktree, build, benchmark process, cache eviction, or cleanup was started."
        )
        write_report(report, args.output)
        return 0
    if not report["loopback_server_available"]:
        raise SystemExit(
            "loopback sockets are unavailable; deterministic E2E scenarios cannot run"
        )
    processes = active_build_processes()
    if processes:
        raise SystemExit(
            f"refusing measurement while build writers are active: {processes}"
        )
    required_free = int(args.minimum_free_gib * 1024**3)
    if free_bytes(repo_root) < required_free:
        raise SystemExit(
            f"insufficient free space: need {args.minimum_free_gib:g} GiB before isolated release builds"
        )

    root = Path(tempfile.mkdtemp(prefix=".buddy_e2e_performance.", dir=repo_root))
    root.chmod(0o700)
    report["measurement_root"] = str(root)
    report["free_bytes_before"] = free_bytes(repo_root)
    samples: list[dict[str, Any]] = []
    cleanup_completed = False
    try:
        revisions = {
            "baseline": (
                RevisionBinary(
                    label="baseline",
                    revision=baseline,
                    binary=external_baseline_binary,
                    sha256=sha256_file(external_baseline_binary),
                )
                if external_baseline_binary is not None
                else build_revision(repo_root, root, "baseline", baseline, target)
            ),
            "current": build_revision(repo_root, root, "current", current, target),
        }
        report["toolchain"] = {
            "cargo": command_output(["cargo", "-V"]),
            "rustc": command_output(["rustc", "-Vv"]),
            "cargo_incremental": "0",
        }
        report["binaries"] = {
            label: {
                "ephemeral_path": str(revision.binary),
                "release_bytes": revision.binary.stat().st_size,
                "sha256": revision.sha256,
            }
            for label, revision in revisions.items()
        }
        ordinal = 0
        for warmup in range(args.warmups):
            for label in ("baseline", "current"):
                for scenario in scenarios:
                    result = sample(
                        revisions[label],
                        scenario,
                        root,
                        ordinal,
                        args.timeout_seconds,
                        current_model_instructions_file if label == "current" else None,
                    )
                    result.update({"phase": "warmup", "pair": warmup, "order": label})
                    samples.append(result)
                    ordinal += 1
        phases = [("warm", args.warm_pairs)]
        if args.cold_pairs and cold_status["available"]:
            phases.append(("cold", args.cold_pairs))
        for phase, pair_count in phases:
            for pair, order in enumerate(
                balanced_order(args.seed + len(phase), pair_count)
            ):
                for label in order:
                    if phase == "cold":
                        evict_page_cache()
                    for scenario in scenarios:
                        result = sample(
                            revisions[label],
                            scenario,
                            root,
                            ordinal,
                            args.timeout_seconds,
                            current_model_instructions_file
                            if label == "current"
                            else None,
                        )
                        result.update({"phase": phase, "pair": pair, "order": order})
                        samples.append(result)
                        ordinal += 1
        report["samples"] = samples
        primary_metrics = {
            "headless_first_turn": "completion_elapsed_ms",
            "interactive_first_turn": "assistant_output_elapsed_ms",
        }
        report["summaries"] = {
            scenario: {
                phase: paired_summary(samples, scenario, metric, phase, args.seed)
                for phase in ("warm", "cold")
                if any(
                    sample_result["scenario"] == scenario
                    and sample_result["phase"] == phase
                    for sample_result in samples
                )
            }
            for scenario, metric in primary_metrics.items()
            if scenario in scenarios
        }
        report["classification"] = (
            "release_eligible"
            if controls["ideal_release_host"] and cold_status["available"]
            else "non_release_evidence"
        )
    finally:
        cleanup_completed = cleanup_measurement_root(repo_root, root, delete_guard)
        report["cleanup_completed"] = cleanup_completed
        report["free_bytes_after"] = free_bytes(repo_root)
    write_report(report, args.output)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (RuntimeError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"e2e performance comparison failed: {error}") from error
