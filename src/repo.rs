use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{GitCommitMeta, GitCredentials, GitRepo, GitRepoCloneRequest, GitRepoInfo};
use git_url_parse::GitUrl;

use git2::{Branch, Commit, Repository};

use color_eyre::eyre::{eyre, Result};
use tracing::debug;

impl GitRepo {
    /// Returns a `GitRepo` after parsing metadata from a repo
    /// - If a local `branch` is not provided, current checked out branch will be used.
    ///   The provided branch will be resolved to its remote branch name
    /// - If `commit_id` is not provided, the current commit (the HEAD of `branch`) will be used
    pub fn open(path: PathBuf, branch: Option<String>, commit_id: Option<String>) -> Result<Self> {
        // First we open the repository and get the remote_url and parse it into components
        let local_repo = Self::to_repository_from_path(path.clone())?;
        let remote_url = GitRepoInfo::git_remote_from_repo(&local_repo)?;

        // Resolve the remote branch name, if possible
        let working_branch_name =
            if let Ok(Some(git2_branch)) = GitRepoInfo::get_git2_branch(&local_repo, &branch) {
                git2_branch.name()?.map(str::to_string)
            } else {
                // Detached HEAD
                None
            };

        // We don't support digging around in past commits if the repo is shallow
        if let Some(_c) = &commit_id {
            if local_repo.is_shallow() {
                return Err(eyre!("Can't open by commit on shallow clones"));
            }
        }

        // This is essential for when we're in Detatched HEAD
        let commit = Self::get_git2_commit(&local_repo, &working_branch_name, &commit_id)?;

        if let Some(url) = remote_url {
            Ok(Self::new(url)?
                .with_path(path)?
                .with_branch(working_branch_name)
                .with_git2_commit(commit))
        } else {
            // Use this when the current branch has no remote ref
            let file_path = path.as_os_str().to_str().unwrap_or_default();
            Ok(Self::new(file_path)?
                .with_path(path)?
                .with_branch(working_branch_name)
                .with_git2_commit(commit))
        }
    }

    /// Set the location of `GitRepo` on the filesystem
    pub fn with_path(mut self, path: PathBuf) -> Result<Self> {
        // We want to get the absolute path of the directory of the repo
        self.path = if let Ok(p) = fs::canonicalize(path) {
            Some(p)
        } else {
            return Err(eyre!("Directory was not found"));
        };
        Ok(self)
    }

    /// Intended to be set with the remote name branch of GitRepo
    pub fn with_branch(mut self, branch: Option<String>) -> Self {
        if let Some(b) = branch {
            self.branch = Some(b);
        }
        self
    }

    /// Reinit `GitRepo` with commit id
    pub fn with_commit(mut self, commit_id: Option<String>) -> Result<Self> {
        self = if let Some(path) = self.path {
            if let Ok(repo) = Self::open(path, self.branch, commit_id) {
                repo
            } else {
                return Err(eyre!("Unable to open GitRepo with commit id"));
            }
        } else {
            return Err(eyre!("No path to GitRepo set"));
        };
        Ok(self)
    }

    /// Set the `GitCommitMeta` from `git2::Commit`
    pub fn with_git2_commit(mut self, commit: Option<Commit>) -> Self {
        match commit {
            Some(c) => {
                let commit_msg = c.message().unwrap_or_default().to_string();

                let commit = GitCommitMeta::new(c.id())
                    .with_message(Some(commit_msg))
                    .with_timestamp(c.time().seconds());

                self.head = Some(commit);
                self
            }
            None => {
                self.head = None;
                self
            }
        }
    }

    /// Set `GitCredentials` for private repos.
    /// `None` indicates public repo
    pub fn with_credentials(mut self, creds: Option<GitCredentials>) -> Self {
        self.credentials = creds;
        self
    }

    /// Create a new `GitRepo` with `url`.
    /// Use along with `with_*` methods to set other fields of `GitRepo`.
    /// Use `GitRepoCloner` if you need to clone the repo, and convert back with `GitRepo.into()`
    pub fn new<S: AsRef<str>>(url: S) -> Result<Self> {
        let url = if let Ok(url) = GitUrl::parse(url.as_ref()) {
            url
        } else {
            return Err(eyre!("url failed to parse as GitUrl"));
        };

        Ok(Self {
            url,
            credentials: None,
            head: None,
            branch: None,
            path: None,
        })
    }

