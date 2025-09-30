use clap::Parser;

#[macro_use]
extern crate log;
extern crate simplelog;

#[macro_use]
extern crate lazy_static;

use indicatif::HumanDuration;
use simplelog::*;

use std::env;
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

use crate::git::{
    get_gitflow_branch_name, get_gitlab_host, get_gitlab_token, get_project_name, get_repository,
};

mod error;
use error::WrError;

mod git;
mod repository_status;

const DEVELOP: &str = "develop";
const MASTER: &str = "master";

lazy_static! {
    static ref DEVELOP_BRANCH: String = get_gitflow_branch_name(DEVELOP);
    static ref MASTER_BRANCH: String = get_gitflow_branch_name(MASTER);
    static ref PROJECT_NAME: String = get_project_name();
    static ref GITLAB_HOST: String = get_gitlab_host();
    static ref GITLAB_TOKEN: String = get_gitlab_token();
}

#[derive(Parser)]
#[clap(version, about, long_about = None)]
struct Cli {
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

fn app() -> Result<(), WrError> {
    let matches = Cli::parse();

    // Get the logger filter level
    let level = if matches.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let force = matches.force;

    let mut log_stdout_config_builder = ConfigBuilder::default();
    log_stdout_config_builder
        .set_time_offset_to_local()
        .unwrap();

    // Define the logger
    TermLogger::init(
        level,
        log_stdout_config_builder.build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    // Set some env variables
    env::set_var("LANG", "en_US.UTF-8");
    env::set_var("GIT_MERGE_AUTOEDIT", "no");

    // Init
    info!("Welcome to wr.");

    // Get a git2 "Repository" struct
    let repository = get_repository()?;

    // Run some system checks
    // This will ensure that everything is in place to do the deployment
    let s = System {
        repository: &repository,
        force,
    };
    info!("[Setup] Performing system checks.");
    s.system_check()?;

    // Get environment
    debug!("Getting the environment name from the arguments.");
    let environment: Environment = matches.environment;
    info!("[Setup] {environment} environment was found from the arguments.");

    // Get semver type
    debug!("Getting the semver type from the arguments.");
    let semver_type: SemverType = matches.semver_type;
    info!("[Setup] {semver_type} semver type was found from the arguments.");

    info!("[Setup] Login into Gitlab instance \"{}\".", *GITLAB_HOST);
    let gitlab = Gitlab::new(&*GITLAB_HOST, &*GITLAB_TOKEN).map_err(|e| {
        WrError::GitlabConnectionFailed {
            host: GITLAB_HOST.clone(),
            token: GITLAB_TOKEN.clone(),
            source: Box::new(e),
        }
    })?;

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

    if matches.deploy {
        if s.has_gitlab_ci() {
            debug!("\"deploy\" flag was found, trying to play the \"deploy\" job.");
            release.deploy()?;
        } else {
            warn!("\"deploy\" flag was found, but the repository has no \".gitlab-ci.yml\" file, impossible to deploy.")
        }
    }

    Ok(())
}

fn main() -> miette::Result<()> {
    let started = Instant::now();

    // Convert our WrError to miette::Report for proper formatting
    if let Err(err) = app() {
        return Err(miette::Report::new(err));
    }

    info!("Done in {}.", HumanDuration(started.elapsed()));
    Ok(())
}
