use crate::{DEVELOP_BRANCH, MASTER_BRANCH};
use std::{fmt, str::FromStr};

const DEPLOY_PROD_JOB: &str = "deploy_prod";
const DEPLOY_STAGING_JOB: &str = "deploy_staging";

#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum Environment {
    #[default]
    Production,
    Staging,
}

impl Environment {
    /// Get the deploy job name for the environment
    pub fn get_deploy_job_name(&self) -> &'static str {
        match self {
            Environment::Production => DEPLOY_PROD_JOB,
            Environment::Staging => DEPLOY_STAGING_JOB,
        }
    }

    /// Get the pipeline ref for the environment
    pub fn get_pipeline_ref(&self) -> &str {
        match self {
            Environment::Production => &MASTER_BRANCH,
            Environment::Staging => &DEVELOP_BRANCH,
        }
    }
}

/// Convert a string to an environment
impl FromStr for Environment {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Production" => Ok(Environment::Production),
            "Staging" => Ok(Environment::Staging),
            _ => Err("Unknown environment"),
        }
    }
}

/// Display the environment as a string
impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
