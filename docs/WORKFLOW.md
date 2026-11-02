# How we work

The one place each thing has, and the one path a change takes from an idea to the players: a
branch, a pull request into `main`, and a build GitHub makes from `main`. Whatever works and is
good is in git; whatever is not in git does not exist. Claude's sessions follow the same rules
(`CLAUDE.md`).

## The folders on the PC

```
C:\Omsi-Hub main folder\
├─ Omsi-Hub\                         this repository's clone: on main, nothing uncommitted
│  └─ .claude\worktrees\<name>\      one worktree per piece of work (git ignores the folder)
├─ Builds\                           every build kept outside GitHub, nothing else
│  ├─ beta\<version>\                a pre-release's installers as GitHub built them
│  ├─ release\<version>\             a release's installers as GitHub built them
│  └─ test\<date>_<branch>_<commit>.exe   a branch's test build, made on this PC
└─ <other repository>\               the website, the CMS, the companion app, the openOMSI fork:
                                     each its own clone beside this one, by the same rules
```

- **`Omsi-Hub\` stays on `main`, clean.** You pull there and start worktrees from there; you do
  not work in it. `git status` there says nothing.
- **Work happens in a worktree** under `Omsi-Hub\.claude\worktrees\`. The Claude app makes one
  for each session and removes it when the session is archived. By hand:
  `git worktree add .claude/worktrees/<name> -b feat/<name> origin/main`.
- **`Builds\` is the only place for a build.** Never an exe in a folder of its own, never a copy
  of the repository ("Old", "-build", "-navmap"): git's history is the backup, `Builds\` the
  shelf.
- `.claude/launch.json` still points at the other repositories under `C:/Omsi-hub/`; when they
  move beside this one, its paths move with them.

## Branches

| Branch | For | Lives |
|---|---|---|
| `main` | what is done: CI passes, the changelog says what changed | always |
| `feat/<topic>` | something new | until merged or dropped |
| `fix/<topic>` | a fault | until merged or dropped |
| `chore/<topic>` | the tools, the docs, CI, this workflow | until merged or dropped |
| `claude/<name>` | a Claude session's work (the app names it) | until merged or dropped |
| `release/<x.y>` | only when `main` must go on with work that is not for the testers | until `<x.y>.0` is out |

There are no `beta/…` branches: a beta is a tag on `main` (below).

1. **One topic, one branch**, started from the newest `origin/main`.
2. **Commit each step that works, and push the branch at the end of every session at the
   latest.** A commit only on this PC is not saved.
3. **Works and good:** a pull request into `main`. CI passes (fmt, clippy, tests), what a player
   notices is under `## Unreleased` in `CHANGELOG.md`, then merge. The branch is deleted and its
   worktree removed.
4. **Not good:** close it, delete the branch. No branch waits more than two weeks without a pull
   request.
5. **Nobody commits on `main` itself**, except the version commit of a build (below).

## Trying a branch before it merges

```
node tools/test-build.mjs
```

builds the branch you are on (Tauri, without installers) and puts the exe in
`Builds\test\<date>_<branch>_<commit>.exe`. The name says exactly what is in it. A name ending in
`-dirty` has changes that were not committed: try it yourself, give it to nobody. Test builds go
once their branch is merged.

## A beta (pre-release)

A beta is built from `main` only, by GitHub's Release workflow, never on the PC.

1. **Everything for the beta is merged** into `main` and CI passes. In `Omsi-Hub\`:
   `git pull`.
2. **The version commit**, on `main`:
   - `Cargo.toml` (`[workspace.package] version`) and `app/tauri.conf.json` (`version`) say
     `1.2.0-beta.3`;
   - in `CHANGELOG.md`, `## Unreleased` becomes `## 1.2.0-beta.3` with a line for the testers, and
     a new empty `## Unreleased` goes above it;
   - `cargo test -p hub-api --test version` passes;
   - the commit is `Omsi-Hub 1.2.0-beta.3: <what it is for>`.
3. **Tag and push:** `git tag v1.2.0-beta.3`, then `git push origin main v1.2.0-beta.3`. The tag
   starts the Release workflow; the `-` in it makes a draft pre-release with the installers for
   Windows, macOS and Linux.
4. **Fetch and try it:** `node tools/get-release.mjs 1.2.0-beta.3` puts the installers in
   `Builds\beta\1.2.0-beta.3\`. Install them and try them.
5. **Good:** publish the draft (Releases → the draft → Edit → Publish release). It stays a
   pre-release; testers download it themselves and the app offers it to nobody
   (`docs/UPDATES.md`). **Wrong:** fix it on a branch with a pull request, and make
   `1.2.0-beta.4`. A tag is never moved or used again.

**A release** goes the same way with `1.2.0`: it lands in `Builds\release\1.2.0\` and is published
as a normal release, which the app offers to the players. A test build is offered nothing, so the
testers install `1.2.0` themselves.

**When `main` has to go on** with work that is not for the testers: `release/1.2` from the beta's
tag, its fixes by pull request into it and merged back into `main` at once, the next betas tagged
on it. Only then; otherwise `main` is the one line.

## The changelog

Every change a player notices gets its lines under `## Unreleased` in the pull request that makes
it, not under a version that is already built. The version commit gives `Unreleased` its number.

## Keeping it tidy

Before each beta, or once a week:

- `git fetch --prune`, then `git branch -r --merged origin/main`: delete those on GitHub.
- `git worktree list`: remove the worktrees of merged or dropped branches (in the app: archive the
  session), then `git worktree prune`.
- `Builds\test\`: delete the builds of merged branches.

On GitHub, once: Settings → General → "Automatically delete head branches", and a rule for `main`
that asks for CI to pass before a merge.
