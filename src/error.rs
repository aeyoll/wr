use miette::{Diagnostic, Result};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum WrError {
    #[error("Git is not installed")]
    #[diagnostic(
        code(wr::setup::git_missing),
        help("Please install git using your package manager:\n  - macOS: brew install git\n  - Ubuntu: sudo apt install git\n  - Windows: https://git-scm.com/download/win")
    )]
    GitNotFound,

    #[error("Git Flow is not installed")]
    #[diagnostic(
        code(wr::setup::gitflow_missing),
        help("Please install git-flow-avh:\n  - macOS: brew install git-flow-avh\n  - Ubuntu: sudo apt install git-flow\n  - Manual: https://github.com/petervanderdoes/gitflow-avh"),
        url("https://github.com/petervanderdoes/gitflow-avh/wiki/Installation")
    )]
    GitFlowNotFound,

    #[error("Wrong git-flow version detected")]
    #[diagnostic(
        code(wr::setup::gitflow_wrong_version),
        help("You need git-flow-avh, not the original git-flow.\nUninstall the current version and install git-flow-avh instead.")
    )]
    GitFlowWrongVersion,

    #[error("Repository is not initialized with git-flow")]
    #[diagnostic(
        code(wr::repo::gitflow_not_init),
        help("Run 'git flow init' to initialize git-flow in this repository")
    )]
    GitFlowNotInitialized,

    #[error("Repository is dirty")]
    #[diagnostic(
        code(wr::repo::dirty),
        help("Please commit or stash your changes before running wr:\n  git add .\n  git commit -m \"Your message\"\n\nOr stash them:\n  git stash")
    )]
    RepositoryDirty,

    #[error("Repository is up-to-date")]
    #[diagnostic(
        code(wr::repo::up_to_date),
        help("Use --force flag to create a release anyway"),
        severity(Warning)
    )]
    RepositoryUpToDate,

    #[error("Repository needs to be pulled first")]
    #[diagnostic(
        code(wr::repo::need_pull),
        help("Run 'git pull' to update your local repository with remote changes")
    )]
    RepositoryNeedPull,

    #[error("Branch have diverged")]
    #[diagnostic(
        code(wr::repo::diverged),
        help("Please fix the conflict first by merging or rebasing the branches")
    )]
    RepositoryDiverged,

    #[error("Not in a git repository")]
    #[diagnostic(
        code(wr::repo::not_git),
        help("Please run wr from within a git repository")
    )]
    NotInGitRepository,

    #[error("Please checkout the {branch} branch")]
    #[diagnostic(
        code(wr::repo::wrong_branch),
        help("Switch to the correct branch using: git checkout {branch}")
    )]
    WrongBranch { branch: String },

    #[error("Failed to connect to GitLab instance \"{host}\"")]
    #[diagnostic(
        code(wr::gitlab::connection_failed),
        help("Check your GitLab configuration:\n  1. Verify GITLAB_HOST environment variable\n  2. Verify GITLAB_TOKEN environment variable\n  3. Ensure your token has the required permissions")
    )]
    GitlabConnectionFailed {
        host: String,
        token: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("No tag found")]
    #[diagnostic(
        code(wr::release::no_tag),
        help("Create an initial tag first: git tag v0.1.0")
    )]
    NoTagFound,

    #[error("Pipeline was not found")]
    #[diagnostic(
        code(wr::deploy::pipeline_not_found),
        help("Check if the GitLab CI/CD pipeline is properly configured")
    )]
    PipelineNotFound,

    #[error("Cancelling operation")]
    #[diagnostic(code(wr::user::cancelled), severity(Warning))]
    UserCancelled,

    #[error("Operation aborted")]
    #[diagnostic(code(wr::user::aborted), severity(Error))]
    UserAborted,

    #[error("Command execution failed")]
    #[diagnostic(code(wr::system::command_failed))]
    CommandFailed {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Git operation failed")]
    #[diagnostic(code(wr::git::operation_failed))]
    GitOperationFailed {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("External error")]
    #[diagnostic(code(wr::external))]
    External {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

// Implement From traits for automatic error conversion
impl From<git2::Error> for WrError {
    fn from(error: git2::Error) -> Self {
        WrError::GitOperationFailed {
            source: Box::new(error),
        }
    }
}

impl From<std::io::Error> for WrError {
    fn from(error: std::io::Error) -> Self {
        WrError::CommandFailed {
            source: Box::new(error),
        }
    }
}

// duct errors are already std::io::Error, so we don't need a separate impl

impl From<gitlab::api::ApiError<gitlab::RestError>> for WrError {
    fn from(error: gitlab::api::ApiError<gitlab::RestError>) -> Self {
        WrError::External {
            source: Box::new(error),
        }
    }
}

impl From<gitlab::GitlabError> for WrError {
    fn from(error: gitlab::GitlabError) -> Self {
        WrError::External {
            source: Box::new(error),
        }
    }
}

// Helper trait for command context
pub trait IntoWrError<T> {
    fn with_command_context(self) -> Result<T, WrError>;
    fn with_git_context(self) -> Result<T, WrError>;
}

impl<T, E> IntoWrError<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_command_context(self) -> Result<T, WrError> {
        self.map_err(|e| WrError::CommandFailed {
            source: Box::new(e),
        })
    }

    fn with_git_context(self) -> Result<T, WrError> {
        self.map_err(|e| WrError::GitOperationFailed {
            source: Box::new(e),
        })
    }
}
