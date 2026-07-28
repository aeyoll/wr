use anyhow::{anyhow, Context, Error};
use dialoguer::{theme::ColorfulTheme, Confirm};
use duct::cmd;
use git2::{Oid, Repository};
use semver::Version;
use std::path::Path;

use crate::{release::Release, DEVELOP_BRANCH, MAIN_BRANCH};

struct Snapshot {
    main_oid: Oid,
    develop_oid: Oid,
    previous_branch: String,
    tag: String,
}

pub struct Hotfix<'a> {
    pub repository: &'a Repository,
    pub commits: &'a [String],
}

impl Hotfix<'_> {
    fn hotfix_branch(tag: &str) -> String {
        format!("hotfix/{tag}")
    }

    fn branch_oid(&self, branch: &str) -> Result<Oid, Error> {
        let reference = self
            .repository
            .find_reference(&format!("refs/heads/{branch}"))
            .with_context(|| format!("Branch {branch} not found"))?;

        reference
            .target()
            .ok_or_else(|| anyhow!("Branch {branch} has no target"))
    }

    fn current_branch(&self) -> Result<String, Error> {
        let head = self.repository.head()?;
        head.shorthand()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("Detached HEAD; checkout a branch first"))
    }

    fn snapshot(&self, tag: &str) -> Result<Snapshot, Error> {
        Ok(Snapshot {
            main_oid: self.branch_oid(&MAIN_BRANCH)?,
            develop_oid: self.branch_oid(&DEVELOP_BRANCH)?,
            previous_branch: self.current_branch()?,
            tag: tag.to_string(),
        })
    }

    fn validate_commits(&self) -> Result<(), Error> {
        for commit in self.commits {
            self.repository
                .revparse_single(commit)
                .with_context(|| format!("Commit \"{commit}\" not found"))?;
        }

        Ok(())
    }

    /// Restore local main/develop/tag/hotfix branch to pre-hotfix state.
    fn rollback(&self, snapshot: &Snapshot) {
        let workdir = self
            .repository
            .workdir()
            .expect("repository has no workdir");
        rollback_local(snapshot, &MAIN_BRANCH, &DEVELOP_BRANCH, workdir);
    }

    fn create_inner(&self, tag: &str) -> Result<(), Error> {
        info!("[Hotfix] Creating hotfix {tag}.");
        cmd!("git", "flow", "hotfix", "start", tag)
            .stdout_capture()
            .stderr_capture()
            .read()?;

        for commit in self.commits {
            info!("[Hotfix] Cherry-picking {commit}.");
            cmd!("git", "cherry-pick", commit)
                .stdout_capture()
                .stderr_capture()
                .read()
                .with_context(|| format!("Cherry-pick of \"{commit}\" failed"))?;
        }

        cmd!("git", "flow", "hotfix", "finish", "-m", tag, tag)
            .stdout_capture()
            .stderr_capture()
            .read()?;

        cmd!("git", "checkout", DEVELOP_BRANCH.to_string())
            .stdout_capture()
            .stderr_capture()
            .read()?;

        Ok(())
    }

    /// Create a hotfix named after the next patch tag, cherry-picking commits in order.
    pub fn create(&self, release: &Release<'_>) -> Result<Version, Error> {
        if self.commits.is_empty() {
            return Err(anyhow!("At least one commit hash is required."));
        }

        self.validate_commits()?;

        let next_tag = release.get_next_tag()?;
        let tag = next_tag.to_string();

        info!("[Hotfix] This will create hotfix tag {tag}.");

        match Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to continue?")
            .interact_opt()
            .unwrap()
        {
            Some(true) => {}
            Some(false) => return Err(anyhow!("Cancelling.")),
            None => return Err(anyhow!("Aborting.")),
        }

        let snapshot = self.snapshot(&tag)?;

        if let Err(err) = self.create_inner(&tag) {
            self.rollback(&snapshot);
            return Err(err);
        }

        Ok(next_tag)
    }
}

