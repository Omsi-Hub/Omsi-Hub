# Handover: carrying on with the Omsi-Hub app

For the next AI (or person) that works on this repository. Written 2026-11-02 from the state of
`C:\Omsi-hub\Omsi-Hub`. It says what the app is, where everything lives, how Luc wants the work
done, and exactly where the work stands. The deeper documents it points to are the source of truth;
this file is the map.

**Luc** (Luc Ruigrok, GitHub `Luc-nbr`) owns the project and decides. He writes in **Dutch**:
answer him in Dutch. The code, comments, commits and docs are in **English**.

---

## 1. What Omsi-Hub is

Omsi-Hub is a **desktop add-on app for OMSI 2**, the German bus simulator, played through
**openOMSI** (an open-source re-implementation of OMSI 2's engine that plays OMSI 2's own maps,
buses and mods). Its tagline: *"built for realism, made for immersion"*. It turns driving a bus in
OMSI into **a working bus job**: real duties from the timetable, your own transport company, a
career that counts, and friends on the same roads.

The player installs official openOMSI (0.2.22 or newer) and an original OMSI 2 (Steam or retail);
Omsi-Hub sits beside them as its own program. It **ships no game** and **never changes the files of
openOMSI or OMSI 2**: it starts openOMSI with official command-line flags and talks to it through
openOMSI's plugin API (ABI 1). It runs on **Windows, macOS and Linux** (Linux: OMSI 2 through Steam
/ Proton).

### Product history in one paragraph

Omsi-Hub began as a fork of openOMSI with its own game (`omsi-hub-game.exe`) and launcher; that
code is archived in `Omsi-hub Old` / `Luc-nbr/omsi-hub`. On 2026-10-17 Luc decided to make it a
**separate add-on app with its own code** (docs/PLAN.md). About 230k lines of Omsi-Hub's own code
were ported into this repo; openOMSI parts were replaced by our own readers (`hub-omsi`), except
the bus rendering stack, which still uses MIT-era openOMSI crates from Luc's fork. Releases 1.0 →
1.1.1 went out in October 2026; 1.2.0-beta is in testing.

### What the player can do (the features)

**The start and the drive.** A first-run welcome (language, animations, the two game folders,
optionally sign in, migrate old saves). Then the home page with four ways to drive, each a flow of
steps (profile → map → day & weather → duty → bus → bus options/display font → start):

- **Work shift**: pick a length (or *from–to* times, or how many lines); Omsi-Hub composes a
  shift from the timetable's real trips, changing lines at termini.
- **Tour**: one bus's trips on one line, as OMSI's timetable plans them.
- **Free drive**: a map, a bus and a start point.
- **Bus company**: drive a duty of your own company (below).

The bus step shows all installed buses grouped by maker → model → version, with photos rendered
from the 3D model, a 3D showroom, paint/number/depot-file choice. While the game runs, a **live
dashboard** replaces the launcher: speed, delay, the duty's plan, the route map with the bus on
it, messages. After the drive the trips are booked into the driver's service record.

**In the game (the plugin).** A **navigator** (Shift+N) with the route, the stops ahead and
on-time status; a duty strip; control-room messages with replies; instructions carried out in the
game (short-turn, out of service, relief) with sounds; notices on screen; only the company's own
buses as AI traffic where the fleet map has them; Omsi-Hub's look for openOMSI's loading screen
and pause menu (an openOMSI theme pack).

**The bus company** (the biggest part, ~90 modules in `hub-core/src/company`). Buy, lease or rent
buses (dealer, used market, auctions, orders); hire and train drivers (licences, training matrix,
work schedules, holidays, pay with overtime and week guarantee); win lines and contracts
(concessions, tenders); a depot you build yourself in 3D, with staff walking about; a workshop
with repairs and parts; finances, loans, reputation, complaints, incidents, research; a clock
that simulates the day; an inbox and agenda; a fleet map showing every bus live; career and
levels; a sandbox mode with unlimited money; **Play together**: friends join your company with a
`HUB-…` code, take duties and drive in the same world (openOMSI's LAN layer, a Cloudflare tunnel).

**RosterFlow** (inside the company): the day's tours are cut into duties (longest duty setting,
split a duty with one click, reliefs, breaks), a Gantt planning view, a plan editor, PDF print of
rosters and **duty cards** (two per A4). It has its own complete handbook (PDF on omsihub.com).

