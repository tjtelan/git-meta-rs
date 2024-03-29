use crate::{
    BranchHeads, GitCommitMeta, GitCredentials, GitRepo, GitRepoCloneRequest, GitRepoInfo,
};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{eyre, Context, ContextCompat, Result};
use git2::{Branch, BranchType, Commit, Cred, Oid, Repository};
use mktemp::Temp;
use tracing::debug;

impl GitRepoInfo {
    pub fn to_repo(&self) -> GitRepo {
        self.into()
    }

    pub fn to_clone(&self) -> GitRepoCloneRequest {
        self.into()
    }

    /// Return the remote name from the given `git2::Repository`
    /// For example, the typical remote name: `origin`
    pub fn get_remote_name(&self, r: &git2::Repository) -> Result<String> {
        let local_branch = r.head().and_then(|h| h.resolve())?;
        let local_branch = local_branch.name();

        if let Some(refname) = local_branch {
            let upstream_remote = r.branch_upstream_remote(refname)?;

            if let Some(name) = upstream_remote.as_str() {
                Ok(name.to_string())
            } else {
                Err(eyre!("Upstream remote name not valid utf-8"))
            }
        } else {
            Err(eyre!("Local branch name not valid utf-8"))
        }
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
        let temp_dir = if let Ok(temp_dir) = Temp::new_dir() {
            temp_dir
        } else {
            return Err(eyre!("Unable to create temp directory"));
        };

        // Check on path. If it doesn't exist, then we gotta clone and open the repo
        // so we can have a git2::Repository to work with
        let repo = if let Some(p) = self.path.clone() {
            GitRepo::to_repository_from_path(p)?
        } else {
            // Shallow clone

            let clone: GitRepoCloneRequest = self.into();
            clone
                .git_clone_shallow(temp_dir.as_path())?
                .to_repository()?
        };

        let cb = self.build_git2_remotecallback();

        let remote_name = if let Ok(name) = self.get_remote_name(&repo) {
            name
        } else {
            return Err(eyre!("Could not read remote name from git2::Repository"));
        };

        let mut remote = if let Ok(r) = repo.find_remote(&remote_name) {
            r
        } else if let Ok(anon_remote) = repo.remote_anonymous(&remote_name) {
            anon_remote
        } else {
            return Err(eyre!(
                "Could not create anonymous remote from: {:?}",
                &remote_name
            ));
        };

        // Connect to the remote and call the printing function for each of the
        // remote references.
        let connection =
            if let Ok(conn) = remote.connect_auth(git2::Direction::Fetch, Some(cb?), None) {
                conn
            } else {
                return Err(eyre!("Unable to connect to git repo"));
            };

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
                .with_message(commit.message().map(|m| m.to_string()));

            ref_map.insert(branch_name, head_commit);
        }

