use crate::{DEVELOP_BRANCH, MASTER_BRANCH};
use anyhow::Error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum Environment {
    Production,
    Staging,
}

impl Environment {
    ///
    pub fn get_deploy_job_name(&self) -> Result<String, Error> {
        let job_name = match self {
            Environment::Production => "deploy_prod".to_string(),
            Environment::Staging => "deploy_staging".to_string(),
        };

        Ok(job_name)
    }

    ///
    pub fn get_pipeline_ref(&self) -> Result<String, Error> {
        let pipeline_ref = match self {
            Environment::Production => MASTER_BRANCH.to_string(),
            Environment::Staging => DEVELOP_BRANCH.to_string(),
        };

        Ok(pipeline_ref)
    }
}

impl Default for Environment {
    fn default() -> Environment {
        Environment::Production
    }
}

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

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