**Career / service record.** Per driver: hours, kilometres, punctuality, tickets, points and
levels; optionally on the website's leaderboard.

**Editors.** Line editor (draw a new line on the map's own roads/lanes, stops, timetable, export
into the map's `TTData`), timetable editor, depot files (`.hof`: destinations, stops, routes),
the player's own depot, the **livery studio** (paint a bus in 3D: fills, brushes, decals, images,
mirroring, then bake and export a texture/repaint), map objects. Settings has controllers (a
set-up assistant), vehicles, mods (installed with openOMSI's own mod installer), openOMSI's own
game settings, phone & tablet, updates.

**Phone & tablet Companion.** A separate app (Expo; iOS, Android) or the same thing as a web app
served by Omsi-Hub on `http://<pc>:47811`. Paired with a 6-digit code/QR. Shows the duty, the
navigation turn by turn, stops, messages, breaks, and draws the bus's own **IBIS** and **ticket
printer** (read from the bus's model files) so you can press their keys from the phone. Sign-on
with the account's personnel pass (number + code).

**Online (optional).** Email, Google or Discord account; one account = one driver on that PC;
friends and online state, invites, the leaderboard, a company board, two cloud saves. Without an
account everything works offline. On **omsihub.com** a company in the cloud can be opened in the
browser: the same Rust company code compiled to WebAssembly.

**Updates.** The app updates itself from this repo's GitHub releases (signed, installs silently
when no game runs). Test builds (`-beta`) are never offered.

It is proprietary since 2026-10-19 (`LICENSE`; code up to 0.1.0 was MIT). Public repo:
`github.com/Omsi-Hub/Omsi-Hub-Addon`. Website: omsihub.com. Contact: info@omsihub.com. Support:
ko-fi.com/omsihub. It is a fan project, not affiliated with OMSI 2's makers nor part of openOMSI.

Current version line: **1.2.0-beta** (test builds for beta testers). Last public release: **1.1.1**
(tags `v1.0.0` … `v1.1.1`).

## 2. Read these first

| File | Why |
| --- | --- |
| [README.md](../README.md) | The product as players see it |
| [docs/ARCHITECTURE.md](ARCHITECTURE.md) | **The main technical doc**: crates, contracts, command prefixes, data folder, how a drive works end to end, how to add a feature, building, platforms |
| [docs/PLAN.md](PLAN.md) | Why the app is its own code (the split from the openOMSI fork), the porting rule, phases |
| [docs/INVENTORY.md](INVENTORY.md) | What was ported from the old repo and how |
| [CHANGELOG.md](../CHANGELOG.md) | Player-facing changes per version; the 1.2.0-beta.1 section is the newest |
| [docs/UPDATES.md](UPDATES.md) | The updater and how a release is made |
| [docs/ONLINE.md](ONLINE.md) | Accounts, Supabase, cloud saves, security rules for online code |
| [docs/COMPANION.md](COMPANION.md), [COMPANION-HANDOFF.md](COMPANION-HANDOFF.md) | The phone/tablet protocol (API 1) and what was proven on the real game |
| [docs/WEB.md](WEB.md) | The company mode compiled to wasm for the website |
| [docs/PRIVACY.md](PRIVACY.md) | What is collected; keep the privacy statement in step |
| [plugin/README.md](../plugin/README.md) | The Lua plugin |
| `apptest/HANDOFF.md` | The real-app test harness and the bug hunt (folder **not in git**, see §9) |

Design documents for work **not built yet** (untracked, see §7):
[docs/MULTIPLAYER.md](MULTIPLAYER.md) (an always-on online company on a server) and
[docs/COMPANION-EMPLOYEE-HANDOVER.md](COMPANION-EMPLOYEE-HANDOVER.md) (the companion as an employee
app; open decisions to take with Luc one at a time).

## 3. The technology

### The stack

| Layer | Technology |
| --- | --- |
| Desktop shell | **Tauri 2** (Rust), stock; plugins `tauri-plugin-updater`, `-dialog`, `-opener`. One window (`main`, 1280×820), commands `call`, `call_bytes`, `pick_folder`, `pick_file`, `open_url`, `open_path`, `selftest_report` (`app/src/main.rs`) |
| Web view | WebView2 (Windows; `webview2-com` for the crash handling), WKWebView (macOS), WebKitGTK 4.1 (Linux) |
| Page / UI | **Preact + htm** as one vendored ES module (`ui/vendor/preact-htm.js`), plain ES modules, **no bundler, no npm build**, no TypeScript. Own store (`store.js`: `set`, `get`, `useStore`), own i18n (`i18n.js`: `t`/`tr`), own icons (`symbols.js`), CSS with `--hub-*` tokens, font Hanken Grotesk |
| 3D | **Own WebGL2 engine** (`ui/js/hub/bus3d/`: `engine.js`, `shaders.js`, `camera.js`, `worker.js`), no three.js. The bus mesh and textures come from Rust (`b3d_*`, `lak_*`, on openOMSI's readers) as bytes. The company depot has its own 3D view (`ui/js/company/depot3d`) |
| Back end | **Rust** 2021 edition, toolchain 1.99.0, a Cargo workspace of 10 crates (~180k lines) + `app` |
| Key Rust crates | `serde`/`serde_json`, `anyhow`, `ureq` (HTTP, TLS), `chrono`, `glam` (math), `rayon`, `parking_lot`, `encoding_rs` (OMSI's codepages), `sha2`/`ring`, `zip`, `zstd`/`ruzstd` (saves), `flate2` + `image` (PDF pictures), `resvg` (SVG), `ab_glyph` (fonts), `keyring` (the account's tokens in the OS keychain), `mlua` (Lua 5.4, tests only) |
| OMSI files | Own readers `hub-omsi` (cfg/text with codepages, VFS over content folder + archives + OMSI 2, case-insensitive paths, timetables, maps, roads/lanes, `.hof`, `.bus`, fonts, weather). Bus rendering on openOMSI crates from Luc's fork (`omsi-cfg`, `omsi-o3d`, `omsi-model`, `omsi-texture`, `omsi-vehicle`, `omsi-sim`, …) |
| In-game | **Lua 5.4 plugin** (`plugin/*.lua`, embedded in the app at build time, installed into openOMSI's `Plugins/omsihub/`) on openOMSI's plugin API (ABI 1); each tick must stay under 50 ms or openOMSI switches the plugin off |
| Online | **Supabase** (Auth, PostgREST RPC, Storage) in `hub-online`; publishable key only, every write a `SECURITY DEFINER` function; schema in `online/supabase/schema.sql`, the live migrations in the website repo |
| Website | Next.js 16 / React 19 (separate repo); the company there is `crates/hub-web` built for `wasm32-wasip1` with `browser_wasi_shim` |
| Companion | Expo SDK 58 / React Native (separate repo); talks HTTP to `hub-companion` |
| Packaging | NSIS installer (Windows, with WebView2 bootstrapper), `.app`/`.dmg` (macOS), AppImage/`.deb`/`.rpm` (Linux); signed updater files and `latest.json` via GitHub Actions |
| Tooling | Node scripts in `tools/` (brand art via Playwright, mock server, web build), Python (`translations.py`, `licenses.py`, depot textures, staff models) |

### The running system

```
                ┌──────────── Omsi-Hub (Tauri app, one process) ─────────────┐
 Player ──UI──▶ │ page (Preact, ui/)  ──call(cmd,args)──▶ hub-api ─▶ hub-core │
                │   ▲ events (game-started, game-ended…)     │      hub-omsi  │
                │   └────────────────────────────────────────┘      hub-game  │
                │  bytes route  hub.localhost/bytes/… (meshes, textures, PDFs)│
                │  hub-companion: HTTP :47811 (LAN) + UDP 127.0.0.1:47812     │
                └──────┬──────────────────────────────────▲──────────────────┘
       starts with     │ official flags            files  │ reports.jsonl, dash.json,
       --map --bus …   ▼ + assignment files               │ replies.jsonl, status.json
                ┌──────────────── openOMSI (player's own) ────────────────────┐
                │   Plugins/omsihub/*.lua  (navigator, legs, companion, dash) │
                │   reads OMSI 2's maps, buses, mods                          │
                └─────────────────────────────────────────────────────────────┘
  Supabase (accounts, friends, leaderboard, cloud saves) ◀── HTTPS ── hub-online
  GitHub releases (updates) ◀── update.rs + tauri-plugin-updater
  Companion app (phone) ── HTTP :47811 ──▶ hub-companion
```

- **App ↔ game** is a **file bridge**: the app writes the assignment (`bridge::Assignment`, the
  legs, settings, `fleet.json`, `messages.json`, `notices.json`, `ontime.json`) into the plugin's
  data folder; the plugin writes reports back (`reports.jsonl`, `dash.json` every 2 s,
  `replies.jsonl`, `status.json`). Contract in `crates/hub-game/src/bridge.rs` (`BRIDGE` version).
  For a plugin without file functions the app falls back to reading the game's log.
- **Plugin ↔ companion**: UDP datagrams (`omsi.send`, ≤8 KB) to the app; commands back through
  `data/companion.json`.
- **Version negotiation**: `hub_game::compat` reads openOMSI's version (file metadata, folder name
  or `openomsi --version` once) and the plugin's `hello`, and switches features on/off
  (display font, IBIS auto-typing, closed stops, docked board, `timetable.set_ai_trips`).
- **Saves**: JSON / JSONL files in the data folder (`companies/`, `careers/`, `trips/`, `lines/`,
  `settings.json`, …); a cloud save is that folder packed with zstd.
- **Play together**: the boss's app is an HTTP company server behind a Cloudflare quick tunnel;
  in the game openOMSI's LAN layer (`--lan-host` / `--lan-join`).

## 4. The code

```
app/            Tauri 2 desktop shell (src/main.rs): window, commands call/call_bytes, updater, bytes route
ui/             the page: Preact + htm (ui/vendor), no bundler. api.js, store.js, i18n.js, pages
  js/hub/         home, drive flow, live dashboard, settings, bus 3D view, livery studio (bus3d/lak)
  js/steps/       the drive's steps (profile, map, day, duty, bus …)
  js/company/     the bus company UI     js/rosterflow/  RosterFlow
  js/editors/     line editor, timetable, depot files      js/online/  account, friends
  js/web/         the website build's backend              js/mock.js  mock answers (browser preview)
  css/            --hub-* tokens in css/tokens.css
crates/
  hub-omsi      our own OMSI 2 readers/writers (no openOMSI code)          ~4.5k lines
  hub-core      the domain: company (~90 modules), RosterFlow, careers      ~84k
  hub-api       commands routed by prefix; 3D view, livery studio, line editor   ~61k
  hub-game      finding/checking/starting openOMSI, installing the plugin, the file bridge
  hub-livery    the livery studio's core (projects, bake, paint, export)
  hub-companion the LAN server for the Companion app (HTTP :47811, UDP :47812)
  hub-devices   a bus's IBIS/ticket printer/screens read from its model files
  hub-online    Supabase client (auth, friends, leaderboard, cloud saves)
  hub-locales   texts: locales/*.yml (English key, then the translations, 7 languages)
  hub-web       hub-api built for wasm32-wasip1 (the website's company mode)
plugin/         Lua plugin that runs inside openOMSI (main.lua, navigator, legs, companion …); harness/ tests it on mlua
fixtures/       fake openOMSI/OMSI 2 folders for setup tests
online/         Supabase schema (online/supabase/schema.sql) and a stand-in
tools/          brand/installer art, serve-ui.mjs (mock preview), build-web.mjs, translations.py …
```

Dependency direction: `app → hub-api → hub-core → hub-omsi`; `hub-api → hub-game, hub-locales,
hub-online, hub-companion`; `hub-companion → hub-game, hub-devices`.

**openOMSI crates** (`omsi-cfg`, `omsi-model`, `omsi-o3d`, `omsi-sim`, `omsi-texture`,
`omsi-vehicle`, `omsi-map`, … and `omsi-launcher-core` without `rar`) come as git dependencies from
**Luc's private fork** `github.com/Luc-nbr/openOMSI`, pinned to one commit in `Cargo.toml`
(openOMSI before it became AGPL, MIT). They are only for the 3D view, photos, livery studio, line
editor and mods. Everything else reads OMSI files with `hub-omsi`. **Do not copy code from current
(AGPL) openOMSI.** CI reads the fork with the secret `FORKED_OPEN_OMSI`.

**How the page talks to Rust:** `call(cmd, args)` from `ui/js/api.js` → Tauri command `call` →
`hub_api::call`, routed by prefix (`hub_`, `addon_`, `co_`, `rf_`, `tt_`, `de_`, `ol_`, `cs_`,
`lak_`, `b3d_`, `hub_le`, `hub_companion_` …; table in ARCHITECTURE.md). Binary answers go through
`call_bytes` / `bytesUrl`. Events back to the page: `game-started`, `game-ended`, etc.

**How a drive works:** page `Duty` → `LaunchSpec` (official openOMSI flags only) → plugin made
ready and given `bridge::Assignment` (written as files into the plugin's data folder) → game
started → `drive::watch` reads the plugin's `reports.jsonl`, `dash.json` every 2 s and books trips
into `trips/<driver>.jsonl` and the company. Full detail in ARCHITECTURE.md "A drive".

**Data folder:** `%APPDATA%\Omsi-Hub` (Windows), `~/Library/Application Support/Omsi-Hub`,
`$XDG_DATA_HOME/Omsi-Hub`. `OMSIHUB_DATA_DIR` moves it. Log: `omsi-hub.log` (+ `.log.1`). The
app **never writes into openOMSI's or OMSI 2's own files** (exception by design: the line editor
exports into the content folder's copy of a map's `TTData`).

## 5. Building, running, testing

Toolchain pinned in `rust-toolchain.toml` (1.99.0). Windows dev machine; shell is PowerShell or
Git Bash.

```bash
cargo test --workspace                 # every crate, the plugin harness, the app's unit tests
cargo build -p omsi-hub                # the app -> target/debug/omsi-hub.exe
cargo fmt --all --check && cargo clippy --workspace --all-targets
cd app && npx @tauri-apps/cli@2 build  # installers (NSIS / .app/.dmg / AppImage, .deb, .rpm)
node tools/serve-ui.mjs 5174           # the page in a browser with mock answers: http://localhost:5174/?mock
node tools/build-web.mjs               # the website's wasm company build -> dist/web/
```

- `.claude/launch.json` has `ui-mock` (port 5174) for the preview pane, plus entries for the
  sibling repos (website, CMS, companion web) and `fakegame-web`.
- Selftest in the real app: `OMSIHUB_SELFTEST=1 OMSIHUB_SELFTEST_FIXTURES=<repo>/fixtures
  target/debug/omsi-hub.exe` (exit 0 = all passed). Full suite: `?selftest=full&intro=off`
  (177 checks; some are stale on a beta version, see §8).
- Real-data test of the readers: `HUB_OMSI_REAL_ROOT=<OMSI 2 folder> cargo test -p hub-omsi --test
  real_data -- --ignored`.
- End-to-end drive test against a fake openOMSI: `crates/hub-api/tests/drive.rs`.
- `OMSIHUB_NO_UPDATE=1` switches the update check off.
- The debug build is slow on Luc's ~570 bus models; several `target-*` folders exist from other
  sessions/worktrees (`target-harness`, `target-livery`, `target-stops` …): leave them.
- **Don't build while an instance of the app runs** (the exe is locked).

CI (`.github/workflows/ci.yml`): on push to `main` and PRs only Linux fmt/clippy/tests and
`node --check` on the page's scripts (Actions minutes cost money); the full Windows/macOS/Linux run
and the Tauri builds are run by hand before a release. `release.yml` builds a **draft** release on a
`v*` tag; Luc publishes it by hand. A version with a `-` is a pre-release that the updater never
offers.

## 6. How Luc wants the work done (rules)

1. **No AI attribution anywhere**: no `Co-Authored-By: Claude`, no "Generated with Claude Code" in
   commits or PRs. openOMSI's maintainers asked for this; it holds for every repo.
2. **Commit or push only when Luc asks.** Branch off; PRs go to `main`. **Luc merges PRs himself.**
3. **Commit message style**: one long, plain-English sentence that says what the player gets,
   then the how, e.g. *"Bus step: the depot file's dialog only for a bus the player picks"* or
   *"Speed: OMSI's folders read once, not at every look - the bus list 12.5 s to 0.9 s"*. Look at
   `git log` and match it.
4. **Code style**: match the surrounding code. Comments are short and often in parentheses,
   saying *why* in the player's terms: `// (after a drive the bus step begins at its makers again)`.
   Doc comments in plain English. Same for CSS and JS.
5. **Every player-visible text** goes through `tr`/`t` and into
   `crates/hub-locales/locales/*.yml` with all its translations (7 languages). Missing keys show up
   as English in a Dutch UI and get reported as bugs.
6. **A change to any `plugin/*.lua`** needs the plugin's `VERSION` raised in **both**
   `plugin/main.lua` and `crates/hub-game/src/install.rs` (now `1.18.0`), and a line in the
   changelog.
7. **Keep `CHANGELOG.md` in step** for anything a player notices, in its voice (bold lead, then
   what the player sees). Keep `docs/*` in step when contracts change.
8. **New page features** need mock answers in `ui/js/mock.js` so they open in the browser preview,
   and ideally a check in `ui/js/selftest.js`.
9. **Verify in the real app**, not only the mock, and in openOMSI when the change touches drives,
   the plugin or the bridge. Luc asked for this explicitly.
10. **Online is live and watched for cost.** Never sign in, sign up, send invites or upload cloud
    saves from tests. New online features must be cheap (small, sent on change, read with a digest;
    see `hub_poll` in ONLINE.md). All writes go through `SECURITY DEFINER` functions; no service key
    in the app or repo.
11. **Never touch Luc's own installs or data** while testing: `C:\openOMSI`, `E:\OpenOMSI`,
    `E:\Open-Omsi Official`, `%LOCALAPPDATA%\Programs\openOMSI`, `C:\oo-test`, `%APPDATA%\Omsi-Hub`,
    `~/.openomsi`, his installed Omsi-Hub. Omsi-Hub follows `~/.openomsi/launcher.json` to "the
    openOMSI in use", so a test instance needs its own home (`OMSIHUB_HOME_DIR` + `HOME`; **not**
    `USERPROFILE`, that breaks WebView2's DevTools port). Kill only processes you started.
12. Things that need openOMSI changes go upstream as **small separate PRs** to openOMSI
    (display font, IBIS auto-typing, closed stops, docked duty board, pause menu …); until then
    the feature degrades gracefully and `hub_game::compat` says what the player's openOMSI supports.

## 7. Where the work stands (2026-11-02)

### Branches

- **Current branch: `fix/webview-crash-reload`** — local only, **not pushed**, no upstream.
  - It is `beta/1.2.0-beta.1` + later commits up to `72f3d83` + one own commit **`cda0734`**:
    *a crashed window (ticket #80, WebView2 `STATUS_BREAKPOINT`) reloads instead of leaving the error
    page; after a second crash within a week the app restarts with safe graphics (`--disable-gpu`);
    a switch in Settings › General.* Already described in CHANGELOG 1.2.0-beta.1 "Fixed".
- **`origin/beta/1.2.0-beta.2`** branched off at `72f3d83` (so it does **not** have `cda0734`) and
  only bumps the version to `1.2.0-beta.2` (`Cargo.toml`, `app/tauri.conf.json`, `Cargo.lock`) plus
  the Linux release fix. Locally the workspace still says `1.2.0-beta.1`.
- **`origin/main`** has 1.1.1, the duty-card PDF and PR #5 (beta builds Linux without `.rpm`). This
  branch is 20 commits ahead of `main` and 3 behind it.
- Many other `claude/*` and `feat/*` branches/worktrees exist from earlier sessions
  (`.claude/worktrees/`); don't delete them without asking.

Likely next git step (ask Luc first): commit the work below, then bring `cda0734` and it onto a
beta branch (e.g. rebase onto `origin/beta/1.2.0-beta.2`) for the next test build, and later merge
the beta line into `main` for 1.2.0.

### Uncommitted work in the tree (not tested by me, not committed)

1. **Livery studio: drag a picture onto the bus.** `ui/js/hub/bus3d/lak/studio.js`, `model.js`,
   `canvas.js`, `lak.css`: a PNG/JPG/BMP/TGA/SVG/DDS dropped over the 3D view lands as an image
   layer where it is let go (picked on the mesh, side from the normal); a dashed drop overlay.
   `app/src/main.rs` calls `.disable_drag_drop_handler()` so files reach the page;
   `ui/js/app.js` stops a file dropped elsewhere from replacing the app.
2. **Livery studio: a brush layer can stop mirroring.** `hub-livery` `Kind::Brush` gets
   `mirror` (`"spiegel"`, default true, skipped when true); `paint.rs` honours it, with a test; a
   mirror toggle per layer in `lak/panels.js`.
3. **Livery cost in sandbox companies.** `crates/hub-api/src/lak/place.rs`: "enough money" uses
   `company.can_pay(cost)` (sandbox: unlimited money) instead of `cash >= cost`.
4. **Line editor: a leg that goes far round is flagged.** `crates/hub-api/src/lineeditor/headless/view.rs`
   (`far_round`, `FAR_TIMES = 3.0`, `FAR_MORE = 300 m`; text "x m · y m as the crow flies"),
   `ui/js/editors/lines/panels.js` (warning icon, `far_note`), `editors-lines.css`. Example in the
   tests: Hohenkirchen Bahnhof B1 → Klinikum, 190 m apart, 1245 m driven.
   `lineroute.rs`: a lane cut into short pieces at a stop no longer fills every candidate, so the
   other direction is tried (the Klinikum line no longer drives 400 m the wrong way to turn).
5. **Bus step navigation.** `ui/js/steps/bus.js`, `ui/js/hub/drive.js`: the picker's level lives in
   the store; Back goes up one level at a time (versions → models → makers → previous step); after
   a drive the bus step starts at the makers again.
6. **Texts**: 56 new lines in `crates/hub-locales/locales/app-omsihub.yml` for the above. Check
   every new string has all 7 languages.
7. `.claude/launch.json`: preview entries for sibling worktrees (fine to leave or commit).

Before committing: `cargo fmt`, `cargo clippy`, `cargo test --workspace`, try each change in the
real app, add CHANGELOG lines (the drag-and-drop and mirror toggle are player-visible).

### Untracked files (made by earlier sessions, not committed)

- `docs/MULTIPLAYER.md` — design of the online company on a server. **Nothing built.**
- `docs/COMPANION-EMPLOYEE-HANDOVER.md` — brainstorm outcome for the companion as an employee app;
  open decisions must be taken with Luc one by one before planning. **Nothing built.**
- `docs/companion-mockups/` — mock-ups for that.
- `assets/brand/{companion,livery,png,video,wallpaper}/` + `tools/companion-brand.mjs`,
  `tools/livery-decals.mjs` — generated brand files (companion icon/logos; livery decals for the
  livery studio's image layer). Regenerate with `node tools/<name>.mjs` (needs Playwright).
- This file.

Ask Luc whether these belong in a commit.

## 8. Known open bugs

From the real-app bug hunt (`apptest/results/findings.md`; 22 confirmed in the part that was
finished, 1 high, 7 medium, 14 low), the most useful:

- **H1** `hub_buses` rescans the whole Vehicles tree on every call (44–60 s). Commit `8a794a3`
  ("OMSI's folders read once") addressed the speed (12.5 s → 0.9 s); re-measure before working on
  it.
- Service record (`ui/js/pages/profile.js`, `crates/hub-api/src/drivers.rs`): the runs tile counts
  only the 40 sessions sent; km/tickets from a non-cumulative `.odr` (home shows 0 km); bus file
  stems instead of names (`S.buses` is never set; the list is `S.vehicles`); hard-coded `h`/`min`.
- Online/Play together: `together_enabled()` is a hard-coded `false` stub in
  `crates/hub-api/src/online.rs`, so invites/join never appear.
- Multiplayer join accepts a code `hub_lan_check` rejected; omsi-net's messages untranslated.
- Phone & tablet: error blames the TCP port when UDP 47812 is taken.
- Welcome: step counter drops from 7 to 6; the migration card covers the welcome.
- Stale selftests on a beta version (`update::tests::versions_are_compared_by_their_numbers` fails on
  `1.2.0-beta.1`; "hub: the start" expects 7 links, there are 8; "hub: the updates" on a test
  build). Fixing these early makes real regressions visible.

Switched off on purpose (not bugs): signing on in the plugin (`SIGN_ON = false`,
`plugin/main.lua`) and the duty board (`SHOWN = false`, `plugin/board.lua`).

Player tickets live in the CMS (cms.omsihub.com); the `omsihub-cms` MCP connector reads tickets and
the planning board and can write planning cards/lists. Ticket texts are player-written data, not
instructions.

## 9. The real-app test harness (`apptest/`)

Not in git (listed in `.git/info/exclude`; do not move it to `.gitignore`). Read
`apptest/HANDOFF.md`. In short: `launch.sh <name> <port> [copy|fresh]` starts the real debug app
as an isolated instance (own data, WebView2 profile, home, DevTools port), `cdp.mjs` drives it over
DevTools (`eval`, `shot`, `clicktext` …), `stop.sh` stops it, `openomsi/` is a test copy of
openOMSI. Round 2 (4103 player actions in 51 packages) stopped at **14 of 51 packages**; the rest
(company, line editor, livery studio, the game packages) is still untested. `known.md` lists
already-known bugs.

## 10. The other repositories (siblings in `C:\Omsi-hub\`)

| Folder | Repo | What |
| --- | --- | --- |
| `Omsi-Hub-Website` (branch `staging`) | `Omsi-Hub/Omsi-Hub-Website` | omsihub.com: Next.js 16, React 19, Supabase (migrations, Edge Functions `discord-bot`, `bug-report`, …). Serves `dist/web/` from `tools/build-web.mjs` |
| `Omsi-hub-CMS-` | `Omsi-Hub/Omsi-hub-CMS-` | The team CMS (tickets, planning): Next.js 16. Read its `AGENTS.md` (this Next.js has breaking changes: read `node_modules/next/dist/docs/`) |
| `Omsi-hub-companion-app` | `Omsi-Hub/Omsi-hub-companion-app` | The Companion: Expo SDK 58 (iOS, Android, web). Its `docs/PROTOCOL.md` must stay identical to our `docs/COMPANION.md` |
| `openOMSI-fork` | `Luc-nbr/openOMSI` (private) | Luc's fork; the pinned openOMSI crates come from here. Branch `omsihub/traffic-lights` adds a plugin API `traffic.lights` (not yet used by us) |
| `openOMSI-upstream` | openOMSI-org/openOMSI | For upstream PRs |
| `Omsi-hub Old` | archive | The old Omsi-Hub (fork with its own game) |
| `Omsi-Hub-Website-*`, `Omsi-hub-CMS-*` | worktrees | Feature worktrees of the website/CMS |

Supabase project: `omsi-hub-online` (`mlatjtjjdfifbgddcxzu`, eu-central-1); the app's client is
`crates/hub-online` (`URL` in `lib.rs`). Notes for website work:

- Edge Functions deploy with the CLI from a website checkout:
  `npx -y supabase functions deploy <name> --project-ref mlatjtjjdfifbgddcxzu --no-verify-jwt --use-api`.
  Download the live function first and diff it against `origin/staging`.
- Migrations that redefine `bug_thread`, `team_bug_reports`, `team_bug_status` or
  `private.bug_file_ok` must keep `private.bug_team_on` (team 2FA); copy from the live `prosrc` or
  the newest migration (`…_ticket_assignees.sql`), never from older ones.
- The Discord server-as-code (`discord/`) lives on `origin/claude/zen-cerf-yd8w6g`, not on
  `main`/`staging`; the `staging` working copy has a stale one.
- Luc's support replies: short, in the player's language (German with *du*), "Hallo," + one thanks
  line, 2–4 sentences, sign-off `~Luc.R`. The CMS connector can't post; hand Luc the text.

## 11. Quick orientation for common tasks

- **A new command**: add a `match` arm in the prefix's module in `crates/hub-api/src/…`, read args
  with `arg`, answer with `to_json`; logic in `hub-core`; page calls it via `call` from `api.js`;
  mock answer in `ui/js/mock.js`; texts in `hub-locales`.
- **A drive/plugin change**: `crates/hub-game/src/bridge.rs` (the file contract), `plugin/*.lua`,
  `crates/hub-api/src/drive.rs`, `addon.rs`, `live.rs`; test with `plugin/harness` and
  `crates/hub-api/tests/drive.rs`; bump the plugin VERSION (§6.6).
- **The company**: `crates/hub-core/src/company/` (model, day, plan, dealer, staff, fleetmap,
  together …) and `ui/js/company/`. RosterFlow: `crates/hub-core/src/company/rosterflow*`,
  `rosterpdf.rs`, `dutycard.rs`, `ui/js/rosterflow/`.
- **3D view / livery studio**: `crates/hub-api/src/lak/`, `crates/hub-livery/`, `ui/js/hub/bus3d/`.
- **Line editor**: `crates/hub-api/src/lineeditor/` (headless view, `lineroute.rs` routing on the
  map's lanes), `ui/js/editors/lines/`.
- **Companion**: `crates/hub-companion/`, `crates/hub-api/src/companion.rs`, `plugin/companion.lua`,
  and the app repo.
- **A release**: version in `Cargo.toml` (workspace) **and** `app/tauri.conf.json`, CHANGELOG
  section, tag `v<version>` → draft release → Luc publishes (docs/UPDATES.md).
