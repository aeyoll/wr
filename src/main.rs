#[macro_use]
extern crate clap;
use clap::App;

use anyhow::{anyhow, Error};

#[macro_use]
extern crate log;
extern crate simplelog;

#[macro_use]
extern crate lazy_static;

use git2::Repository;
use simplelog::*;

use std::env;
use std::process;

use gitlab::Gitlab;

mod system;
use system::System;

mod environment;
use environment::Environment;

mod semver_type;
use semver_type::SemverType;

mod release;
use release::Release;

use crate::git::get_gitflow_branch_name;

mod git;

const DEVELOP: &str = "develop";
const MASTER: &str = "master";

lazy_static! {
    static ref DEVELOP_BRANCH: String = get_gitflow_branch_name(DEVELOP);
    static ref MASTER_BRANCH: String = get_gitflow_branch_name(MASTER);
}

fn app() -> Result<(), Error> {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    // Define the logger
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    // Set some env variables
    env::set_var("LANG", "en_US.UTF-8");
    env::set_var("GIT_MERGE_AUTOEDIT", "no");

    // Init
    info!("Welcome to wr");
    let gitlab_host = env::var("WR_GITLAB_HOST").unwrap_or_else(|_| "gitlab.com".to_string());
    let gitlab_token = env::var("WR_GITLAB_TOKEN").unwrap_or_else(|_| "".to_string());

    // Run some system checks
    let s = System;
    s.system_check()?;

    // Get environment
    let environment: Environment = match matches
        .value_of("environment")
        .unwrap_or(&Environment::Production.to_string())
        .parse()
    {
        Ok(environment) => environment,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };

    // Get semver type
    let semver_type: SemverType = match matches
        .value_of("semver_type")
        .unwrap_or(&SemverType::Patch.to_string())
        .parse()
    {
        Ok(semver_type) => semver_type,
        Err(e) => return Err(anyhow!("{}", e.to_string())),
    };

    let deploy = matches.is_present("deploy");

    let gitlab = match Gitlab::new(&gitlab_host, &gitlab_token) {
        Ok(client) => client,
        Err(_) => {
            return Err(anyhow!(
                "Failed to connect to {} with token {}",
                &gitlab_host,
                &gitlab_token
            ))
        }
    };

    let current_dir = env::current_dir().unwrap();
    let repository = match Repository::open(current_dir) {
        Ok(repo) => repo,
        Err(e) => return Err(anyhow!("Failed to open: {}", e)),
    };

    let release = Release {
        gitlab,
        repository,
        environment,
        semver_type,
    };
    release.create()?;

    if deploy {
        release.deploy()?;
    }

    Ok(())
}

fn main() {
    process::exit(match app() {
        Ok(_) => 0,
        Err(err) => {
            error!("{}", err.to_string());
            1
        }
    });
}
