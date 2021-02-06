use crate::GitRepo;
use crate::{GitCommitMeta, GitRepoCloner};
use color_eyre::eyre::Result;
use mktemp::Temp;
use std::collections::HashMap;

impl GitRepo {
    /// Return the remote name from the given Repository
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

    pub fn get_remote_branch_head_refs(
        &self,
        //repo: git2::Repository,
        branch_filter: Option<Vec<String>>,
    ) -> Result<HashMap<String, GitCommitMeta>> {
        // Create a temp directory (In case we need to clone)
        let temp_dir = Temp::new_dir().unwrap();

        // Check on path. If it doesn't exist, then we gotta clone and open the repo
        // so we can have a git2::Repository to work with
        let repo = if let Some(p) = self.path.clone() {
            GitRepo::get_local_repo_from_path(p.clone())?
        } else {
            // FIXME: Hacking with the type system to do a clone
            let cloner: GitRepoCloner = self.clone().into();

            // Shallow clone
            cloner.git_clone_shallow(temp_dir.as_path())?
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
            .unwrap();

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
}
