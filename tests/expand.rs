use git_meta::GitRepo;
use std::env;

#[test]
fn expand() {
    let current_dir = env::current_dir().unwrap();
    let repo = GitRepo::open(current_dir, None, None).unwrap();

    assert_eq!(
        repo.expand_partial_commit_id("c097ad2").unwrap(),
        "c097ad2a8c07bf2e3df64e6e603eee0473ad8133"
    );
}
