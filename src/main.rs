use clap::{Args, Parser, Subcommand};

use miette::Result;

#[macro_use]
extern crate log;
extern crate simplelog;

use simplelog::*;

use std::env;
use std::sync::LazyLock;
use std::time::Instant;

use gitlab::Gitlab;

mod error;
mod system;
use system::System;

mod job;

mod pipeline;

mod environment;
use environment::Environment;

mod semver_type;
use semver_type::SemverType;

mod release;
use release::Release;

mod hotfix;
use hotfix::Hotfix;

use crate::error::GitlabError;
use crate::git::{
    get_gitflow_branch_name, get_gitlab_host, get_gitlab_token, get_project_name, get_repository,
};

mod git;
mod repository_status;

const DEVELOP: &str = "develop";
const MAIN: &str = "main";

static DEVELOP_BRANCH: LazyLock<String> = LazyLock::new(|| get_gitflow_branch_name(DEVELOP));
static MAIN_BRANCH: LazyLock<String> = LazyLock::new(|| get_gitflow_branch_name(MAIN));
static PROJECT_NAME: LazyLock<String> = LazyLock::new(get_project_name);
static GITLAB_HOST: LazyLock<String> = LazyLock::new(get_gitlab_host);
static GITLAB_TOKEN: LazyLock<String> = LazyLock::new(get_gitlab_token);

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Legacy: `wr [flags]` with no subcommand means release.
    #[command(flatten)]
    release: ReleaseArgs,
}

#[derive(Subcommand)]
enum Commands {
    /// Create and optionally deploy a release
    Release(ReleaseArgs),

    /// Create and optionally deploy a hotfix from commit hashes
    Hotfix(HotfixArgs),
}

#[derive(Args)]
struct ReleaseArgs {
    /// Launch a deploy job after the release
    #[arg(long)]
    deploy: bool,

    /// Print additional debug information
    #[arg(short, long)]
    debug: bool,

    /// Allow to make a release even if the remote is up to date
    #[arg(short, long)]
    force: bool,

    /// Define the deploy environment
    #[arg(short, long, value_enum, default_value_t = Environment::Production)]
    environment: Environment,

    /// Define how to increment the version number
    #[arg(short, long, value_enum, default_value_t = SemverType::Patch)]
    semver_type: SemverType,
}

#[derive(Args)]
struct HotfixArgs {
    /// Commit hashes to cherry-pick onto the hotfix, in order
    #[arg(required = true)]
    commits: Vec<String>,

    /// Launch a deploy job after the hotfix
    #[arg(long)]
    deploy: bool,

    /// Print additional debug information
    #[arg(short, long)]
    debug: bool,

    /// Unused for hotfix status checks; kept for CLI consistency with release
    #[arg(short, long)]
    force: bool,
}

fn init_logging(debug: bool) {
    let level = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let mut log_stdout_config_builder = ConfigBuilder::default();
    log_stdout_config_builder
        .set_time_offset_to_local()
        .unwrap();

    TermLogger::init(
        level,
        log_stdout_config_builder.build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();
}

fn setup_env() {
    env::set_var("LANG", "en_US.UTF-8");
    env::set_var("GIT_MERGE_AUTOEDIT", "no");
}

fn connect_gitlab() -> Result<Gitlab, GitlabError> {
    info!("[Setup] Login into Gitlab instance \"{}\".", *GITLAB_HOST);
    Gitlab::new(&*GITLAB_HOST, &*GITLAB_TOKEN).map_err(|source| GitlabError::ConnectFailed {
        host: GITLAB_HOST.clone(),
        source,
    })
}

fn maybe_deploy(release: &Release<'_>, system: &System<'_>, deploy: bool) -> Result<()> {
    if !deploy {
        return Ok(());
    }

    if system.has_gitlab_ci() {
        debug!("\"deploy\" flag was found, trying to play the \"deploy\" job.");
        release.deploy()?;
    } else {
        warn!("\"deploy\" flag was found, but the repository has no \".gitlab-ci.yml\" file, impossible to deploy.")
    }

    Ok(())
}

fn run_release(matches: ReleaseArgs) -> Result<()> {
    init_logging(matches.debug);
    setup_env();

    info!("Welcome to wr.");

    let repository = get_repository()?;

    let s = System {
        repository: &repository,
        force: matches.force,
    };
    info!("[Setup] Performing system checks.");
    s.system_check()?;

    let environment: Environment = matches.environment;
    info!("[Setup] {environment} environment was found from the arguments.");

    let semver_type: SemverType = matches.semver_type;
    info!("[Setup] {semver_type} semver type was found from the arguments.");

    let gitlab = connect_gitlab()?;

    let release = Release {
        gitlab,
        repository: &repository,
        environment,
        semver_type,
    };

    debug!("[Release] Creating a new {environment} release.");
    release.create()?;
    info!("[Release] A new {environment} release has been created.");

    debug!("[Release] Pushing the {environment} release to the remote repository.");
    release.push()?;
    info!("[Release] {environment} release has been pushed to the remote repository.");

    maybe_deploy(&release, &s, matches.deploy)?;

    Ok(())
}

fn run_hotfix(matches: HotfixArgs) -> Result<()> {
    init_logging(matches.debug);
    setup_env();

    info!("Welcome to wr.");

    let repository = get_repository()?;

    let s = System {
        repository: &repository,
        force: matches.force,
    };
    info!("[Setup] Performing system checks.");
    s.hotfix_system_check()?;

    let gitlab = connect_gitlab()?;

    let release = Release {
        gitlab,
        repository: &repository,
        environment: Environment::Production,
        semver_type: SemverType::Patch,
    };

    let hotfix = Hotfix {
        repository: &repository,
        commits: &matches.commits,
    };

    debug!("[Hotfix] Creating a new production hotfix.");
    hotfix.create(&release)?;
    info!("[Hotfix] A new production hotfix has been created.");

    debug!("[Hotfix] Pushing the hotfix to the remote repository.");
    release.push()?;
    info!("[Hotfix] Hotfix has been pushed to the remote repository.");

    maybe_deploy(&release, &s, matches.deploy)?;

    Ok(())
}

fn app() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Release(matches)) => run_release(matches),
        Some(Commands::Hotfix(matches)) => run_hotfix(matches),
        None => run_release(cli.release),
    }
}