    pub fn to_clone(&self) -> GitRepoCloneRequest {
        self.into()
    }

    pub fn to_info(&self) -> GitRepoInfo {
        self.into()
    }

    /// Returns a `git2::Repository` from `self.path`
    pub fn to_repository(&self) -> Result<Repository> {
        if let Some(path) = self.path.as_ref() {
            Ok(Self::to_repository_from_path(path.as_os_str())?)
        } else {
            Err(eyre!("No path set to open"))
        }
    }

    /// Returns a `git2::Repository` from a given repo directory path
    pub fn to_repository_from_path<P: AsRef<Path> + Debug>(path: P) -> Result<Repository> {
        if let Ok(repo) = Repository::open(path.as_ref().as_os_str()) {
            Ok(repo)
        } else {
            Err(eyre!("Failed to open repo at {path:#?}"))
        }
    }

    /// Return a `git2::Commit` that refers to the commit object requested for building
    /// If commit id is not provided, then we'll use the HEAD commit of whatever branch is active or provided
    fn get_git2_commit<'repo>(
        r: &'repo Repository,
        branch: &Option<String>,
        commit_id: &Option<String>,
    ) -> Result<Option<Commit<'repo>>> {
        // If branch or commit not given, return the HEAD of `r`
        if let (None, None) = (branch, commit_id) {
            // Do I need to verify that we're in detached head?
            // if r.head_detached()? {}

            if let Ok(commit) = r.head()?.peel_to_commit() {
                return Ok(Some(commit));
            } else {
                return Err(eyre!(
                    "Unable to retrieve HEAD commit object from remote branch"
                ));
            }
        }

        match commit_id {
            Some(id) => {
                debug!("Commit provided. Using {}", id);
                let commit = r.find_commit(git2::Oid::from_str(id)?)?;

                // TODO: Verify if the commit is in the branch. If not, return Ok(None)
                // Do we care about detatched HEAD?
                //let _ = GitRepo::is_commit_in_branch(
                //    r,
                //    &commit,
                //    &Branch::wrap(working_branch.into_reference()),
                //);

                Ok(Some(commit))
            }

            // We want the HEAD of the remote branch (as opposed to the working branch)
            None => {
                debug!("No commit provided. Attempting to use HEAD commit from remote branch");

                if branch.is_some() {
                    if let Ok(Some(git2_branch)) = GitRepoInfo::get_git2_branch(r, branch) {
                        match git2_branch.upstream() {
                            Ok(upstream_branch) => {
                                let working_ref = upstream_branch.into_reference();

                                let commit = if let Ok(commit) = working_ref.peel_to_commit() {
                                    commit
                                } else {
                                    return Err(eyre!(
                                        "Unable to retrieve HEAD commit object from remote branch"
                                    ));
                                };

                                let _ = GitRepoInfo::is_commit_in_branch(
                                    r,
                                    &commit,
                                    &Branch::wrap(working_ref),
                                );

                                Ok(Some(commit))
                            }
                            // This match-arm supports branches that are local-only
                            Err(_e) => {
                                debug!(
                                    "No remote branch found. Using HEAD commit from local branch"
                                );
                                let working_ref = git2_branch.into_reference();

                                let commit = if let Ok(commit) = working_ref.peel_to_commit() {
                                    commit
                                } else {
                                    return Err(eyre!(
                                        "Unable to retrieve HEAD commit object from remote branch"
                                    ));
                                };

                                let _ = GitRepoInfo::is_commit_in_branch(
                                    r,
                                    &commit,
                                    &Branch::wrap(working_ref),
                                );

                                Ok(Some(commit))
                            }
                        }
                    } else {
                        // This happens if the branch doesn't exist. Should this be Err()?
                        Ok(None)
                    }
                } else {
                    unreachable!("We should have returned Err() early if both commit and branch not provided. We need one.")
                }
            }
        }
    }

    /// Test whether `GitRepo` is a shallow clone
    pub fn is_shallow(&self) -> Result<bool> {
        let repo = self.to_repository()?;
        Ok(repo.is_shallow())
    }
}
