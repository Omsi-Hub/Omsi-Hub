// The installers of a release GitHub's Release workflow built (a draft too) into Builds/ beside the
// main clone: Builds/beta/<version>/ for a version with a "-", else Builds/release/<version>/
// (docs/WORKFLOW.md). Needs the GitHub CLI, signed in. Files already there are kept.
//
//   node tools/get-release.mjs 1.2.0-beta.3

import { execSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";

const version = process.argv[2]?.replace(/^v/, "");
if (!version) {
  console.error("usage: node tools/get-release.mjs <version>");
  process.exit(1);
}

const git = (args) => execSync(`git ${args}`, { encoding: "utf8" }).trim();
// (the main clone is the .git folder's parent, also from a worktree under .claude/worktrees)
const clone = dirname(git("rev-parse --path-format=absolute --git-common-dir"));
const dir = join(dirname(clone), "Builds", version.includes("-") ? "beta" : "release", version);

mkdirSync(dir, { recursive: true });
execSync(`gh release download v${version} --dir "${dir}" --skip-existing`, { stdio: "inherit" });
console.log(dir);
