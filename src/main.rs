use clap::{Parser, Subcommand};

use anyhow::{anyhow, Error};

#[macro_use]
extern crate log;
extern crate simplelog;

#[macro_use]
extern crate lazy_static;

use indicatif::HumanDuration;
use simplelog::*;

use std::env;
use std::process;
use std::time::Instant;

use gitlab::Gitlab;

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

use crate::git::{
    get_gitflow_branch_name, get_gitlab_host, get_gitlab_token, get_project_name, get_repository,
};

mod git;
mod repository_status;

const DEVELOP: &str = "develop";
const MAIN: &str = "main";

lazy_static! {
    static ref DEVELOP_BRANCH: String = get_gitflow_branch_name(DEVELOP);
    static ref MAIN_BRANCH: String = get_gitflow_branch_name(MAIN);
    static ref PROJECT_NAME: String = get_project_name();
    static ref GITLAB_HOST: String = get_gitlab_host();
    static ref GITLAB_TOKEN: String = get_gitlab_token();
}

#[derive(Parser)]
#[clap(version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create and optionally deploy a release
    Release(ReleaseArgs),

    /// Create and optionally deploy a hotfix from commit hashes
    Hotfix(HotfixArgs),
}

#[derive(Parser)]
struct ReleaseArgs {
    /// Launch a deploy job after the release
    #[clap(long, action)]
    deploy: bool,

    /// Print additional debug information
    #[clap(short, long, action)]
    debug: bool,

    /// Allow to make a release even if the remote is up to date
    #[clap(short, long, action)]
    force: bool,

    /// Define the deploy environment
    #[clap(short, long, value_enum, default_value_t = Environment::Production)]
    environment: Environment,

    /// Define how to increment the version number
    #[clap(short, long, value_enum, default_value_t = SemverType::Patch)]
    semver_type: SemverType,
}

#[derive(Parser)]
struct HotfixArgs {
    /// Commit hashes to cherry-pick onto the hotfix, in order
    #[clap(required = true)]
    commits: Vec<String>,

    /// Launch a deploy job after the hotfix
    #[clap(long, action)]
    deploy: bool,

    /// Print additional debug information
    #[clap(short, long, action)]
    debug: bool,

    /// Unused for hotfix status checks; kept for CLI consistency with release
    #[clap(short, long, action)]
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

fn connect_gitlab() -> Result<Gitlab, Error> {
    info!("[Setup] Login into Gitlab instance \"{}\".", *GITLAB_HOST);
    Gitlab::new(&*GITLAB_HOST, &*GITLAB_TOKEN).map_err(|e| {
        anyhow!(
            "Failed to connect to Gitlab instance \"{}\", with token \"{}\" ({:?})",
            *GITLAB_HOST,
            *GITLAB_TOKEN,
            e
        )
    })
}

fn maybe_deploy(release: &Release<'_>, system: &System<'_>, deploy: bool) -> Result<(), Error> {
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

fn run_release(matches: ReleaseArgs) -> Result<(), Error> {
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

fn run_hotfix(matches: HotfixArgs) -> Result<(), Error> {
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

fn app() -> Result<(), Error> {
    match Cli::parse().command {
        Commands::Release(matches) => run_release(matches),
        Commands::Hotfix(matches) => run_hotfix(matches),
    }
}

fn main() {
    let started = Instant::now();

    process::exit(match app() {
        Ok(_) => {
            info!("Done in {}.", HumanDuration(started.elapsed()));
            0
        }
        Err(err) => {
            error!("{err}");
            1
        }
    });
}
