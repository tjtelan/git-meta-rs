use git_meta::GitRepo;
use mktemp::Temp;

#[test]
fn partial_on_deep_clone() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    assert_eq!(
        repo.expand_partial_commit_id("c097ad2").unwrap(),
        "c097ad2a8c07bf2e3df64e6e603eee0473ad8133"
    );
}

#[test]
fn partial_on_shallow_clone() {
    let tempdir = Temp::new_dir().unwrap();

    let repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone_shallow(&tempdir)
        .unwrap();

    assert_eq!(repo.expand_partial_commit_id("c097ad2").is_ok(), false);
}
