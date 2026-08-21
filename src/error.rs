use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum SystemError {
    #[error("\"git\" not found")]
    #[diagnostic(
        code(wr::system::git_not_found),
        help("Install git and ensure it is on PATH.")
    )]
    GitNotFound,

    #[error("\"git-flow\" not found")]
    #[diagnostic(
        code(wr::system::git_flow_not_found),
        help("Install git-flow (on macOS: git-flow-avh).")
    )]
    GitFlowNotFound,

    #[error("Wrong git-flow edition (need AVH)")]
    #[diagnostic(
        code(wr::system::git_flow_wrong_version),
        help("On macOS install git-flow-avh.")
    )]
    GitFlowWrongVersion,

    #[error("git-flow is not initialized")]
    #[diagnostic(
        code(wr::system::git_flow_not_initialized),
        help("Run `git flow init`.")
    )]
    GitFlowNotInitialized,

    #[error("Not on the {branch} branch")]
    #[diagnostic(code(wr::system::wrong_branch), help("Run `git checkout {branch}`."))]
    WrongBranch { branch: String },

    #[error("Upstream for {branch} is not set")]
    #[diagnostic(
        code(wr::system::upstream_not_defined),
        help("git checkout {branch} && git branch --set-upstream-to=origin/{branch} {branch}")
    )]
    UpstreamNotDefined { branch: String },

    #[error("Repository is up-to-date, nothing to do")]
    #[diagnostic(
        code(wr::system::repo_up_to_date),
        help("Pass --force to release anyway.")
    )]
    RepoUpToDate,

    #[error("Local branch is behind remote")]
    #[diagnostic(code(wr::system::repo_need_pull), help("Run `git pull`, then retry."))]
    RepoNeedPull,

    #[error("Local and remote branches have diverged")]
    #[diagnostic(
        code(wr::system::repo_diverged),
        help("Reconcile the conflict, then retry.")
    )]
    RepoDiverged,

    #[error("Repository has uncommitted changes")]
    #[diagnostic(
        code(wr::system::repo_dirty),
        help("Commit or stash before running wr.")
    )]
    RepoDirty,

    #[error(transparent)]
    #[diagnostic(code(wr::system::io))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(wr::system::git2))]
    Git2(#[from] git2::Error),
}

#[derive(Error, Diagnostic, Debug)]
pub enum GitError {
    #[error("Not inside a git repository")]
    #[diagnostic(
        code(wr::git::not_a_repository),
        help("cd into a git repository and retry.")
    )]
    NotARepository,

    #[error(transparent)]
    #[diagnostic(code(wr::git::git2))]
    Git2(#[from] git2::Error),
}

#[derive(Error, Diagnostic, Debug)]
pub enum HotfixError {
    #[error("No commit hashes given")]
    #[diagnostic(code(wr::hotfix::empty_commits), help("Usage: wr hotfix <sha>…"))]
    EmptyCommits,

    #[error("Commit \"{commit}\" not found")]
    #[diagnostic(
        code(wr::hotfix::commit_not_found),
        help("Use a hash that exists in this repository.")
    )]
    CommitNotFound { commit: String },

    #[error("Branch {branch} not found")]
    #[diagnostic(code(wr::hotfix::branch_not_found), help("Create or fetch it first."))]
    BranchNotFound { branch: String },

    #[error("Branch {branch} has no target")]
    #[diagnostic(code(wr::hotfix::branch_no_target))]
    BranchNoTarget { branch: String },

    #[error("Detached HEAD")]
    #[diagnostic(code(wr::hotfix::detached_head), help("Run `git checkout <branch>`."))]
    DetachedHead,

    #[error("Cherry-pick of \"{commit}\" failed")]
    #[diagnostic(
        code(wr::hotfix::cherry_pick_failed),
        help("Resolve conflicts or pick a different commit, then retry.")
    )]
    CherryPickFailed {
        commit: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Cancelling.")]
    #[diagnostic(code(wr::hotfix::cancelled))]
    Cancelled,

    #[error("Aborting.")]
    #[diagnostic(code(wr::hotfix::aborted))]
    Aborted,

    #[error(transparent)]
    #[diagnostic(transparent)]
    Release(#[from] ReleaseError),

    #[error(transparent)]
    #[diagnostic(code(wr::hotfix::io))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(wr::hotfix::git2))]
    Git2(#[from] git2::Error),
}

#[derive(Error, Diagnostic, Debug)]
pub enum ReleaseError {
    #[error("No tag found")]
    #[diagnostic(
        code(wr::release::no_tag),
        help("Create a semver tag, or let the first release start at 1.0.0.")
    )]
    NoTagFound,

    #[error("[Deploy] Pipeline was not found")]
    #[diagnostic(
        code(wr::release::pipeline_not_found),
        help("Check GitLab CI ran for this ref, then retry.")
    )]
    PipelineNotFound,

    #[error("Cancelling.")]
    #[diagnostic(code(wr::release::cancelled))]
    Cancelled,

    #[error("Aborting.")]
    #[diagnostic(code(wr::release::aborted))]
    Aborted,

    #[error("{message}")]
    #[diagnostic(code(wr::release::gitlab))]
    Gitlab { message: String },

    #[error(transparent)]
    #[diagnostic(code(wr::release::io))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(code(wr::release::git2))]
    Git2(#[from] git2::Error),
}

#[derive(Error, Diagnostic, Debug)]
pub enum GitlabError {
    #[error("Failed to connect to Gitlab instance \"{host}\"")]
    #[diagnostic(
        code(wr::gitlab::connect_failed),
        help("Check GITLAB_HOST, GITLAB_TOKEN, and network access.")
    )]
    ConnectFailed {
        host: String,
        #[source]
        source: gitlab::GitlabError,
    },
}