fn main() -> Result<()> {
    let started = Instant::now();
    app()?;
    info!("Done in {:?}.", started.elapsed());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use serial_test::serial;
    use tempfile::TempDir;

    fn parse(args: &[&str]) -> Commands {
        let cli = Cli::try_parse_from(args).expect("cli");
        match cli.command {
            Some(command) => command,
            None => Commands::Release(cli.release),
        }
    }

    #[test]
    fn no_command_falls_back_to_release() {
        match parse(&["wr"]) {
            Commands::Release(args) => {
                assert!(!args.deploy);
                assert!(!args.force);
                assert_eq!(args.environment, Environment::Production);
                assert_eq!(args.semver_type, SemverType::Patch);
            }
            Commands::Hotfix(_) => panic!("expected release"),
        }
    }

    #[test]
    fn flags_without_command_fall_back_to_release() {
        match parse(&["wr", "--deploy", "-f"]) {
            Commands::Release(args) => {
                assert!(args.deploy);
                assert!(args.force);
            }
            Commands::Hotfix(_) => panic!("expected release"),
        }
    }

    #[test]
    fn release_parses_environment_and_semver() {
        match parse(&[
            "wr",
            "release",
            "--environment",
            "staging",
            "--semver-type",
            "minor",
            "--deploy",
        ]) {
            Commands::Release(args) => {
                assert_eq!(args.environment, Environment::Staging);
                assert_eq!(args.semver_type, SemverType::Minor);
                assert!(args.deploy);
            }
            Commands::Hotfix(_) => panic!("expected release"),
        }
    }

    #[test]
    fn hotfix_parses_commits_and_deploy() {
        match parse(&["wr", "hotfix", "abc", "def", "--deploy"]) {
            Commands::Hotfix(args) => {
                assert_eq!(args.commits, ["abc", "def"]);
                assert!(args.deploy);
            }
            Commands::Release(_) => panic!("expected hotfix"),
        }
    }

    #[test]
    fn hotfix_requires_commit_arg() {
        assert!(Cli::try_parse_from(["wr", "hotfix"]).is_err());
    }

    #[test]
    #[serial]
    fn maybe_deploy_skips_when_flag_false_or_no_ci() {
        use httpmock::Method::GET;
        use httpmock::MockServer;

        let temp_dir = TempDir::new().unwrap();
        let repo = Repository::init(temp_dir.path()).unwrap();

        let server = MockServer::start();
        let _user = server.mock(|when, then| {
            when.method(GET).path("/api/v4/user");
            then.status(200).body("{}");
        });
        let gitlab = Gitlab::new_insecure(format!("127.0.0.1:{}", server.port()), "token").unwrap();

        let release = Release {
            gitlab,
            repository: &repo,
            environment: Environment::Production,
            semver_type: SemverType::Patch,
        };
        let system = System {
            repository: &repo,
            force: false,
        };

        assert!(maybe_deploy(&release, &system, false).is_ok());

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();
        assert!(!system.has_gitlab_ci());
        // deploy=true but no .gitlab-ci.yml → warn and skip (never calls GitLab).
        assert!(maybe_deploy(&release, &system, true).is_ok());
        env::set_current_dir(original_dir).unwrap();
    }
}
