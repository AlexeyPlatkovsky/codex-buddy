import errno
import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPT_PATH = Path(__file__).with_name("permanent_delete.py")
SPEC = importlib.util.spec_from_file_location("permanent_delete", SCRIPT_PATH)
assert SPEC is not None
assert SPEC.loader is not None
permanent_delete = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(permanent_delete)


class RewriteTrashCommandTest(unittest.TestCase):
    def test_rewrites_standalone_trash_command(self) -> None:
        rewritten = permanent_delete.rewrite_trash_command(
            "trash 'folder with spaces' file.txt",
            interpreter="/usr/bin/python3",
            script_path=SCRIPT_PATH,
        )

        self.assertEqual(
            rewritten,
            f"/usr/bin/python3 {SCRIPT_PATH} --delete -- 'folder with spaces' file.txt",
        )

    def test_ignores_commands_that_already_delete_permanently(self) -> None:
        self.assertIsNone(
            permanent_delete.rewrite_trash_command(
                "rm -rf build",
                interpreter="python3",
                script_path=SCRIPT_PATH,
            )
        )

    def test_rejects_compound_trash_command(self) -> None:
        with self.assertRaisesRegex(
            permanent_delete.PermanentDeleteError,
            "combined with other shell operations",
        ):
            permanent_delete.rewrite_trash_command(
                "trash build && echo done",
                interpreter="python3",
                script_path=SCRIPT_PATH,
            )

    def test_rejects_trash_after_another_command(self) -> None:
        with self.assertRaisesRegex(
            permanent_delete.PermanentDeleteError,
            "wrapped or compound Trash commands",
        ):
            permanent_delete.rewrite_trash_command(
                "echo ready && /usr/bin/trash build",
                interpreter="python3",
                script_path=SCRIPT_PATH,
            )

    def test_rejects_dynamic_target(self) -> None:
        with self.assertRaisesRegex(
            permanent_delete.PermanentDeleteError,
            "dynamic Trash paths",
        ):
            permanent_delete.rewrite_trash_command(
                "trash *.tmp",
                interpreter="python3",
                script_path=SCRIPT_PATH,
            )


class DeleteTargetsTest(unittest.TestCase):
    def test_deletes_files_directories_and_external_symlinks(self) -> None:
        with (
            tempfile.TemporaryDirectory() as workspace_dir,
            tempfile.TemporaryDirectory() as external_dir,
        ):
            workspace = Path(workspace_dir)
            file_path = workspace / "file.txt"
            file_path.write_text("content")
            directory = workspace / "build"
            directory.mkdir()
            (directory / "artifact").write_text("content")
            external = Path(external_dir) / "keep.txt"
            external.write_text("content")
            link = workspace / "external-link"
            link.symlink_to(external)

            permanent_delete.delete_targets(
                ["file.txt", "build", "external-link"],
                cwd=workspace,
                workspace_root=workspace,
            )

            self.assertFalse(file_path.exists())
            self.assertFalse(directory.exists())
            self.assertFalse(link.exists())
            self.assertTrue(external.exists())

    def test_retries_when_metadata_is_recreated_during_directory_removal(self) -> None:
        with tempfile.TemporaryDirectory() as workspace_dir:
            workspace = Path(workspace_dir)
            directory = workspace / "target"
            directory.mkdir()
            (directory / "artifact").write_text("content")
            real_rmtree = shutil.rmtree
            attempts = 0

            def racing_rmtree(path: Path) -> None:
                nonlocal attempts
                attempts += 1
                real_rmtree(path)
                if attempts == 1:
                    path.mkdir()
                    (path / ".DS_Store").write_text("metadata")
                    raise OSError(errno.ENOTEMPTY, "directory not empty", path)

            with mock.patch.object(
                permanent_delete.shutil,
                "rmtree",
                side_effect=racing_rmtree,
            ):
                permanent_delete.delete_targets(
                    ["target"],
                    cwd=workspace,
                    workspace_root=workspace,
                )

            self.assertEqual(attempts, 2)
            self.assertFalse(directory.exists())

    def test_refuses_workspace_root_and_outside_paths(self) -> None:
        with (
            tempfile.TemporaryDirectory() as workspace_dir,
            tempfile.TemporaryDirectory() as external_dir,
        ):
            workspace = Path(workspace_dir)
            external = Path(external_dir) / "keep.txt"
            external.write_text("content")

            with self.assertRaisesRegex(
                permanent_delete.PermanentDeleteError,
                "workspace root",
            ):
                permanent_delete.delete_targets(
                    [str(workspace)],
                    cwd=workspace,
                    workspace_root=workspace,
                )
            with self.assertRaisesRegex(
                permanent_delete.PermanentDeleteError,
                "outside the workspace",
            ):
                permanent_delete.delete_targets(
                    [str(external)],
                    cwd=workspace,
                    workspace_root=workspace,
                )

            self.assertTrue(external.exists())


if __name__ == "__main__":
    unittest.main()
