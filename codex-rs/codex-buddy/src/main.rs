use anyhow::Result;
use clap::Args;
use clap::FromArgMatches;
use clap::Parser;
use codex_arg0::Arg0DispatchPaths;
use codex_arg0::arg0_dispatch_or_else;
use codex_chatgpt::apply_command::ApplyCommand;
use codex_chatgpt::apply_command::run_apply_command;
use codex_cli::mcp_cmd::McpCli;
use codex_cli::read_access_token_from_stdin;
use codex_cli::read_api_key_from_stdin;
use codex_cli::run_login_status;
use codex_cli::run_login_with_access_token;
use codex_cli::run_login_with_api_key;
use codex_cli::run_login_with_chatgpt;
use codex_cli::run_login_with_device_code;
use codex_cli::run_logout;
use codex_config::LoaderOverrides;
use codex_exec::Cli as ExecCli;
use codex_exec::Command as ExecCommand;
use codex_exec::ReviewArgs;
use codex_runtime_profile::RuntimePreset;
use codex_tui::Cli as TuiCli;
use codex_utils_cli::CliConfigOverrides;

const BUDDY_RUNTIME_PRESET: RuntimePreset = RuntimePreset::Coding;

#[cfg(target_os = "macos")]
type HostSandboxArgs = codex_cli::SeatbeltCommand;
#[cfg(target_os = "linux")]
type HostSandboxArgs = codex_cli::LandlockCommand;
#[cfg(target_os = "windows")]
type HostSandboxArgs = codex_cli::WindowsCommand;

#[derive(Debug, Parser)]
#[command(
    name = "codex-buddy",
    bin_name = "codex-buddy",
    version,
    about = "Coding-focused Codex CLI",
    subcommand_negates_reqs = true,
    override_usage = "codex-buddy [OPTIONS] [PROMPT]\n       codex-buddy [OPTIONS] <COMMAND> [ARGS]"
)]
struct BuddyCli {
    #[clap(flatten)]
    config_overrides: CliConfigOverrides,
    #[clap(flatten)]
    interactive: TuiCli,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    /// Run Codex non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),
    /// Run a code review non-interactively.
    Review(ReviewCommand),
    /// Manage login (also available as `auth`).
    #[clap(visible_alias = "auth")]
    Login(LoginCommand),
    /// Remove stored authentication credentials.
    Logout(LogoutCommand),
    /// Manage explicitly configured MCP servers.
    Mcp(McpCli),
    /// Run commands within a Codex-provided sandbox.
    Sandbox(HostSandboxArgs),
    /// Apply the latest agent diff to the working tree.
    Apply(ApplyCommand),
    /// Resume a previous interactive session.
    Resume(ResumeCommand),
    /// Fork a previous interactive session.
    Fork(ForkCommand),
}

#[derive(Debug, Parser)]
struct ReviewCommand {
    #[arg(long = "strict-config", default_value_t = false)]
    strict_config: bool,
    #[clap(flatten)]
    args: ReviewArgs,
}

#[derive(Debug, Parser)]
struct LoginCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,
    #[arg(
        long = "with-api-key",
        conflicts_with_all = ["with_access_token", "use_device_code"]
    )]
    with_api_key: bool,
    #[arg(
        long = "with-access-token",
        conflicts_with_all = ["with_api_key", "use_device_code"]
    )]
    with_access_token: bool,
    #[arg(
        long = "device-auth",
        conflicts_with_all = ["with_api_key", "with_access_token"]
    )]
    use_device_code: bool,
    #[arg(long = "experimental_issuer", hide = true)]
    issuer_base_url: Option<String>,
    #[arg(long = "experimental_client-id", hide = true)]
    client_id: Option<String>,
    #[command(subcommand)]
    action: Option<LoginAction>,
}

#[derive(Debug, clap::Subcommand)]
enum LoginAction {
    /// Show login status.
    Status,
}

#[derive(Debug, Parser)]
struct LogoutCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
struct ResumeCommand {
    #[arg(value_name = "SESSION_ID")]
    session_id: Option<String>,
    #[arg(long, default_value_t = false)]
    last: bool,
    #[arg(long, default_value_t = false)]
    all: bool,
    #[arg(long = "include-non-interactive", default_value_t = false)]
    include_non_interactive: bool,
    #[clap(flatten)]
    config_overrides: SessionTuiCli,
}

