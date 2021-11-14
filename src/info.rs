use crate::{BranchHeads, GitCommitMeta, GitRepo};

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{eyre, Result};
use git2::{Branch, BranchType, Commit, Oid, Repository};
use log::debug;
use mktemp::Temp;

impl GitRepo {
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
            self.git_clone_shallow(temp_dir.as_path())?
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
        } else {
            if let Ok(anon_remote) = repo.remote_anonymous(&remote_name) {
                anon_remote
            } else {
                return Err(eyre!(
                    "Could not create anonymous remote from: {:?}",
                    &remote_name
                ));
            }
        };

        // Connect to the remote and call the printing function for each of the
        // remote references.
        let connection =
            if let Ok(conn) = remote.connect_auth(git2::Direction::Fetch, Some(cb), None) {
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
                let b = r.find_branch(branch, BranchType::Local)?;
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

                let local_branch_name = if let Ok(Some(name)) = local_branch.name() {
                    name
                } else {
                    return Err(eyre!("Unable to return local branch name"));
                };

                // Convert git2::Error to Error
                match r.find_branch(local_branch_name, BranchType::Local) {
                    Ok(b) => Ok(b),
                    Err(e) => Err(e.into()),
                }
            }
        }
    }

    /// Return the remote url from the given Repository
    pub fn remote_url_from_repository(r: &Repository) -> Result<String> {
        // Get the name of the remote from the Repository
        let remote_name = GitRepo::remote_name_from_repository(r)?;

        let remote_url: String = if let Some(url) = r.find_remote(&remote_name)?.url() {
            url.chars().collect()
        } else {
            return Err(eyre!("Unable to extract repo url from remote"));
        };

        Ok(remote_url)
    }

    /// Return the remote name from the given Repository
    fn remote_name_from_repository(r: &Repository) -> Result<String> {
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
        GitRepo::remote_url_from_repository(local_repo)
    }

    /// Returns a `Result<Option<Vec<PathBuf>>>` containing files changed between `commit1` and `commit2`
    pub fn list_files_changed_between<S: AsRef<str>>(
        &self,
        commit1: S,
        commit2: S,
    ) -> Result<Option<Vec<PathBuf>>> {
        let repo = self.to_repository()?;

        let commit1 = self.expand_partial_commit_id(commit1.as_ref())?;
        let commit2 = self.expand_partial_commit_id(commit2.as_ref())?;

        let oid1 = Oid::from_str(&commit1)?;
        let oid2 = Oid::from_str(&commit2)?;

        let git2_commit1 = repo.find_commit(oid1)?.tree()?;
        let git2_commit2 = repo.find_commit(oid2)?.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&git2_commit1), Some(&git2_commit2), None)?;

        let mut paths = Vec::new();

        diff.print(git2::DiffFormat::NameOnly, |delta, _hunk, _line| {
            paths.push(
                delta
                    .new_file()
                    .path()
                    .expect("Expected the new file path")
                    .to_path_buf(),
            );
            //let f = delta.new_file().path().unwrap().display();
            //println!("{:?}", f );
            true
        })?;

        if !paths.is_empty() {
            return Ok(Some(paths));
        }

        Ok(None)
    }

    /// Returns a `Result<Option<Vec<PathBuf>>>` containing files changed between `commit` and `commit~1` (the previous commit)
    pub fn list_files_changed_at<S: AsRef<str>>(&self, commit: S) -> Result<Option<Vec<PathBuf>>> {
        let repo = self.to_repository()?;

        let commit = self.expand_partial_commit_id(commit.as_ref())?;

        let oid = Oid::from_str(&commit)?;
        let git2_commit = repo.find_commit(oid)?;

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
        // Don't need to do anything if the commit is already complete
        // I guess the only issue is not validating it exists. Is that ok?
        if partial_commit_id.as_ref().len() == 40 {
            return Ok(partial_commit_id.as_ref().to_string());
        }

        // We can't reliably succeed if repo is a shallow clone
        if self.to_repository()?.is_shallow() {
            return Err(eyre!(
                "No support for partial commit id expand on shallow clones"
            ));
        }

        let repo = self.to_repository()?;

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
    pub fn has_path_changed<P: AsRef<Path>>(&self, path: P) -> bool {
        let repo = self.to_repository().expect("Could not open repo");

        // Get `HEAD~1` commit
        // This could actually be multiple parent commits, if merge commit
        let head = repo
            .head()
            .expect("Could not get HEAD ref")
            .peel_to_commit()
            .expect("Could not convert to commit");
        let head_commit_id = hex::encode(head.id().as_bytes());
        for commit in head.parents() {
            let parent_commit_id = hex::encode(commit.id().as_bytes());

            if self.has_path_changed_between(&path, &head_commit_id, &parent_commit_id) {
                return true;
            }
        }

        false
    }

    /// Checks the list of files changed between 2 commits (`commit1` and `commit2`).
    /// Returns `bool` depending on whether any changes were made in `path`.
    /// A `path` should be relative to the repo root. Can be a file or a directory.
    pub fn has_path_changed_between<P: AsRef<Path>, S: AsRef<str>>(
        &self,
        path: P,
        commit1: S,
        commit2: S,
    ) -> bool {
        let commit1 = self
            .expand_partial_commit_id(commit1.as_ref())
            .expect("Could not expand partial id");
        let commit2 = self
            .expand_partial_commit_id(commit2.as_ref())
            .expect("Could not expand partial id");

        let changed_files = self
            .list_files_changed_between(&commit1, &commit2)
            .expect("Error retrieving commit changes");

        if let Some(files) = changed_files {
            for f in files.iter() {
                if f.to_str()
                    .expect("Couldn't convert pathbuf to str")
                    .starts_with(
                        &path
                            .as_ref()
                            .to_path_buf()
                            .to_str()
                            .expect("Couldn't convert pathbuf to str"),
                    )
                {
                    return true;
                }
            }
        }

        false
    }

    /// Check if new commits exist by performing a shallow clone and comparing branch heads
    pub fn new_commits_exist(&self) -> bool {
        // Let's do a shallow clone behind the scenes using the same branch and creds
        let repo = GitRepo::new(self.url.to_string())
            .expect("Could not crete new GitUrl")
            .with_branch(Some(self.branch.clone().expect("No branch set")))
            .with_credentials(self.credentials.clone());

        let tempdir = Temp::new_dir().expect("Could not create temporary dir");

        // We can do a shallow clone, because we only want the newest history
        let repo = repo
            .git_clone_shallow(tempdir)
            .expect("Could not shallow clone dir");

        // If the HEAD commits don't match, we assume that `repo` is newer
        self.head != repo.head
    }
}
