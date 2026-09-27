use super::ensure_process_hint;
use pretty_assertions::assert_eq;
use std::os::unix::process::CommandExt;
use std::process::Command;

#[test]
fn herdr_hint_is_visible_in_the_process_environment_and_preserves_arguments() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let report = directory.path().join("report");
    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg0("buddy-herdr-dispatch-alias")
        .args([
            "--exact",
            "herdr::tests::child_probe",
            "--ignored",
            "--nocapture",
        ])
        .env("HERDR_PANE_ID", "test:pane")
        .env_remove("HERDR_AGENT")
        .env("BUDDY_HERDR_TEST_REPORT", &report)
        .env("BUDDY_HERDR_TEST_EXPECT_HINT", "codex")
        .status()
        .expect("run child");
    assert!(status.success());
    let report = std::fs::read_to_string(report).expect("read report");
    assert_eq!(
        report,
        concat!(
            "Some(\"codex\")\n",
            "[\"buddy-herdr-dispatch-alias\", \"--exact\", ",
            "\"herdr::tests::child_probe\", \"--ignored\", \"--nocapture\"]\n",
        )
    );
}

#[test]
fn herdr_hint_respects_existing_values_and_requires_a_pane() {
    for (pane, hint) in [
        (Some("test:pane"), Some("claude")),
        (Some("test:pane"), Some("")),
        (None, None),
        (Some(""), None),
    ] {
        let directory = tempfile::tempdir().expect("temporary directory");
        let report = directory.path().join("report");
        let mut command = Command::new(std::env::current_exe().expect("test executable"));
        command
            .args(["--exact", "herdr::tests::child_probe", "--ignored"])
            .env_remove("HERDR_PANE_ID")
            .env_remove("HERDR_AGENT")
            .env_remove("BUDDY_HERDR_TEST_EXPECT_HINT")
            .env("BUDDY_HERDR_TEST_REPORT", &report);
        if let Some(pane) = pane {
            command.env("HERDR_PANE_ID", pane);
        }
        if let Some(hint) = hint {
            command.env("HERDR_AGENT", hint);
        }
        assert!(command.status().expect("run child").success());
        let report = std::fs::read_to_string(report).expect("read report");
        assert!(report.starts_with(&format!("{hint:?}\n")));
    }
}

#[test]
#[ignore = "subprocess fixture for the startup environment tests"]
fn child_probe() {
    ensure_process_hint().expect("prepare Herdr process hint");
    if let Ok(expected) = std::env::var("BUDDY_HERDR_TEST_EXPECT_HINT") {
        // Herdr reads the OS launch environment, rather than Rust's current
        // environment map. Verify that boundary after the startup adapter.
        #[cfg(target_os = "linux")]
        let environment = std::fs::read("/proc/self/environ").expect("process environment");
        #[cfg(target_os = "macos")]
        let environment = {
            let output = Command::new("/bin/ps")
                .args([
                    "eww",
                    "-p",
                    &std::process::id().to_string(),
                    "-o",
                    "command=",
                ])
                .output()
                .expect("read process launch environment");
            assert!(output.status.success());
            output.stdout
        };
        let needle = format!("HERDR_AGENT={expected}");
        assert!(
            environment
                .windows(needle.len())
                .any(|window| window == needle.as_bytes()),
            "Herdr hint must be visible through the OS process environment"
        );
    }
    std::fs::write(
        std::env::var_os("BUDDY_HERDR_TEST_REPORT").expect("report path"),
        format!(
            "{:?}\n{:?}\n",
            std::env::var("HERDR_AGENT").ok(),
            std::env::args_os().collect::<Vec<_>>()
        ),
    )
    .expect("write child report");
}
