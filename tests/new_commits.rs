use git_meta::GitRepo;
use mktemp::Temp;

#[test]
fn new_commits_deep_clone() {
    let tempdir = Temp::new_dir().unwrap();

    // We're just using this for cloning
    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    let repo = GitRepo::open(
        tempdir.to_path_buf(),
        Some("main".to_string()),
        Some("f6eb3d6b7998989a48ed1024313fcac401c175fb".to_string()),
    )
    .unwrap();

    assert!(repo.new_commits_exist());
}