#[derive(Debug, Parser)]
struct ForkCommand {
    #[arg(value_name = "SESSION_ID")]
    session_id: Option<String>,
    #[arg(long, default_value_t = false)]
    last: bool,
    #[arg(long, default_value_t = false)]
    all: bool,
    #[clap(flatten)]
    config_overrides: SessionTuiCli,
}

#[derive(Debug)]
struct SessionTuiCli(TuiCli);

impl Args for SessionTuiCli {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        TuiCli::augment_args(cmd).mut_arg("prompt", |arg| arg.conflicts_with("last"))
    }
    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        TuiCli::augment_args_for_update(cmd).mut_arg("prompt", |arg| arg.conflicts_with("last"))
    }
}

impl FromArgMatches for SessionTuiCli {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        TuiCli::from_arg_matches(matches).map(Self)
    }
    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        self.0.update_from_arg_matches(matches)
    }
}

fn main() -> Result<()> {
    arg0_dispatch_or_else(|paths| async move { run(paths).await })
}

async fn run(paths: Arg0DispatchPaths) -> Result<()> {
    let BuddyCli {
        config_overrides: root,
        mut interactive,
        command,
    } = BuddyCli::parse();
    reject_root_strict_config_for_subcommand(interactive.strict_config, &command)?;
    reject_root_profile_for_subcommand(interactive.config_profile_v2.as_ref(), &command)?;
    match command {
        None => {
            interactive.config_overrides.prepend_root_overrides(root);
            codex_tui::run_main_with_runtime_preset(
                interactive,
                paths,
                LoaderOverrides::default(),
                None,
                BUDDY_RUNTIME_PRESET,
            )
            .await?;
        }
        Some(Command::Exec(mut exec)) => {
            inherit_exec_root_options(&mut exec, &interactive);
            exec.config_overrides.prepend_root_overrides(root);
            codex_exec::run_main_with_runtime_preset(exec, paths, BUDDY_RUNTIME_PRESET).await?;
        }
        Some(Command::Review(ReviewCommand {
            strict_config,
            args,
        })) => {
            let mut exec = ExecCli::try_parse_from(["codex-buddy", "exec"])?;
            exec.strict_config = strict_config;
            inherit_exec_root_options(&mut exec, &interactive);
            exec.command = Some(ExecCommand::Review(args));
            exec.config_overrides.prepend_root_overrides(root);
            codex_exec::run_main_with_runtime_preset(exec, paths, BUDDY_RUNTIME_PRESET).await?;
        }
        Some(Command::Login(mut login)) => {
            login.config_overrides.prepend_root_overrides(root);
            match login.action {
                Some(LoginAction::Status) => run_login_status(login.config_overrides).await,
                None if login.use_device_code => {
                    run_login_with_device_code(
                        login.config_overrides,
                        login.issuer_base_url,
                        login.client_id,
                    )
                    .await
                }
                None if login.with_api_key => {
                    run_login_with_api_key(login.config_overrides, read_api_key_from_stdin()).await
                }
                None if login.with_access_token => {
                    run_login_with_access_token(
                        login.config_overrides,
                        read_access_token_from_stdin(),
                    )
                    .await
                }
                None => run_login_with_chatgpt(login.config_overrides).await,
            }
        }
        Some(Command::Logout(mut logout)) => {
            logout.config_overrides.prepend_root_overrides(root);
            run_logout(logout.config_overrides).await;
        }
        Some(Command::Mcp(mut mcp)) => {
            mcp.config_overrides.prepend_root_overrides(root);
            let loader_overrides =
                codex_cli::loader_overrides_for_profile(interactive.config_profile_v2.as_ref())?;
            mcp.run_with_runtime_preset(loader_overrides, BUDDY_RUNTIME_PRESET)
                .await?;
        }
        Some(Command::Sandbox(mut sandbox)) => {
            let config_profile = sandbox
                .config_profile
                .as_ref()
                .or(interactive.config_profile_v2.as_ref());
            let loader_overrides = codex_cli::loader_overrides_for_profile(config_profile)?;
            sandbox.config_overrides.prepend_root_overrides(root);
            #[cfg(target_os = "windows")]
            if sandbox
                .command
                .first()
                .is_some_and(|command| command == "setup")
            {
                anyhow::bail!(
                    "`codex-buddy sandbox setup` is not supported; use `codex sandbox setup`"
                );
            }
            #[cfg(target_os = "macos")]
            codex_cli::run_command_under_seatbelt(
                sandbox,
                paths.codex_linux_sandbox_exe,
                loader_overrides,
            )
            .await?;
            #[cfg(target_os = "linux")]
            codex_cli::run_command_under_landlock(
                sandbox,
                paths.codex_linux_sandbox_exe,
                loader_overrides,
            )
            .await?;
            #[cfg(target_os = "windows")]
            codex_cli::run_command_under_windows_sandbox(
                sandbox,
                paths.codex_linux_sandbox_exe,
                loader_overrides,
            )
            .await?;
        }
        Some(Command::Apply(mut apply)) => {
            apply.config_overrides.prepend_root_overrides(root);
            run_apply_command(apply, None).await?;
        }
        Some(Command::Resume(command)) => {
            let SessionTuiCli(mut options) = command.config_overrides;
            options.config_overrides.prepend_root_overrides(root);
            let session_id = resolve_session_id(&mut options, command.session_id, command.last);
            merge_interactive_cli_flags(&mut interactive, options);
            interactive.resume_session_id = session_id;
            interactive.resume_last = command.last;
            interactive.resume_picker = interactive.resume_session_id.is_none() && !command.last;
            interactive.resume_show_all = command.all;
            interactive.resume_include_non_interactive = command.include_non_interactive;
            codex_tui::run_main_with_runtime_preset(
                interactive,
                paths,
                LoaderOverrides::default(),
                None,
                BUDDY_RUNTIME_PRESET,
            )
            .await?;
        }
        Some(Command::Fork(command)) => {
            let SessionTuiCli(mut options) = command.config_overrides;
            options.config_overrides.prepend_root_overrides(root);
            let session_id = resolve_session_id(&mut options, command.session_id, command.last);
            merge_interactive_cli_flags(&mut interactive, options);
            interactive.fork_session_id = session_id;
            interactive.fork_last = command.last;
            interactive.fork_picker = interactive.fork_session_id.is_none() && !command.last;
            interactive.fork_show_all = command.all;
            codex_tui::run_main_with_runtime_preset(
                interactive,
                paths,
                LoaderOverrides::default(),
                None,
                BUDDY_RUNTIME_PRESET,
            )
            .await?;
        }
    }
    Ok(())
}

