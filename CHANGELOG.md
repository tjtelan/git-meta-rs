# [0.2.1](https://github.com/tjtelan/git-meta-rs/compare/v0.1.0...v0.2.0) (2021-02-12)
- Check for changes in a path ([#4](https://github.com/tjtelan/git-meta-rs/issues/4))
- Expand partial commit ids ([#7](https://github.com/tjtelan/git-meta-rs/issues/7))
- Added some tests and examples using this repo's commits
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