        Ok(ref_map)
    }

    /// Returns a `bool` if a commit exists in the branch using the `git2` crate
    pub fn is_commit_in_branch<'repo>(
        r: &'repo Repository,
        commit: &Commit,
        branch: &Branch,
    ) -> Result<bool> {
        let branch_head = branch.get().peel_to_commit();

        if branch_head.is_err() {
            return Ok(false);
        }

        let branch_head = branch_head.wrap_err("Unable to extract branch HEAD commit")?;
        if branch_head.id() == commit.id() {
            return Ok(true);
        }

        // We get here if we're not working with HEAD commits, and we gotta dig deeper

        let check_commit_in_branch = r.graph_descendant_of(branch_head.id(), commit.id());
        //println!("is {:?} a decendent of {:?}: {:?}", &commit.id(), &branch_head.id(), is_commit_in_branch);

        if check_commit_in_branch.is_err() {
            return Ok(false);
        }

        check_commit_in_branch.wrap_err("Unable to determine if commit exists within branch")
    }

    /// Return the `git2::Branch` struct for a local repo (as opposed to a remote repo)
    /// If `local_branch` is not provided, we'll select the current active branch, based on HEAD
    pub fn get_git2_branch<'repo>(
        r: &'repo Repository,
        local_branch: &Option<String>,
    ) -> Result<Option<Branch<'repo>>> {
        match local_branch {
            Some(branch) => {
                //println!("User passed branch: {:?}", branch);
                if let Ok(git2_branch) = r.find_branch(branch, BranchType::Local) {
                    debug!("Returning given branch: {:?}", &git2_branch.name());
                    Ok(Some(git2_branch))
                } else {
                    // If detached HEAD, we won't have a branch
                    Ok(None)
                }
            }
            None => {
                // Getting the HEAD of the current
                let head = r.head();

                // Find the current local branch...
                let local_branch = Branch::wrap(head?);

                debug!("Returning HEAD branch: {:?}", local_branch.name()?);

                let maybe_local_branch_name = if let Ok(Some(name)) = local_branch.name() {
                    Some(name)
                } else {
                    // This occurs when you check out commit (i.e., detached HEAD).
                    None
                };

                if let Some(local_branch_name) = maybe_local_branch_name {
                    match r.find_branch(local_branch_name, BranchType::Local) {
                        Ok(b) => Ok(Some(b)),
                        Err(_e) => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Return the remote url from the given Repository
    ///
    /// Returns `None` if current branch is local only
    pub fn remote_url_from_repository(r: &Repository) -> Result<Option<String>> {
        // Get the name of the remote from the Repository
        let remote_name = GitRepoInfo::remote_name_from_repository(r)?;

        if let Some(remote) = remote_name {
            let remote_url: String = if let Some(url) = r.find_remote(&remote)?.url() {
                url.chars().collect()
            } else {
                return Err(eyre!("Unable to extract repo url from remote"));
            };

            Ok(Some(remote_url))
        } else {
            Ok(None)
        }
    }

    /// Return the remote name from the given Repository
    fn remote_name_from_repository(r: &Repository) -> Result<Option<String>> {
        let local_branch = r.head().and_then(|h| h.resolve())?;
        let local_branch_name = if let Some(name) = local_branch.name() {
            name
        } else {
            return Err(eyre!("Local branch name is not valid utf-8"));
        };

        let upstream_remote_name_buf =
            if let Ok(remote) = r.branch_upstream_remote(local_branch_name) {
                Some(remote)
            } else {
                //return Err(eyre!("Could not retrieve remote name from local branch"));
                None
            };

        if let Some(remote) = upstream_remote_name_buf {
            let remote_name = if let Some(name) = remote.as_str() {
                Some(name.to_string())
            } else {
                return Err(eyre!("Remote name not valid utf-8"));
            };

            debug!("Remote name: {:?}", &remote_name);

            Ok(remote_name)
        } else {
            Ok(None)
        }
    }

    /// Returns the remote url after opening and validating repo from the local path
    pub fn git_remote_from_path(path: &Path) -> Result<Option<String>> {
        let r = GitRepo::to_repository_from_path(path)?;
        GitRepoInfo::remote_url_from_repository(&r)
    }

    /// Returns the remote url from the `git2::Repository` struct
    pub fn git_remote_from_repo(local_repo: &Repository) -> Result<Option<String>> {
        GitRepoInfo::remote_url_from_repository(local_repo)
    }

    /// Returns a `Result<Option<Vec<PathBuf>>>` containing files changed between `commit1` and `commit2`
    pub fn list_files_changed_between<S: AsRef<str>>(
        &self,
        commit1: S,
        commit2: S,
    ) -> Result<Option<Vec<PathBuf>>> {
        let repo = self.to_repo();

        let commit1 = self.expand_partial_commit_id(commit1.as_ref())?;
        let commit2 = self.expand_partial_commit_id(commit2.as_ref())?;

        let repo = repo.to_repository()?;

        let oid1 = Oid::from_str(&commit1)?;
        let oid2 = Oid::from_str(&commit2)?;

        let git2_commit1 = repo.find_commit(oid1)?.tree()?;
        let git2_commit2 = repo.find_commit(oid2)?.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&git2_commit1), Some(&git2_commit2), None)?;

        let mut paths = Vec::new();

        diff.print(git2::DiffFormat::NameOnly, |delta, _hunk, _line| {
            let delta_path = if let Some(p) = delta.new_file().path() {
                p
            } else {
                return false;
            };

            paths.push(delta_path.to_path_buf());
            true
        })
        .wrap_err("File path not found in new commit to compare")?;

        if !paths.is_empty() {
            return Ok(Some(paths));
        }

        Ok(None)
    }

    /// Returns a `Result<Option<Vec<PathBuf>>>` containing files changed between `commit` and `commit~1` (the previous commit)
    pub fn list_files_changed_at<S: AsRef<str>>(&self, commit: S) -> Result<Option<Vec<PathBuf>>> {
        let repo = self.to_repo();

        let commit = self.expand_partial_commit_id(commit.as_ref())?;

        let git2_repo = repo.to_repository()?;

        let oid = Oid::from_str(&commit)?;
        let git2_commit = git2_repo.find_commit(oid)?;

        let mut changed_files = Vec::new();

        for parent in git2_commit.parents() {
            let parent_commit_id = hex::encode(parent.id().as_bytes());

            if let Some(path_vec) = self.list_files_changed_between(&parent_commit_id, &commit)? {
                for p in path_vec {
                    changed_files.push(p);
                }
            }
        }

        if !changed_files.is_empty() {
            Ok(Some(changed_files))
        } else {
            Ok(None)
        }
    }

    /// Takes in a partial commit SHA-1, and attempts to expand to the full 40-char commit id
    pub fn expand_partial_commit_id<S: AsRef<str>>(&self, partial_commit_id: S) -> Result<String> {
        let repo: GitRepo = self.to_repo();

        // Don't need to do anything if the commit is already complete
        // I guess the only issue is not validating it exists. Is that ok?
        if partial_commit_id.as_ref().len() == 40 {
            return Ok(partial_commit_id.as_ref().to_string());
        }

        // We can't reliably succeed if repo is a shallow clone
        if repo.to_repository()?.is_shallow() {
            return Err(eyre!(
                "No support for partial commit id expand on shallow clones"
            ));
        }

        let repo = repo.to_repository()?;

        let extended_commit = hex::encode(
            repo.revparse_single(partial_commit_id.as_ref())?
                .peel_to_commit()?
                .id()
                .as_bytes(),
        );

        Ok(extended_commit)
    }

    /// Checks the list of files changed between last 2 commits (`HEAD` and `HEAD~1`).
    /// Returns `bool` depending on whether any changes were made in `path`.
    /// A `path` should be relative to the repo root. Can be a file or a directory.
    pub fn has_path_changed<P: AsRef<Path>>(&self, path: P) -> Result<bool> {
        let repo = self.to_repo();
        let git2_repo = repo.to_repository().wrap_err("Could not open repo")?;

        // Get `HEAD~1` commit
        // This could actually be multiple parent commits, if merge commit
        let head = git2_repo
            .head()
            .wrap_err("Could not get HEAD ref")?
            .peel_to_commit()
            .wrap_err("Could not convert to commit")?;
        let head_commit_id = hex::encode(head.id().as_bytes());
        for commit in head.parents() {
            let parent_commit_id = hex::encode(commit.id().as_bytes());

            if self.has_path_changed_between(&path, &head_commit_id, &parent_commit_id)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Checks the list of files changed between 2 commits (`commit1` and `commit2`).
    /// Returns `bool` depending on whether any changes were made in `path`.
    /// A `path` should be relative to the repo root. Can be a file or a directory.
    pub fn has_path_changed_between<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        commit1: S,
        commit2: S,
    ) -> Result<bool> {
        let commit1 = self
            .expand_partial_commit_id(commit1.as_ref())
            .wrap_err("Could not expand partial commit id for commit1")?;
        let commit2 = self
            .expand_partial_commit_id(commit2.as_ref())
            .wrap_err("Could not expand partial commit id for commit2")?;

        let changed_files = self
            .list_files_changed_between(&commit1, &commit2)
            .wrap_err("Error retrieving commit changes")?;

        if let Some(files) = changed_files {
            for f in files.iter() {
                if f.to_str()
                    .wrap_err("Couldn't convert pathbuf to str")?
                    .starts_with(
                        &path
                            .as_ref()
                            .to_path_buf()
                            .to_str()
                            .wrap_err("Couldn't convert pathbuf to str")?,
                    )
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if new commits exist by performing a shallow clone and comparing branch heads
    pub fn new_commits_exist(&self) -> Result<bool> {
        // Let's do a shallow clone behind the scenes using the same branch and creds
        let repo = if let Ok(gitrepo) = GitRepo::new(self.url.to_string()) {
            let branch = if let Some(branch) = self.branch.clone() {
                branch
            } else {
                return Err(eyre!("No branch set"));
            };

            gitrepo
                .with_branch(Some(branch))
                .with_credentials(self.credentials.clone())
        } else {
            return Err(eyre!("Could not crete new GitUrl"));
        };

        let tempdir = if let Ok(dir) = Temp::new_dir() {
            dir
        } else {
            return Err(eyre!("Could not create temporary dir"));
        };

        // We can do a shallow clone, because we only want the newest history
        let clone: GitRepoCloneRequest = repo.into();
        let repo = if let Ok(gitrepo) = clone.git_clone_shallow(tempdir) {
            gitrepo
        } else {
            return Err(eyre!("Could not shallow clone dir"));
        };

        // If the HEAD commits don't match, we assume that `repo` is newer
        Ok(self.head != repo.head)
    }

    /// Builds a `git2::RemoteCallbacks` using `self.credentials` to be used
    /// in authenticated calls to a remote repo
    pub fn build_git2_remotecallback(&self) -> Result<git2::RemoteCallbacks> {
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
                                let key = if let Ok(key) =
                                    Cred::ssh_key(&username, None, private_key.as_path(), None)
                                {
                                    key
                                } else {
                                    return Err(git2::Error::from_str(
                                        "Could not create credentials object for ssh key",
                                    ));
                                };
                                Ok(key)
                            }
                            (None, Some(pp)) => {
                                let key = if let Ok(key) = Cred::ssh_key(
                                    &username,
                                    None,
                                    private_key.as_path(),
                                    Some(pp.as_ref()),
                                ) {
                                    key
                                } else {
                                    return Err(git2::Error::from_str(
                                        "Could not create credentials object for ssh key",
                                    ));
                                };
                                Ok(key)
                            }
                            (Some(pk), None) => {
                                let key = if let Ok(key) = Cred::ssh_key(
                                    &username,
                                    Some(pk.as_path()),
                                    private_key.as_path(),
                                    None,
                                ) {
                                    key
                                } else {
                                    return Err(git2::Error::from_str(
                                        "Could not create credentials object for ssh key",
                                    ));
                                };
                                Ok(key)
                            }
                            (Some(pk), Some(pp)) => {
                                let key = if let Ok(key) = Cred::ssh_key(
                                    &username,
                                    Some(pk.as_path()),
                                    private_key.as_path(),
                                    Some(pp.as_ref()),
                                ) {
                                    key
                                } else {
                                    return Err(git2::Error::from_str(
                                        "Could not create credentials object for ssh key",
                                    ));
                                };
                                Ok(key)
                            }
                        },
                    );

                    Ok(cb)
                }
                GitCredentials::UserPassPlaintext { username, password } => {
                    let mut cb = git2::RemoteCallbacks::new();
                    cb.credentials(move |_, _, _| {
                        Cred::userpass_plaintext(username.as_str(), password.as_str())
                    });

                    Ok(cb)
                }
            }
        } else {
            // No credentials. Repo is public
            Ok(git2::RemoteCallbacks::new())
        }
    }
}
