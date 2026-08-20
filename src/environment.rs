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
            Environment::Production => &MAIN_BRANCH,
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
    fn get_deploy_job_name_returns_correct_names() {
        assert_eq!(Environment::Production.get_deploy_job_name(), "deploy_prod");
        assert_eq!(Environment::Staging.get_deploy_job_name(), "deploy_staging");
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
    }
}
