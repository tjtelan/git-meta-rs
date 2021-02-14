use git_meta::GitRepo;
use mktemp::Temp;

#[test]
fn deep_clone_defaults() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    let repo_clone = GitRepo::open(tempdir.to_path_buf(), None, None).is_ok();

    assert!(repo_clone);
}

#[test]
fn deep_clone_by_branch() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    let repo_clone = GitRepo::open(tempdir.to_path_buf(), Some("main".to_string()), None).is_ok();

    assert!(repo_clone);
}

#[test]
fn deep_clone_by_id() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    let repo_clone = GitRepo::open(
        tempdir.to_path_buf(),
        None,
        Some("f6eb3d6b7998989a48ed1024313fcac401c175fb".to_string()),
    )
    .is_ok();

    assert!(repo_clone);
}

#[test]
fn deep_clone_by_branch_id() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone(&tempdir)
        .unwrap();

    let repo_clone = GitRepo::open(
        tempdir.to_path_buf(),
        Some("main".to_string()),
        Some("f6eb3d6b7998989a48ed1024313fcac401c175fb".to_string()),
    )
    .is_ok();

    assert!(repo_clone);
}

#[test]
fn shallow_clone_defaults() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone_shallow(&tempdir)
        .unwrap();

    let repo_clone = GitRepo::open(tempdir.to_path_buf(), None, None).is_ok();

    assert!(repo_clone);
}

#[test]
fn shallow_clone_by_branch() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone_shallow(&tempdir)
        .unwrap();

    let repo_clone = GitRepo::open(tempdir.to_path_buf(), Some("main".to_string()), None).is_ok();

    assert!(repo_clone);
}

#[test]
fn shallow_clone_by_id() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone_shallow(&tempdir)
        .unwrap();

    // We shouldn't be able to open a shallow clone by commit
    let repo_clone = GitRepo::open(
        tempdir.to_path_buf(),
        Some("main".to_string()),
        Some("f6eb3d6b7998989a48ed1024313fcac401c175fb".to_string()),
    )
    .is_ok();

    assert_eq!(repo_clone, false);
}

#[test]
fn shallow_clone_by_branch_id() {
    let tempdir = Temp::new_dir().unwrap();

    let _clone_repo = GitRepo::new("https://github.com/tjtelan/git-meta-rs.git")
        .unwrap()
        .git_clone_shallow(&tempdir)
        .unwrap();

    // We shouldn't be able to open a shallow clone by commit
    let repo_clone = GitRepo::open(
        tempdir.to_path_buf(),
        Some("main".to_string()),
        Some("f6eb3d6b7998989a48ed1024313fcac401c175fb".to_string()),
    )
    .is_ok();

    assert_eq!(repo_clone, false);
}