fn reject_root_strict_config_for_subcommand(
    strict_config: bool,
    command: &Option<Command>,
) -> Result<()> {
    if !strict_config {
        return Ok(());
    }
    let unsupported = match command {
        None
        | Some(Command::Exec(_))
        | Some(Command::Review(_))
        | Some(Command::Resume(_))
        | Some(Command::Fork(_)) => None,
        Some(Command::Login(_)) => Some("login"),
        Some(Command::Logout(_)) => Some("logout"),
        Some(Command::Mcp(_)) => Some("mcp"),
        Some(Command::Sandbox(_)) => Some("sandbox"),
        Some(Command::Apply(_)) => Some("apply"),
    };
    if let Some(command) = unsupported {
        anyhow::bail!("`--strict-config` is not supported for `codex-buddy {command}`");
    }
    Ok(())
}

fn reject_root_profile_for_subcommand(
    profile: Option<&codex_utils_cli::ProfileV2Name>,
    command: &Option<Command>,
) -> Result<()> {
    if profile.is_none() {
        return Ok(());
    }
    match command {
        Some(Command::Login(_)) | Some(Command::Logout(_)) | Some(Command::Apply(_)) => {
            anyhow::bail!(
                "--profile only applies to runtime commands and `codex-buddy mcp`: `codex-buddy`, `codex-buddy exec`, `codex-buddy review`, `codex-buddy resume`, `codex-buddy fork`, `codex-buddy mcp`, and `codex-buddy sandbox`."
            )
        }
        None
        | Some(Command::Exec(_))
        | Some(Command::Review(_))
        | Some(Command::Mcp(_))
        | Some(Command::Sandbox(_))
        | Some(Command::Resume(_))
        | Some(Command::Fork(_)) => Ok(()),
    }
}

