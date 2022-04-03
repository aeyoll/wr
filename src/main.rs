#[macro_use]
extern crate clap;
use clap::App;

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

fn app() -> Result<(), Error> {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    // Get the logger filter level
    let level = if matches.is_present("debug") {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    // Define the logger
    TermLogger::init(
        level,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    // Set some env variables
    env::set_var("LANG", "en_US.UTF-8");
    env::set_var("GIT_MERGE_AUTOEDIT", "no");

    // Init
    info!("Welcome to wr.");
    let gitlab_host = env::var("WR_GITLAB_HOST").unwrap_or_else(|_| "gitlab.com".to_string());
    let gitlab_token = env::var("WR_GITLAB_TOKEN").unwrap_or_else(|_| "".to_string());

    // Get a git2 "Repository" struct
    let repository = get_repository()?;

    // Run some system checks
    // This will ensure that everything is in place to do the deployment
    let s = System {
        repository: &repository,
    };
    info!("[Setup] Performing system checks.");
    s.system_check()?;

    // Get environment
    debug!("Getting the environment name from the arguments.");
    let environment: Environment = match matches
        .value_of("environment")
        .unwrap_or(&Environment::Production.to_string())
        .parse()
    {
        Ok(environment) => environment,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };
    info!(
        "[Setup] {} environment was found from the arguments.",
        environment
    );

    // Get semver type
    debug!("Getting the semver type from the arguments.");
    let semver_type: SemverType = match matches
        .value_of("semver_type")
        .unwrap_or(&SemverType::Patch.to_string())
        .parse()
    {
        Ok(semver_type) => semver_type,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };
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

    let deploy = matches.is_present("deploy");
    if deploy {
        if s.has_gitlab_ci() {
            debug!("\"deploy\" flag was found, trying to play the \"deploy\" job.");
            release.deploy()?;
        } else {
            warn!("\"deploy\" flag was found, but the repository has no \".gitlab-ci.yml\" file, impossible to deploy.")
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
