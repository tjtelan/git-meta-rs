//! # Git-meta
//!
//! Git-meta is a collection of functionality for gathering information about git repos and commits

//! You can open an existing repo with `GitRepo::open(path)`
//! (Branch and commits provided for example. Provide `None` to use current checked out values)
//!
//! ```ignore
//! use std::path::PathBuf;
//! use git_meta::GitRepo;
//! GitRepo::open(
//!         PathBuf::from("/path/to/repo"),
//!         Some("main".to_string()),
//!         Some("b24fe6112e97eb9ee0cc1fd5aaa520bf8814f6c3".to_string()))
//!     .expect("Unable to clone repo");
//! ```
//!
//! You can create a new repo for cloning with `GitRepo::new(url)`
//!
//! ```ignore
//! use std::path::PathBuf;
//! use git_meta::{GitCredentials, GitRepo};
//! use mktemp::Temp;
//! let temp_dir = Temp::new_dir().expect("Unable to create test clone dir");
//!
//! let creds = GitCredentials::SshKey {
//!     username: "git".to_string(),
//!     public_key: None,
//!     private_key: PathBuf::from("/path/to/private/key"),
//!     passphrase: None,
//! };
//!
//! GitRepo::new("https://github.com/tjtelan/git-meta-rs")
//!     .expect("Unable to create GitRepo")
//!     .with_credentials(Some(creds))
//!     .git_clone_shallow(temp_dir.as_path())
//!     .expect("Unable to clone repo");
//! ```
//!
//! *Note:* Shallow cloning requires `git` CLI to be installed

use chrono::prelude::*;
use color_eyre::eyre::{eyre, Result};
use git2::Cred;
use git2::{Branch, Commit, Repository};
use git_url_parse::GitUrl;
use hex::ToHex;
use log::debug;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[doc(hidden)]
pub mod clone;
#[doc(hidden)]
pub mod info;
#[doc(hidden)]
pub mod types;

// Re-export our types in the root
#[doc(inline)]
pub use crate::types::*;

impl GitCommitMeta {
    /// Trait bound for `id` is to convert the output from:
    /// `git2::Commit.id().as_bytes()` into a `String`
    pub fn new<I: ToHex + AsRef<[u8]>>(id: I) -> GitCommitMeta {
        GitCommitMeta {
            id: hex::encode(id),
            message: None,
            timestamp: None,
        }
    }

    /// `time` is intended to convert output from:
    /// `git2::Commit.time().seconds()` into `Datetime<Utc>`
    pub fn with_timestamp(mut self, time: i64) -> Self {
        let naive_datetime = NaiveDateTime::from_timestamp(time, 0);
        let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);

        self.timestamp = Some(datetime);
        self
    }

    /// Set the commit message
    pub fn with_message(mut self, msg: Option<String>) -> Self {
        self.message = msg;
        self
    }
}

impl From<Repository> for GitRepo {
    /// Convert from `git2::Repository` to `GitRepo`.
    fn from(repo: Repository) -> Self {
        GitRepo::open(repo.path().to_path_buf(), None, None)
            .expect("Failed to convert Repository to GitRepo")
    }
}

impl GitRepo {
    /// Returns a `GitRepo` after parsing metadata from a repo
    /// - If a local `branch` is not provided, current checked out branch will be used.
    ///   The provided branch will be resolved to its remote branch name
    /// - If `commit_id` is not provided, the current commit (the HEAD of `branch`) will be used
    pub fn open(
        path: PathBuf,
        branch: Option<String>,
        commit_id: Option<String>,
    ) -> Result<GitRepo> {
        // First we open the repository and get the remote_url and parse it into components
        let local_repo = GitRepo::to_repository_from_path(path.clone())?;
        let remote_url = GitRepo::git_remote_from_repo(&local_repo)?;

        let working_branch_name = GitRepo::get_git2_branch(&local_repo, &branch)?
            .name()?
            .expect("Unable to extract branch name")
            .to_string();

        // We don't support digging around in past commits if the repo is shallow
        if let Some(_c) = &commit_id {
            if local_repo.is_shallow() {
                return Err(eyre!("Can't open by commit on shallow clones"));
            }
        }

        let commit =
            GitRepo::get_git2_commit(&local_repo, &Some(working_branch_name.clone()), &commit_id)?;

        Ok(GitRepo::new(remote_url)?
            .with_path(path)
            .with_branch(Some(working_branch_name))
            .with_git2_commit(commit))
    }

    /// Set the location of `GitRepo` on the filesystem
    pub fn with_path(mut self, path: PathBuf) -> Self {
        // We want to get the absolute path of the directory of the repo
        self.path = Some(fs::canonicalize(path).expect("Directory was not found"));
        self
    }

