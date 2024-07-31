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

use crate::git::get_project_name;
use crate::git::{get_gitflow_branch_name, get_repository};

mod git;
mod repository_status;

const DEVELOP: &str = "develop";
const MASTER: &str = "master";

lazy_static! {
    static ref DEVELOP_BRANCH: String = get_gitflow_branch_name(DEVELOP);
    static ref MASTER_BRANCH: String = get_gitflow_branch_name(MASTER);
    static ref PROJECT_NAME: String = get_project_name();
}

#[derive(Parser)]
#[clap(version, about, long_about = None)]
struct Cli {
    /// Launch a deployment job after the release
    #[clap(long, action)]
    deploy: bool,

    /// Print additional debug information
    #[clap(short, long, action)]
    debug: bool,

    /// Allow to make a release even if the remote is up-to-date
    #[clap(short, long, action)]
    force: bool,

    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Create a new release")]
    Release {
        /// Define the deploy environment
        #[clap(short, long, value_enum, default_value_t = Environment::Production)]
        environment: Environment,

        /// Define how to increment the version number
        #[clap(short, long, value_enum, default_value_t = SemverType::Patch)]
        semver_type: SemverType,
    },
}

fn app() -> Result<(), Error> {
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
    let gitlab_host = env::var("GITLAB_HOST").unwrap_or_else(|_| "gitlab.com".to_string());
    let gitlab_token = env::var("GITLAB_TOKEN").unwrap_or_else(|_| "".to_string());

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

    match matches.commands {
        Commands::Release {
            environment,
            semver_type,
        } => {
            // Get environment
            debug!("Getting the environment name from the arguments.");
            let environment: Environment = environment;
            info!(
                "[Setup] {} environment was found from the arguments.",
                environment
            );

            // Get semver type
            debug!("Getting the semver type from the arguments.");
            let semver_type: SemverType = semver_type;
            info!(
                "[Setup] {} semver type was found from the arguments.",
                semver_type
            );

            info!("[Setup] Login into Gitlab instance \"{}\".", gitlab_host);
            let gitlab = match Gitlab::new(&gitlab_host, &gitlab_token) {
                Ok(client) => client,
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to connect to Gitlab instance \"{}\", with token \"{}\" ({:?})",
                        &gitlab_host,
                        &gitlab_token,
                        e
                    ))
                }
            };

            let release = Release {
                gitlab,
                repository: &repository,
                environment,
                semver_type,
            };

            debug!("[Release] Creating a new {} release.", environment);
            release.create()?;
            info!("[Release] A new {} release has been created.", environment);

            debug!(
                "[Release] Pushing the {} release to the remote repository.",
                environment
            );
            release.push()?;
            info!(
                "[Release] {} release has been pushed to the remote repository.",
                environment
            );

            if matches.deploy {
                if s.has_gitlab_ci() {
                    debug!("\"deploy\" flag was found, trying to play the \"deploy\" job.");
                    release.deploy()?;
                } else {
                    warn!("\"deploy\" flag was found, but the repository has no \".gitlab-ci.yml\" file, impossible to deploy.")
                }
            }
        }
    }

    Ok(())
}

fn main() {
    let started = Instant::now();

    process::exit(match app() {
        Ok(_) => {
            info!("Done in {}.", HumanDuration(started.elapsed()));
            0
        }
        Err(err) => {
            error!("{}", err.to_string());
            1
        }
    });
}
