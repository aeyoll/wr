use crate::{DEVELOP_BRANCH, MAIN_BRANCH};
use std::fmt;
use std::str::FromStr;

const DEPLOY_PROD_JOB: &str = "deploy_prod";
const DEPLOY_STAGING_JOB: &str = "deploy_staging";

#[derive(Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum Environment {
    #[default]
    Production,
    Staging,
    /// Production release, then play deploy_prod and deploy_staging.
    Both,
}

impl Environment {
    /// Deploy job names, in the order they are played.
    pub fn deploy_jobs(&self) -> &'static [&'static str] {
        match self {
            Environment::Production => &[DEPLOY_PROD_JOB],
            Environment::Staging => &[DEPLOY_STAGING_JOB],
            Environment::Both => &[DEPLOY_PROD_JOB, DEPLOY_STAGING_JOB],
        }
    }

    /// Branch whose pipeline contains `job_name`.
    pub fn pipeline_ref_for_job(job_name: &str) -> &str {
        if job_name == DEPLOY_STAGING_JOB {
            &DEVELOP_BRANCH
        } else {
            &MAIN_BRANCH
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
            "Both" => Ok(Environment::Both),
            _ => Err("Unknown environment"),
        }
    }
}

/// Display the environment as a string
impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_environment_is_production() {
        assert_eq!(Environment::default(), Environment::Production);
    }

    #[test]
    fn deploy_jobs_lists_prod_then_staging_for_both() {
        assert_eq!(Environment::Production.deploy_jobs(), ["deploy_prod"]);
        assert_eq!(Environment::Staging.deploy_jobs(), ["deploy_staging"]);
        assert_eq!(
            Environment::Both.deploy_jobs(),
            ["deploy_prod", "deploy_staging"]
        );
    }

    #[test]
    fn from_str_parses_and_rejects() {
        assert_eq!(
            "Production".parse::<Environment>().unwrap(),
            Environment::Production
        );
        assert_eq!(
            "Staging".parse::<Environment>().unwrap(),
            Environment::Staging
        );
        assert_eq!("Both".parse::<Environment>().unwrap(), Environment::Both);
        assert_eq!(
            "Invalid".parse::<Environment>().unwrap_err(),
            "Unknown environment"
        );
        assert!("production".parse::<Environment>().is_err());
    }

    #[test]
    fn display_formatting() {
        assert_eq!(format!("{}", Environment::Production), "Production");
        assert_eq!(format!("{}", Environment::Staging), "Staging");
        assert_eq!(format!("{}", Environment::Both), "Both");
    }
}
