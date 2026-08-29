import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import compare_prompt_quality as benchmark


class ComparePromptQualityTests(unittest.TestCase):
    def test_all_fixtures_pass_after_the_expected_change(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for scenario in benchmark.DEFAULT_SCENARIOS:
                fixture = root / scenario
                fixture.mkdir()
                benchmark.write_fixture(fixture, scenario)
                if scenario == "calculator_fix":
                    (fixture / "calculator.py").write_text(
                        "def add(left: int, right: int) -> int:\n    return left + right\n"
                    )
                elif scenario == "nested_agents":
                    (fixture / "services" / "config.txt").write_text(
                        "mode=restricted\n"
                    )
                elif scenario == "slugify_fix":
                    (fixture / "text_tools.py").write_text(
                        "def slugify(value: str) -> str:\n"
                        '    return value.strip().lower().replace(" ", "-")\n'
                    )
                elif scenario == "multistep_release":
                    (fixture / "formatter.py").write_text(
                        "def render(value: str) -> str:\n    return value.lower()\n"
                    )
                    (fixture / "CHANGELOG.md").write_text(
                        "# Changelog\n\nFormatter fix\n"
                    )
                elif scenario == "retry_after_tool_failure":
                    (fixture / ".retry-attempts").write_text("2")
                    (fixture / ".retry-success").write_text("verified\n")
                elif scenario == "project_skill_file":
                    (fixture / "skill_result.txt").write_text("skill=used\n")
                elif scenario == "subagent_delegation":
                    (fixture / "agent_note.txt").write_text("agent=used\n")
                    (fixture / "final.txt").write_text("delegation=complete\n")
                else:
                    (fixture / "copied.txt").write_text("sentinel\n")

                self.assertTrue(benchmark.grade_fixture(fixture, scenario)["passed"])

    def test_test_modification_does_not_pass_the_grader(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            benchmark.write_fixture(root, "calculator_fix")
            expected_protected_hashes = benchmark.protected_hashes(
                root, "calculator_fix"
            )
            (root / "test_calculator.py").write_text("import unittest\n")

            result = benchmark.grade_fixture(
                root, "calculator_fix", expected_protected_hashes
            )

            self.assertFalse(result["passed"])

    def test_nested_agents_grader_accepts_the_exact_content_without_a_trailing_newline(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            benchmark.write_fixture(root, "nested_agents")
            expected_protected_hashes = benchmark.protected_hashes(
                root, "nested_agents"
            )
            (root / "services" / "config.txt").write_text("mode=restricted")

            result = benchmark.grade_fixture(
                root, "nested_agents", expected_protected_hashes
            )

            self.assertTrue(result["passed"])

    def test_command_uses_only_current_instruction_override(self) -> None:
        scenario = benchmark.SCENARIOS["calculator_fix"]
        root = Path("/tmp/quality-fixture")
        command = benchmark.command_for(
            Path("/tmp/codex"),
            scenario,
            root,
            root / "last-message.txt",
            None,
            Path("/tmp/minimal.md"),
        )

        self.assertIn("--ignore-user-config", command)
        self.assertIn('model_instructions_file="/tmp/minimal.md"', command)
        self.assertIn('approval_policy="never"', command)

    def test_subagent_command_enables_both_runtime_versions(self) -> None:
        scenario = benchmark.SCENARIOS["subagent_delegation"]
        root = Path("/tmp/quality-fixture")

        command = benchmark.command_for(
            Path("/tmp/codex"),
            scenario,
            root,
            root / "last-message.txt",
            None,
            None,
        )

        self.assertIn("features.multi_agent=true", command)
        self.assertIn("features.multi_agent_v2=true", command)
        self.assertIn("--json", command)

    def test_decoded_output_handles_timeout_bytes(self) -> None:
        self.assertEqual(
            benchmark.decoded_output(b"incomplete output"), "incomplete output"
        )
        self.assertEqual(benchmark.decoded_output(None), "")


if __name__ == "__main__":
    unittest.main()
