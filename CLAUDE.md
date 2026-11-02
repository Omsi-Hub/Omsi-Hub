# Omsi-Hub: how a session works

The whole workflow is in `docs/WORKFLOW.md`. What every session keeps to:

- **Work in your own worktree and branch** (`claude/<name>`, `feat/<topic>`, `fix/<topic>`), from
  the newest `origin/main`. Never commit on `main`; the clone at `Omsi-Hub\` stays on `main`,
  clean.
- **Commit each step that works, and push the branch before the session ends.** Ask before
  pushing when the user has not said so; say plainly when something is left unpushed.
- **Done and good means a pull request into `main`** with CI's checks passing locally
  (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`)
  and the player-facing change under `## Unreleased` in `CHANGELOG.md`.
- **Builds go in `Builds\` beside the clone, nowhere else**: a test build of a branch with
  `node tools/test-build.mjs`, a beta's or a release's installers with
  `node tools/get-release.mjs <version>`. Never an exe or a copy of the repository in a folder of
  its own.
- **A beta or a release is a tag on `main`** (`v1.2.0-beta.3`), built by GitHub's Release
  workflow; the version commit and the tag are the user's call (docs/WORKFLOW.md, "A beta").
- Write the repository's texts (code comments, docs, commit messages) in English, as the rest.
