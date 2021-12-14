# [0.5.0](https://github.com/tjtelan/git-meta-rs/compare/v0.3.0...v0.4.0) (2021-12-13)
- Add `GitRepoCloneRequest` and moved repo cloning code from `GitRepo`
- Add support to `GitRepo` for opening repo filepath w/ local-only branches

# [0.4.0](https://github.com/tjtelan/git-meta-rs/compare/v0.3.0...v0.4.0) (2021-11-13)
- Fix clippy warnings
- Replace panic behavior with returning `Err()`
- Update all `bool` returning functions to `Result<bool>`
- Migrate to Rust 2021
# [0.3.1](https://github.com/tjtelan/git-meta-rs/compare/v0.3.0...v0.3.1) (2021-02-26)
- Changed the order of short-circuit checks in `expand_partial_commit_id()` ([#15](https://github.com/tjtelan/git-meta-rs/issues/15))
# [0.3.0](https://github.com/tjtelan/git-meta-rs/compare/v0.2.1...v0.3.0) (2021-02-14)
- Changed `with_commit()` to take a commit id. Moved the `git2::Commit` builder to `with_git2_commit()`
- Changed `with_branch()` to take an `Option<String>`
- Renamed `list_files_changed` to `list_files_changed_between` for listing changed files between 2 commits
- Added `list_files_changed_at` to list changed files between a single commit and it's previous commit
# [0.2.1](https://github.com/tjtelan/git-meta-rs/compare/v0.2.0...v0.2.1) (2021-02-13)
- Check for new commits in a branch ([#2](https://github.com/tjtelan/git-meta-rs/issues/2))
- Check for changes in a path ([#4](https://github.com/tjtelan/git-meta-rs/issues/4))
- Expand partial commit ids ([#7](https://github.com/tjtelan/git-meta-rs/issues/7))
- Added some tests and examples using this repo's commits
- Checking if repo is a shallow clone ([#9](https://github.com/tjtelan/git-meta-rs/issues/9))
# [0.2.0](https://github.com/tjtelan/git-meta-rs/compare/v0.1.0...v0.2.0) (2021-02-12)
- Loosened requirements for local branches resolving to remote branches ([#5](https://github.com/tjtelan/git-meta-rs/issues/5))
- Modified `GitRepo::with_commit()` to take `Option<Commit>`
# [0.1.0](https://github.com/tjtelan/git-meta-rs/compare/v0.0.1...v0.1.0) (2021-02-08)
- Removed `GitRepoCloner`
- `GitRepo::open()` now returns `Result<GitRepo>`
- Minor renaming and rearranging of several impls in codebase
- Lots of new documentation
# [0.0.1](https://github.com/tjtelan/git-meta-rs/commit/b24fe6112e97eb9ee0cc1fd5aaa520bf8814f6c3) (2021-02-06)
- Merging functionality from [Orbital](https://github.com/orbitalci/orbital) and from [git-event-rs](https://github.com/tjtelan/git-event-rs)
- Introduced `GitRepo` struct
- Introduced `GitCommitMeta` impl `new` with some options.
- Converting `GitCommitMeta::epoch_time` timestamps from `i64` to `GitCommitchrono::DateTime<Utc>`
- Convert commit hash from Vec[u8] to String
- `GitCredential::SshKey` take in `PathBuf` to clarify usage
- Introduced temporary `GitRepoCloner` struct with `From<GitRepo>` and vice-versa