fn rollback_local(snapshot: &Snapshot, main: &str, develop: &str, workdir: &Path) {
    warn!("[Hotfix] Rolling back local changes.");

    let _ = cmd!("git", "cherry-pick", "--abort")
        .dir(workdir)
        .stdout_capture()
        .stderr_capture()
        .run();

    let hotfix_branch = Hotfix::hotfix_branch(&snapshot.tag);

    // Leave the hotfix branch before deleting it.
    let _ = cmd!("git", "checkout", &snapshot.previous_branch)
        .dir(workdir)
        .stdout_capture()
        .stderr_capture()
        .run();

    let _ = cmd!("git", "branch", "-D", &hotfix_branch)
        .dir(workdir)
        .stdout_capture()
        .stderr_capture()
        .run();

    let _ = cmd!("git", "tag", "-d", &snapshot.tag)
        .dir(workdir)
        .stdout_capture()
        .stderr_capture()
        .run();

    let _ = cmd!("git", "branch", "-f", main, snapshot.main_oid.to_string())
        .dir(workdir)
        .stdout_capture()
        .stderr_capture()
        .run();

    let _ = cmd!(
        "git",
        "branch",
        "-f",
        develop,
        snapshot.develop_oid.to_string()
    )
    .dir(workdir)
    .stdout_capture()
    .stderr_capture()
    .run();

    let _ = cmd!("git", "checkout", &snapshot.previous_branch)
        .dir(workdir)
        .stdout_capture()
        .stderr_capture()
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use duct::cmd;
    use git2::Signature;
    use tempfile::TempDir;

    fn create_repo_with_branches() -> (TempDir, Repository, String) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(temp_dir.path()).expect("Failed to init repo");

        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let oid = {
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap()
        };

        // Rename default branch to main for git-flow-like layout in tests.
        let default_branch = {
            let head = repo.head().unwrap();
            head.shorthand().unwrap().to_string()
        };
        if default_branch != "main" {
            repo.branch("main", &repo.find_commit(oid).unwrap(), true)
                .unwrap();
        }
        repo.branch("develop", &repo.find_commit(oid).unwrap(), true)
            .unwrap();

        // Second commit for cherry-pick target.
        {
            std::fs::write(temp_dir.path().join("fix.txt"), "fix").unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(std::path::Path::new("fix.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.find_commit(oid).unwrap();
            repo.commit(
                Some("refs/heads/develop"),
                &sig,
                &sig,
                "Fix commit",
                &tree,
                &[&parent],
            )
            .unwrap();
        }

        let fix_oid = repo
            .find_reference("refs/heads/develop")
            .unwrap()
            .target()
            .unwrap()
            .to_string();

        (temp_dir, repo, fix_oid)
    }

    #[test]
    fn validate_commits_rejects_unknown_hash() {
        let (_temp_dir, repo, _) = create_repo_with_branches();
        let commits = vec!["deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string()];
        let hotfix = Hotfix {
            repository: &repo,
            commits: &commits,
        };

        let result = hotfix.validate_commits();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn validate_commits_accepts_existing_hash() {
        let (_temp_dir, repo, fix_oid) = create_repo_with_branches();
        let commits = vec![fix_oid];
        let hotfix = Hotfix {
            repository: &repo,
            commits: &commits,
        };

        assert!(hotfix.validate_commits().is_ok());
    }

    #[test]
    fn rollback_restores_branch_oids_and_deletes_tag() {
        let (temp_dir, repo, _) = create_repo_with_branches();
        let workdir = temp_dir.path();

        let main_oid = repo
            .find_reference("refs/heads/main")
            .unwrap()
            .target()
            .unwrap();
        let develop_oid = repo
            .find_reference("refs/heads/develop")
            .unwrap()
            .target()
            .unwrap();

        let snapshot = Snapshot {
            main_oid,
            develop_oid,
            previous_branch: "main".to_string(),
            tag: "9.9.9".to_string(),
        };

        // Simulate a failed hotfix leaving a branch + tag.
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let develop_commit = repo.find_commit(develop_oid).unwrap();
        repo.branch("hotfix/9.9.9", &develop_commit, true).unwrap();
        repo.tag("9.9.9", develop_commit.as_object(), &sig, "9.9.9", false)
            .unwrap();

        // Checkout main so branch -D can delete the hotfix branch.
        cmd!("git", "checkout", "main")
            .dir(workdir)
            .stdout_capture()
            .stderr_capture()
            .run()
            .unwrap();

        rollback_local(&snapshot, "main", "develop", workdir);

        assert!(repo.find_reference("refs/heads/hotfix/9.9.9").is_err());
        assert!(repo.find_reference("refs/tags/9.9.9").is_err());
        assert_eq!(
            repo.find_reference("refs/heads/main")
                .unwrap()
                .target()
                .unwrap(),
            main_oid
        );
        assert_eq!(
            repo.find_reference("refs/heads/develop")
                .unwrap()
                .target()
                .unwrap(),
            develop_oid
        );
    }

    #[test]
    fn hotfix_branch_name_uses_tag() {
        assert_eq!(Hotfix::hotfix_branch("1.2.4"), "hotfix/1.2.4");
    }
}
