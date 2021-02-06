use chrono::prelude::*;
use color_eyre::eyre::Result;
use git2::Cred;
use git2::{Branch, BranchType, Commit, ObjectType, Repository};
use git_url_parse::GitUrl;
use hex::ToHex;
use log::debug;
use std::path::Path;
use std::path::PathBuf;

pub mod clone;
pub mod info;

#[derive(Clone, Debug)]
pub enum GitCredentials {
    SshKey {
        username: String,
        public_key: Option<PathBuf>,
        private_key: PathBuf,
        passphrase: Option<String>,
    },
    UserPassPlaintext {
        username: String,
        password: String,
    },
}

#[derive(Clone, Debug)]
pub struct GitRepoCloner {
    pub url: GitUrl,
    pub credentials: Option<GitCredentials>,
    pub branch: Option<String>,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Default)]
pub struct GitRepo {
    pub url: GitUrl,
    pub head: Option<GitCommitMeta>,
    pub credentials: Option<GitCredentials>,
    pub branch: Option<String>,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitCommitMeta {
    pub id: String,
    pub message: Option<String>,
    pub epoch_time: Option<DateTime<Utc>>,
}

impl GitCommitMeta {
    /// Trait bound for `id` is to convert the output from:
    /// `git2::Commit.id().as_bytes()` into a `String`
    pub fn new<I: ToHex + AsRef<[u8]>>(id: I) -> GitCommitMeta {
        GitCommitMeta {
            id: hex::encode(id),
            message: None,
            epoch_time: None,
        }
    }

    /// `time` is intended to convert output from:
    /// `git2::Commit.time().seconds() into `Datetime<Utc>`
    pub fn with_timestamp(mut self, time: i64) -> Self {
        let naive_datetime = NaiveDateTime::from_timestamp(time, 0);
        let datetime: DateTime<Utc> = DateTime::from_utc(naive_datetime, Utc);

        self.epoch_time = Some(datetime);
        self
    }

    pub fn with_message(mut self, msg: Option<String>) -> Self {
        self.message = msg;
        self
    }
}

impl GitRepo {
    /// Returns a `git2::Repository` from a given repo directory path
    fn get_local_repo_from_path<P: AsRef<Path>>(path: P) -> Result<Repository, git2::Error> {
        Repository::open(path.as_ref().as_os_str())
    }

    /// Return the remote url from the given Repository
    fn _get_remote_url<'repo>(r: &'repo Repository) -> Result<String> {
        // Get the name of the remote from the Repository
        let remote_name = GitRepo::_get_remote_name(&r)?;

        let remote_url: String = r
            .find_remote(&remote_name)?
            .url()
            .expect("Unable to extract repo url from remote")
            .chars()
            .collect();

        Ok(remote_url)
    }

    /// Return the remote name from the given Repository
    fn _get_remote_name<'repo>(r: &'repo Repository) -> Result<String> {
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
        let r = GitRepo::get_local_repo_from_path(path)?;
        GitRepo::_get_remote_url(&r)
    }

    /// Returns the remote url from the `git2::Repository` struct
    fn git_remote_from_repo(local_repo: &Repository) -> Result<String> {
        GitRepo::_get_remote_url(&local_repo)
    }

    /// Return the `git2::Branch` struct for a local repo (as opposed to a remote repo)
    /// If `local_branch` is not provided, we'll select the current active branch, based on HEAD
    fn get_working_branch<'repo>(
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

    /// Returns a `bool` if the `git2::Commit` is a descendent of the `git2::Branch`
    fn is_commit_in_branch<'repo>(r: &'repo Repository, commit: &Commit, branch: &Branch) -> bool {
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

    // TODO: Verify if commit is not in branch, that we'll end up in detached HEAD
    /// Return a `git2::Commit` that refers to the commit object requested for building
    /// If commit id is not provided, then we'll use the HEAD commit of whatever branch is active or provided
    fn get_target_commit<'repo>(
        r: &'repo Repository,
        branch: &Option<String>,
        commit_id: &Option<String>,
    ) -> Result<Commit<'repo>> {
        let working_branch = GitRepo::get_working_branch(r, branch)?;

        match commit_id {
            Some(id) => {
                let working_ref = working_branch.into_reference();

                debug!("Commit provided. Using {}", id);
                let oid = git2::Oid::from_str(id)?;

                let obj = r.find_object(oid, ObjectType::from_str("commit"))?;
                let commit = obj
                    .into_commit()
                    .expect("Unable to convert commit id into commit object");

                let _ = GitRepo::is_commit_in_branch(r, &commit, &Branch::wrap(working_ref));

                Ok(commit)
            }

            // We want the HEAD of the remote branch (as opposed to the working branch)
            None => {
                debug!("No commit provided. Using HEAD commit from remote branch");

                let upstream_branch = working_branch.upstream()?;
                let working_ref = upstream_branch.into_reference();

                let commit = working_ref
                    .peel_to_commit()
                    .expect("Unable to retrieve HEAD commit object from remote branch");

                let _ = GitRepo::is_commit_in_branch(r, &commit, &Branch::wrap(working_ref));

                Ok(commit)
            }
        }
    }

    /// Returns a `GitRepo` after parsing metadata from a repo
    /// If branch is not provided, current checked out branch will be used
    /// If commit id is not provided, the HEAD of the branch will be used
    pub fn open(
        path: PathBuf,
        branch: Option<String>,
        commit_id: Option<String>,
    ) -> Result<GitRepo> {
        // First we open the repository and get the remote_url and parse it into components
        let local_repo = GitRepo::get_local_repo_from_path(path.clone())?;
        let remote_url = GitRepo::git_remote_from_repo(&local_repo)?;

        let working_branch_name = GitRepo::get_working_branch(&local_repo, &branch)?
            .name()?
            .expect("Unable to extract branch name")
            .to_string();

        let commit = GitRepo::get_target_commit(
            &local_repo,
            &Some(working_branch_name.clone()),
            &commit_id,
        )?;

        Ok(GitRepo::new(remote_url)?
            .with_path(path)
            .with_branch(working_branch_name)
            .with_commit(commit))
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn with_branch<S: Into<String>>(mut self, branch: S) -> Self {
        self.branch = Some(branch.into());
        self
    }

    pub fn with_commit(mut self, commit: Commit) -> Self {
        let commit_msg = commit.clone().message().unwrap_or_default().to_string();

        let commit = GitCommitMeta::new(commit.id())
            .with_message(Some(commit_msg))
            .with_timestamp(commit.time().seconds());

        self.head = Some(commit);
        self
    }

    pub fn with_credentials(mut self, creds: GitCredentials) -> Self {
        self.credentials = Some(creds);
        self
    }

    pub fn new<S: AsRef<str>>(url: S) -> Result<GitRepo> {
        Ok(GitRepo {
            url: GitUrl::parse(url.as_ref()).expect("url failed to parse as GitUrl"),
            credentials: None,
            head: None,
            branch: None,
            path: None,
        })
    }

    pub fn build_git2_remotecallback(&self) -> git2::RemoteCallbacks {
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