    /// Intended to be set with the remote name branch of GitRepo
    pub fn with_branch(mut self, branch: Option<String>) -> Self {
        if let Some(b) = branch {
            self.branch = Some(b.into());
        }
        self
    }

    /// Reinit `GitRepo` with commit id
    pub fn with_commit(mut self, commit_id: Option<String>) -> Self {
        self = GitRepo::open(self.path.expect("No path set"), self.branch, commit_id)
            .expect("Unable to open GitRepo with commit id");
        self
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
    pub fn new<S: AsRef<str>>(url: S) -> Result<GitRepo> {
        Ok(GitRepo {
            url: GitUrl::parse(url.as_ref()).expect("url failed to parse as GitUrl"),
            credentials: None,
            head: None,
            branch: None,
            path: None,
        })
    }

    /// Returns a `git2::Repository` from `self.path`
    pub fn to_repository(&self) -> Result<Repository, git2::Error> {
        GitRepo::to_repository_from_path(
            self.path.clone().expect("No path set to open").as_os_str(),
        )
    }

    /// Returns a `git2::Repository` from a given repo directory path
    fn to_repository_from_path<P: AsRef<Path>>(path: P) -> Result<Repository, git2::Error> {
        Repository::open(path.as_ref().as_os_str())
    }

    /// Return a `git2::Commit` that refers to the commit object requested for building
    /// If commit id is not provided, then we'll use the HEAD commit of whatever branch is active or provided
    fn get_git2_commit<'repo>(
        r: &'repo Repository,
        branch: &Option<String>,
        commit_id: &Option<String>,
    ) -> Result<Option<Commit<'repo>>> {
        let working_branch = GitRepo::get_git2_branch(r, branch)?;

        match commit_id {
            Some(id) => {
                debug!("Commit provided. Using {}", id);
                let commit = r.find_commit(git2::Oid::from_str(id)?)?;

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

                match working_branch.upstream() {
                    Ok(upstream_branch) => {
                        let working_ref = upstream_branch.into_reference();

                        let commit = working_ref
                            .peel_to_commit()
                            .expect("Unable to retrieve HEAD commit object from remote branch");

                        let _ =
                            GitRepo::is_commit_in_branch(r, &commit, &Branch::wrap(working_ref));

                        Ok(Some(commit))
                    }
                    // This match-arm supports branches that are local-only
                    Err(_e) => {
                        debug!("No remote branch found. Using HEAD commit from local branch");
                        let working_ref = working_branch.into_reference();

                        let commit = working_ref
                            .peel_to_commit()
                            .expect("Unable to retrieve HEAD commit object from local branch");

                        let _ =
                            GitRepo::is_commit_in_branch(r, &commit, &Branch::wrap(working_ref));

                        Ok(Some(commit))
                    }
                }
            }
        }
    }

    /// Builds a `git2::RemoteCallbacks` using `self.credentials` to be used
    /// in authenticated calls to a remote repo
    fn build_git2_remotecallback(&self) -> git2::RemoteCallbacks {
        if let Some(cred) = self.credentials.clone() {
            debug!("Before building callback: {:?}", &cred);

            match cred {
                GitCredentials::SshKey {
                    username,
                    public_key,
                    private_key,
                    passphrase,
                } => {
                    let mut cb = git2::RemoteCallbacks::new();

                    cb.credentials(
                        move |_, _, _| match (public_key.clone(), passphrase.clone()) {
                            (None, None) => {
                                Ok(Cred::ssh_key(&username, None, private_key.as_path(), None)
                                    .expect("Could not create credentials object for ssh key"))
                            }
                            (None, Some(pp)) => Ok(Cred::ssh_key(
                                &username,
                                None,
                                private_key.as_path(),
                                Some(pp.as_ref()),
                            )
                            .expect("Could not create credentials object for ssh key")),
                            (Some(pk), None) => Ok(Cred::ssh_key(
                                &username,
                                Some(pk.as_path()),
                                private_key.as_path(),
                                None,
                            )
                            .expect("Could not create credentials object for ssh key")),
                            (Some(pk), Some(pp)) => Ok(Cred::ssh_key(
                                &username,
                                Some(pk.as_path()),
                                private_key.as_path(),
                                Some(pp.as_ref()),
                            )
                            .expect("Could not create credentials object for ssh key")),
                        },
                    );

                    cb
                }
                GitCredentials::UserPassPlaintext { username, password } => {
                    let mut cb = git2::RemoteCallbacks::new();
                    cb.credentials(move |_, _, _| {
                        Cred::userpass_plaintext(username.as_str(), password.as_str())
                    });

                    cb
                }
            }
        } else {
            // No credentials. Repo is public
            git2::RemoteCallbacks::new()
        }
    }

    /// Test whether `GitRepo` is a shallow clone
    pub fn is_shallow(&self) -> bool {
        let repo = self.to_repository().expect("Could not read repo");
        repo.is_shallow()
    }
}
