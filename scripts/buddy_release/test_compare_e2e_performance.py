import http.client
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import compare_e2e_performance as benchmark


class CompareE2ePerformanceTests(unittest.TestCase):
    def test_balanced_order_is_seeded_and_balanced(self) -> None:
        first = benchmark.balanced_order(7, 12)

        self.assertEqual(first, benchmark.balanced_order(7, 12))
        self.assertEqual(len(first), 12)
        self.assertTrue(all(set(order) == {"baseline", "current"} for order in first))

    def test_paired_summary_uses_paired_deltas(self) -> None:
        samples = [
            {
                "scenario": "headless_first_turn",
                "phase": "warm",
                "pair": pair,
                "revision_label": label,
                "success": True,
                "metrics_ms": {"completion_elapsed_ms": value},
            }
            for pair, baseline, current in ((0, 100.0, 102.0), (1, 200.0, 204.0))
            for label, value in (("baseline", baseline), ("current", current))
        ]

        summary = benchmark.paired_summary(
            samples,
            "headless_first_turn",
            "completion_elapsed_ms",
            "warm",
            11,
        )

        self.assertEqual(summary["paired_samples"], 2)
        self.assertEqual(summary["status"], "non_regressing")
        self.assertEqual(summary["median_delta_percent"], 2.0)

    def test_loopback_server_returns_deterministic_sse_and_records_request(
        self,
    ) -> None:
        try:
            server = benchmark.ResponsesServer()
        except PermissionError as error:
            self.skipTest(f"loopback sockets are unavailable: {error}")
        with server:
            connection = http.client.HTTPConnection(
                "127.0.0.1", server._server.server_port
            )
            connection.request(
                "POST",
                "/v1/responses",
                body=b'{"input":"benchmark"}',
                headers={"content-type": "application/json"},
            )
            response = connection.getresponse()
            body = response.read()

            self.assertEqual(response.status, 200)
            self.assertIn(b"response.output_item.done", body)
            self.assertIn(benchmark.RESPONSE_TEXT.encode(), body)
            self.assertEqual(server.evidence()["request_count"], 1)

    def test_request_payload_summary_is_redacted_and_deterministic(self) -> None:
        summary = benchmark.request_payload_summary(
            b'{"instructions":"do not retain me","input":[{"type":"message"}],"tools":[]}'
        )

        self.assertEqual(summary["body_bytes"], 75)
        self.assertEqual(summary["approximate_tokens"], 19)
        self.assertEqual(set(summary["sections"]), {"instructions", "input", "tools"})
        self.assertNotIn("do not retain me", json.dumps(summary))

    def test_isolated_environment_removes_credentials_and_writes_loopback_config(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = os.environ.get("CODEX_API_KEY")
            os.environ["CODEX_API_KEY"] = "not-a-real-key"
            try:
                environment = benchmark.isolated_environment(root / "codex-home")
            finally:
                if original is None:
                    os.environ.pop("CODEX_API_KEY", None)
                else:
                    os.environ["CODEX_API_KEY"] = original
            benchmark.write_config(root / "codex-home", "http://127.0.0.1:1234")

            self.assertNotIn("CODEX_API_KEY", environment)
            self.assertEqual(environment["NO_PROXY"], "127.0.0.1,localhost")
            self.assertIn(
                'base_url = "http://127.0.0.1:1234/v1"',
                (root / "codex-home" / "config.toml").read_text(),
            )

    def test_write_config_copies_an_instruction_override(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "minimal.md"
            source.write_text("Keep this root instruction private.")

            benchmark.write_config(root / "codex-home", "http://127.0.0.1:1234", source)

            config = (root / "codex-home" / "config.toml").read_text()
            copied = root / "codex-home" / "model_instructions.md"
            self.assertIn(f'model_instructions_file = "{copied}"', config)
            self.assertEqual(copied.read_text(), source.read_text())

    def test_fixture_has_expected_event_sequence(self) -> None:
        events = [
            json.loads(line.removeprefix("data: "))
            for line in benchmark.sse_fixture().decode().splitlines()
            if line.startswith("data: ")
        ]

        self.assertEqual(
            [event["type"] for event in events],
            ["response.created", "response.output_item.done", "response.completed"],
        )


if __name__ == "__main__":
    unittest.main()
