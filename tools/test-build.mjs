// A test build of the branch checked out here, to try it before it merges (docs/WORKFLOW.md):
// Tauri's build without installers, its exe copied to Builds/test/ beside the main clone as
// <date>_<branch>_<commit>.exe, with "-dirty" when changes are not committed. Not for players.
//
//   node tools/test-build.mjs

import { execSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const git = (args) => execSync(`git ${args}`, { encoding: "utf8" }).trim();
const top = git("rev-parse --show-toplevel");
// (the main clone is the .git folder's parent, also from a worktree under .claude/worktrees)
const clone = dirname(git("rev-parse --path-format=absolute --git-common-dir"));
const builds = join(dirname(clone), "Builds");

const branch = git("rev-parse --abbrev-ref HEAD").replace(/[\\/]/g, "-");
const commit = git("rev-parse --short HEAD");
const dirty = git("status --porcelain") ? "-dirty" : "";
const date = new Date().toISOString().slice(0, 10);
const win = process.platform === "win32";

execSync("npx --yes @tauri-apps/cli@2 build --no-bundle", { cwd: join(top, "app"), stdio: "inherit" });

const target = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR) : join(top, "target");
const out = join(builds, "test", `${date}_${branch}_${commit}${dirty}${win ? ".exe" : ""}`);
mkdirSync(dirname(out), { recursive: true });
copyFileSync(join(target, "release", win ? "omsi-hub.exe" : "omsi-hub"), out);
console.log(`\n${out}`);