fn inherit_exec_root_options(exec: &mut ExecCli, interactive: &TuiCli) {
    exec.shared.inherit_exec_root_options(&interactive.shared);
    exec.strict_config |= interactive.strict_config;
}

fn resolve_session_id(
    options: &mut TuiCli,
    session_id: Option<String>,
    last: bool,
) -> Option<String> {
    if last && options.prompt.is_none() {
        options.prompt = session_id;
        None
    } else {
        session_id
    }
}

fn merge_interactive_cli_flags(interactive: &mut TuiCli, subcommand_cli: TuiCli) {
    let TuiCli {
        shared,
        strict_config,
        approval_policy,
        web_search,
        no_alt_screen,
        prompt,
        mut config_overrides,
        ..
    } = subcommand_cli;
    let subcommand_auto_review = shared.auto_review;
    interactive
        .shared
        .apply_subcommand_overrides(shared.into_inner());
    interactive
        .shared
        .take_auto_review_config_overrides(&mut config_overrides);
    if subcommand_auto_review {
        interactive.approval_policy = None;
    } else if let Some(approval) = approval_policy {
        interactive.approval_policy = Some(approval);
    }
    if web_search {
        interactive.web_search = true;
    }
    interactive.no_alt_screen |= no_alt_screen;
    if strict_config {
        interactive.strict_config = true;
    }
    if let Some(prompt) = prompt {
        interactive.prompt = Some(prompt.replace("\r\n", "\n").replace('\r', "\n"));
    }
    interactive
        .config_overrides
        .raw_overrides
        .extend(config_overrides.raw_overrides);
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use codex_core::config::ConfigBuilder;
    use tempfile::TempDir;

    #[test]
    fn command_surface_is_exact() {
        let cli = BuddyCli::command();
        let mut names: Vec<_> = cli.get_subcommands().map(clap::Command::get_name).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![
                "apply", "exec", "fork", "login", "logout", "mcp", "resume", "review", "sandbox",
            ]
        );
    }

    #[test]
    fn help_excludes_non_coding_commands() {
        let help = BuddyCli::command().render_help().to_string();
        for name in [
            "cloud",
            "app-server",
            "remote-control",
            "mcp-server",
            "exec-server",
            "responses-api-proxy",
            "stdio-to-uds",
            "plugin",
            "debug",
        ] {
            assert!(
                !help
                    .lines()
                    .any(|line| line.starts_with(&format!("  {name} ")))
            );
        }
    }

    #[test]
    fn login_rejects_multiple_credential_sources() {
        assert!(
            BuddyCli::try_parse_from([
                "codex-buddy",
                "login",
                "--with-api-key",
                "--with-access-token",
            ])
            .is_err()
        );
    }

    #[test]
    fn exec_subcommand_options_override_root_options() {
        let BuddyCli {
            interactive,
            command: Some(Command::Exec(mut exec)),
            ..
        } = BuddyCli::try_parse_from([
            "codex-buddy",
            "--model",
            "root-model",
            "--strict-config",
            "exec",
            "--model",
            "subcommand-model",
        ])
        .expect("parse exec invocation")
        else {
            panic!("expected exec command");
        };

        inherit_exec_root_options(&mut exec, &interactive);
        assert_eq!(exec.shared.model.as_deref(), Some("subcommand-model"));
        assert!(exec.strict_config);
    }

    #[test]
    fn root_strict_config_rejects_unsupported_commands() {
        for args in [
            vec!["codex-buddy", "--strict-config", "login", "status"],
            vec!["codex-buddy", "--strict-config", "logout"],
            vec!["codex-buddy", "--strict-config", "mcp", "list"],
            vec!["codex-buddy", "--strict-config", "sandbox", "--", "true"],
            vec!["codex-buddy", "--strict-config", "apply", "1"],
        ] {
            let cli = BuddyCli::try_parse_from(args).expect("parse unsupported invocation");
            assert!(
                reject_root_strict_config_for_subcommand(
                    cli.interactive.strict_config,
                    &cli.command,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn root_profile_rejects_commands_that_cannot_honor_it() {
        for args in [
            vec!["codex-buddy", "--profile", "work", "login", "status"],
            vec!["codex-buddy", "--profile", "work", "logout"],
            vec!["codex-buddy", "--profile", "work", "apply", "1"],
        ] {
            let cli = BuddyCli::try_parse_from(args).expect("parse unsupported invocation");
            assert!(
                reject_root_profile_for_subcommand(
                    cli.interactive.config_profile_v2.as_ref(),
                    &cli.command,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn resume_subcommand_overrides_root_and_last_argument_becomes_prompt() {
        let BuddyCli {
            mut interactive,
            command: Some(Command::Resume(command)),
            ..
        } = BuddyCli::try_parse_from([
            "codex-buddy",
            "--model",
            "root-model",
            "resume",
            "--last",
            "continue here",
            "--model",
            "subcommand-model",
        ])
        .expect("parse resume invocation")
        else {
            panic!("expected resume command");
        };
        let SessionTuiCli(mut options) = command.config_overrides;
        let session_id = resolve_session_id(&mut options, command.session_id, command.last);
        merge_interactive_cli_flags(&mut interactive, options);

        assert_eq!(session_id, None);
        assert_eq!(interactive.prompt.as_deref(), Some("continue here"));
        assert_eq!(
            interactive.shared.model.as_deref(),
            Some("subcommand-model")
        );
    }

    #[test]
    fn review_inherits_root_strict_config() {
        let BuddyCli {
            interactive,
            command:
                Some(Command::Review(ReviewCommand {
                    strict_config,
                    args,
                })),
            ..
        } = BuddyCli::try_parse_from(["codex-buddy", "--strict-config", "review"])
            .expect("parse review invocation")
        else {
            panic!("expected review command");
        };
        let mut exec = ExecCli::try_parse_from(["codex-buddy", "exec"])
            .expect("construct review execution command");
        exec.strict_config = strict_config;
        inherit_exec_root_options(&mut exec, &interactive);
        exec.command = Some(ExecCommand::Review(args));

        assert!(exec.strict_config);
    }

    #[test]
    fn resume_last_treats_session_argument_as_prompt() {
        let BuddyCli {
            command: Some(Command::Resume(command)),
            ..
        } = BuddyCli::try_parse_from(["codex-buddy", "resume", "--last", "continue here"])
            .expect("parse resume --last")
        else {
            panic!("expected resume command");
        };
        let SessionTuiCli(mut options) = command.config_overrides;

        let session_id = resolve_session_id(&mut options, command.session_id, command.last);
        assert_eq!(session_id, None);
        assert_eq!(options.prompt.as_deref(), Some("continue here"));
    }

    #[test]
    fn fork_last_treats_session_argument_as_prompt() {
        let BuddyCli {
            command: Some(Command::Fork(command)),
            ..
        } = BuddyCli::try_parse_from(["codex-buddy", "fork", "--last", "try again"])
            .expect("parse fork --last")
        else {
            panic!("expected fork command");
        };
        let SessionTuiCli(mut options) = command.config_overrides;

        let session_id = resolve_session_id(&mut options, command.session_id, command.last);
        assert_eq!(session_id, None);
        assert_eq!(options.prompt.as_deref(), Some("try again"));
    }

    #[tokio::test]
    async fn buddy_runtime_preset_resolves_to_coding() {
        let home = TempDir::new().expect("temporary Codex home");
        let config = ConfigBuilder::default()
            .codex_home(home.path().to_path_buf())
            .runtime_preset(RuntimePreset::Coding)
            .build()
            .await
            .expect("coding config");
        assert_eq!(BUDDY_RUNTIME_PRESET, RuntimePreset::Coding);
        assert_eq!(config.runtime_profile.preset(), BUDDY_RUNTIME_PRESET);
    }
}
