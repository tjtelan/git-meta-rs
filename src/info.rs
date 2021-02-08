use crate::{BranchHeads, GitCommitMeta, GitRepo};

use std::collections::HashMap;
use std::path::Path;

use color_eyre::eyre::Result;
use git2::{Branch, BranchType, Commit, Repository};
use log::debug;
use mktemp::Temp;

impl GitRepo {
    /// Return the remote name from the given `git2::Repository`
    /// For example, the typical remote name: `origin`
    pub fn get_remote_name(&self, r: &git2::Repository) -> Result<String> {
        let remote_name = r
            .branch_upstream_remote(
                r.head()
                    .and_then(|h| h.resolve())?
                    .name()
                    .expect("branch name is valid utf8"),
            )
            .map(|b| b.as_str().expect("valid utf8").to_string())
            .unwrap_or_else(|_| "origin".into());

        Ok(remote_name)
    }

    /// Return a `HashMap<String, GitCommitMeta>` for a branch containing
    /// the branch names and the latest commit of the branch`.
    /// Providing a `branch_filter` will only return branches based on
    /// patterns matching the start of the branch name.
    pub fn get_remote_branch_head_refs(
        &self,
        branch_filter: Option<Vec<String>>,
    ) -> Result<BranchHeads> {
        // Create a temp directory (In case we need to clone)
        let temp_dir = Temp::new_dir().unwrap();

        // Check on path. If it doesn't exist, then we gotta clone and open the repo
        // so we can have a git2::Repository to work with
        let repo = if let Some(p) = self.path.clone() {
            GitRepo::to_repository_from_path(p.clone())?
        } else {
            // Shallow clone
            self.git_clone_shallow(temp_dir.as_path())?
                .to_repository()?
        };

        let cb = self.build_git2_remotecallback();

        let remote = self
            .get_remote_name(&repo)
            .expect("Could not read remote name from git2::Repository");

        let mut remote = repo
            .find_remote(&remote)
            .or_else(|_| repo.remote_anonymous(&remote))
            .unwrap();

        // Connect to the remote and call the printing function for each of the
        // remote references.
        let connection = remote
            .connect_auth(git2::Direction::Fetch, Some(cb), None)
            .expect("Unable to connect to git repo");

        let git_branch_ref_prefix = "refs/heads/";
        let mut ref_map: HashMap<String, GitCommitMeta> = HashMap::new();

        for git_ref in connection
            .list()?
            .iter()
            .filter(|head| head.name().starts_with(git_branch_ref_prefix))
        {
            let branch_name = git_ref
                .name()
                .to_string()
                .rsplit(git_branch_ref_prefix)
                .collect::<Vec<&str>>()[0]
                .to_string();

            if let Some(ref branches) = branch_filter {
                if branches.contains(&branch_name.to_string()) {
                    continue;
                }
            }

            // Get the commit object
            let commit = repo.find_commit(git_ref.oid())?;

            let head_commit = GitCommitMeta::new(commit.id().as_bytes())
                .with_timestamp(commit.time().seconds())
                .with_message(commit.message().map_or(None, |m| Some(m.to_string())));

            ref_map.insert(branch_name, head_commit);
        }

        Ok(ref_map)
    }

    /// Returns a `bool` if a commit exists in the branch using the `git2` crate
    pub fn is_commit_in_branch<'repo>(
        r: &'repo Repository,
        commit: &Commit,
        branch: &Branch,
    ) -> bool {
        let branch_head = branch.get().peel_to_commit();

        if branch_head.is_err() {
            return false;
        }

        let branch_head = branch_head.expect("Unable to extract branch HEAD commit");
        if branch_head.id() == commit.id() {
            return true;
        }

        // We get here if we're not working with HEAD commits, and we gotta dig deeper

        let check_commit_in_branch = r.graph_descendant_of(branch_head.id(), commit.id());
        //println!("is {:?} a decendent of {:?}: {:?}", &commit.id(), &branch_head.id(), is_commit_in_branch);

        if check_commit_in_branch.is_err() {
            return false;
        }

        check_commit_in_branch.expect("Unable to determine if commit exists within branch")
    }

    /// Return the `git2::Branch` struct for a local repo (as opposed to a remote repo)
    /// If `local_branch` is not provided, we'll select the current active branch, based on HEAD
    pub fn get_git2_branch<'repo>(
        r: &'repo Repository,
        local_branch: &Option<String>,
    ) -> Result<Branch<'repo>> {
        match local_branch {
            Some(branch) => {
                //println!("User passed branch: {:?}", branch);
                let b = r.find_branch(&branch, BranchType::Local)?;
                debug!("Returning given branch: {:?}", &b.name());
                Ok(b)
            }
            None => {
                // Getting the HEAD of the current
                let head = r.head();
                //let commit = head.unwrap().peel_to_commit();
                //println!("{:?}", commit);

                // Find the current local branch...
                let local_branch = Branch::wrap(head?);

                debug!("Returning HEAD branch: {:?}", local_branch.name()?);

                // Convert git2::Error to anyhow::Error
                match r.find_branch(
                    local_branch
                        .name()?
                        .expect("Unable to return local branch name"),
                    BranchType::Local,
                ) {
                    Ok(b) => Ok(b),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    /// Return the remote url from the given Repository
    pub fn remote_url_from_repository<'repo>(r: &'repo Repository) -> Result<String> {
        // Get the name of the remote from the Repository
        let remote_name = GitRepo::remote_name_from_repository(&r)?;

        let remote_url: String = r
            .find_remote(&remote_name)?
            .url()
            .expect("Unable to extract repo url from remote")
            .chars()
            .collect();

        Ok(remote_url)
    }

    /// Return the remote name from the given Repository
    fn remote_name_from_repository<'repo>(r: &'repo Repository) -> Result<String> {
        let remote_name = r
            .branch_upstream_remote(
                r.head()
                    .and_then(|h| h.resolve())?
                    .name()
                    .expect("branch name is valid utf8"),
            )
            .map(|b| b.as_str().expect("valid utf8").to_string())
            .unwrap_or_else(|_| "origin".into());

        debug!("Remote name: {:?}", &remote_name);

        Ok(remote_name)
    }

    /// Returns the remote url after opening and validating repo from the local path
    pub fn git_remote_from_path(path: &Path) -> Result<String> {
        let r = GitRepo::to_repository_from_path(path)?;
        GitRepo::remote_url_from_repository(&r)
    }

    /// Returns the remote url from the `git2::Repository` struct
    pub fn git_remote_from_repo(local_repo: &Repository) -> Result<String> {
        GitRepo::remote_url_from_repository(&local_repo)
    }

    // TODO: check for if there are new commits
    // pub fn new_commits_exist(&self) -> bool {}
}
