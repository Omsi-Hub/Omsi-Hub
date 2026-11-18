# On-board computer (boordcomputer) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace everything Omsi-Hub's plugin draws in openOMSI while driving with one on-board
computer: a Companion-app screen (`/device`) in a frameless, non-activating Omsi-Hub window over
the game.

**Architecture:** The plugin stops drawing and instead reports every driving part as data over the
Companion channel (UDP datagrams, `data/companion.json` commands). `crates/hub-companion` always
runs while Omsi-Hub runs - on `127.0.0.1` only unless Phone & tablet is on - and accepts a local
device key that Omsi-Hub hands to its own window. The Companion app gets a `/device` route laid
out as a device (layout A), built from its existing map, turn card, stop list and device screens;
its web build is embedded in Omsi-Hub as today.

**Tech Stack:** Rust (Tauri 2.12, `windows` 0.62), Lua 5.4 plugin on openOMSI's API (mlua harness),
Expo / React Native web (TypeScript, expo-router).

**Spec:** `docs/superpowers/specs/2026-11-17-boordcomputer-design.md` (read it first).

**Repositories and branches:**
- Omsi-Hub: branch `feat/boordcomputer` from `docs/boordcomputer-design` (it carries the spec and
  this plan), in a worktree `.claude/worktrees/boordcomputer-build` (CLAUDE.md: never on `main`).
- Companion app (`C:\Omsi-Hub main folder\Companion app\Omsi-hub-companion-app`): branch
  `feat/boordcomputer` from `claude/pensive-planck-lan958` (the GitHub default), in its own
  worktree beside the clone.
- **Merge order:** the Companion PR first; then Task 13 re-embeds its web build; only then the
  Omsi-Hub PR. The Omsi-Hub version that removes the plugin's panels must carry the window.

## Global Constraints

- The plugin never runs one call longer than 50 ms (`tests/budget.rs`); a datagram is at most
  8000 bytes - longer lists are paged as `trip` is (`{t, v, pages, page, <key>: [...]}`).
- Any `.lua` change raises `VERSION` in `plugin/main.lua` and `crates/hub-game/src/install.rs`
  together (test `install.rs:581`); every `tr()` literal is in `install.rs` `TEXTS` and back (test
  `install.rs:570`); `FILES` lists exactly the plugin's files. This work ships plugin **1.19.0**.
- Timer names `traffic`, `fleet`, `companion`, `dash`, `board` stay (`tests/budget.rs:139`).
- The theme stays: `offer_theme()` still calls `omsi.ui.set_theme("omsihub")`.
- The LAN listener and its firewall watch stay behind the `companion` setting exactly as now;
  without it nothing listens beyond `127.0.0.1`.
- The device window never activates (never takes the keyboard from openOMSI).
- Device sizes (logical px, window width; height = width × `DEVICE_RATIO` 0.68): S 420, M 560,
  L 760, smallest 360, largest 1200. Default corner `bottom-right`, offset 24 × 24, width M.
  A drop within 48 px of a monitor corner snaps to it.
- No tachograph anywhere.
- CI before every commit that touches Rust: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets`, `cargo test --workspace`. Companion app:
  `npm run typecheck`, `npm run lint`, `npm test`.
- Repository texts in English; player-facing strings translated (Omsi-Hub: `hub-locales`;
  Companion: `src/lib/locales/{nl,de,es}.ts`).

## Review Focus

1. **A click on the device takes the keyboard from openOMSI** (WebView2 calls `SetFocus` on its
   child). Expected: bus keys keep working after any tap. → Task 11 step "focus check".
2. **The server restarts mid-drive** (Phone & tablet toggled): the window's key must still work.
   Expected: the local key lives for Omsi-Hub's process, not the server. → Task 1 test
   `local_key_survives_a_restart`.
3. **Port 47811 is taken** by something else. Expected: the loopback listener falls back to a
   free port and the window uses that port. → Task 1 test `a_taken_port_falls_back_to_a_free_one`.
4. **The game moves to another monitor or changes resolution.** Expected: the device stays inside
   the game's monitor. → Task 11 test `a_place_off_the_monitor_is_clamped_into_it`.
5. **Long lists** (many notes, a walk-around with many items). Expected: they arrive whole, paged.
   → Task 3 test `thirty_notes_arrive_paged_and_whole`.

---

## Part 1 - Omsi-Hub: the server

### Task 1: The server always runs, on loopback, with a local key

**Files:**
- Modify: `crates/hub-companion/src/lib.rs` (`Config`, `Companion::start`, `status`)
- Modify: `crates/hub-companion/src/server.rs` (`Shared::new`, `handle` key check, bind)
- Modify: `crates/hub-api/src/companion.rs` (`start`, `start_if_on`, `hub_companion_set`)
- Modify: `app/src/main.rs:352` (call the renamed start)

**Interfaces:**
- Produces: `Config { .., lan: bool, local_key: String }`; `Companion::port(&self) -> u16`
  (existing); `hub_api::companion::start_always()` replacing `start_if_on()`;
  `hub_api::companion::device_url() -> Option<String>` =
  `http://127.0.0.1:<actual port>/device#key=<local key>`.

- [ ] **Step 1: Write the failing tests** (in `server.rs` / `lib.rs` test modules, with the
  existing `shared(tag)` helpers)
  - `the_local_key_is_let_in_without_pairing`: `req("GET", "/api/state?after=0", Some(LOCAL), "")`
    on a `Shared` built with `local_key: LOCAL` → status 200; a random other key → 401.
  - `without_lan_only_loopback_listens`: `Companion::start(Config { lan: false, port: 0, .. })` →
    the server's `addr.ip()` is `127.0.0.1`; with `lan: true` → `0.0.0.0`.
  - `a_taken_port_falls_back_to_a_free_one`: hold a `TcpListener` on `127.0.0.1:P`, start with
    `lan: false, port: P` → starts, `port() != P`. With `lan: true` the old error stays.
  - In `hub-api`: `local_key_survives_a_restart`: `device_url()` before and after `start(port)` is
    called twice has the same `#key=`.
- [ ] **Step 2: Run** `cargo test -p hub-companion -p hub-api companion` → the four fail.
- [ ] **Step 3: Implement.** The local key is a `static LOCAL_KEY: OnceLock<String>` in
  `hub-api/src/companion.rs` from `hub_companion::random_key()`; `handle` accepts
  `req.key() == Some(local_key)` before `pairing.device_of` and does not record it in `seen` or
  the devices file. `start_always()` starts with `lan: setting_on(&s)`, calls `firewall::watch`
  only when `lan`. `hub_companion_set { on }` restarts with the new `lan` instead of dropping the
  server. `status()` reports devices without the local one.
- [ ] **Step 4: Run** the same command → PASS; then the CI trio.
- [ ] **Step 5: Commit** `feat(companion): always serve on loopback, with a key for Omsi-Hub's own window`

### Task 2: The server carries the driving parts and their commands

**Files:**
- Modify: `crates/hub-companion/src/heard.rs` (new kinds), `state.rs` (`build`), `http.rs`
  (`Command`, `for_plugin`), `server.rs` (`run`)
- Modify: `docs/COMPANION.md` (the new datagrams, commands and the local device)

**Interfaces:**
- Consumes: the datagram kinds of Task 3.
- Produces: `State` JSON gains `strip`, `ontime`, `board`, `notes`, `card` - each the latest
  datagram's payload as-is (`serde_json::Value`), absent when never heard or older than
  `GAME_FRESH`. `Command` gains `start_duty`, `sign_off`, `part {id, on}`, `note_close {n}`,
  `strip_note {text}`, `strip_check {ok: bool, items: [String], text}`, `card_close` - each
  passed to the plugin under the same name (`for_plugin`).

- [ ] **Step 1: Failing tests** in `heard.rs` / `server.rs`:
  `the_strip_is_heard_and_served` (hear `{"t":"strip", ...}` → `/api/state` has `strip` equal to
  it), the same for `ontime`, `board`, `card`; `notes_come_paged` (two pages of `notes` → one list
  in order); `sign_off_reaches_the_plugin` (`POST /api/do {"do":"sign_off"}` → `plugin_reads`
  shows `{"do":"sign_off"}` with a seq).
- [ ] **Step 2: Run** `cargo test -p hub-companion` → FAIL.
- [ ] **Step 3: Implement** following how `trip`/`trips` are kept (paged `notes`) and how
  `msg_*` commands are mapped.
- [ ] **Step 4: Run** → PASS; CI trio.
- [ ] **Step 5: Commit** `feat(companion): serve the duty strip, on time, the board, notes and the trip card`

## Part 2 - Omsi-Hub: the plugin

### Task 3: Every driving part reports its facts

**Files:**
- Modify: `plugin/duty_strip.lua`, `ontime.lua`, `company.lua`, `notes.lua`, `card.lua`,
  `plugin/companion.lua`, `plugin/main.lua` (timer wiring)
- Test: `plugin/harness/tests/companion.rs`

**Interfaces:**
- Produces, each a plain Lua table with only data (no element types), mirroring what the part's
  `rows`/`elements` show today:
  - `duty_strip.facts()` → `{piece, next_break, next_relief, relief_warning, fuel = {level, kind,
    low, to_depot}, notes_in = {...}, templates = {{id, text}}, checks = {{id, text}},
    text_max, can_note, can_check, signed_on}`
  - `ontime.facts()` → `{delay, tone, headway = {ahead, behind}, connections = {...}, closed =
    {stop names}, off}`
  - `company.facts(parts, assignment)` → `{parts = {coach, fares, contract, dispatch, strip =
    bool}, coach = {...}, fares = {...}, contract = {...}, dispatch = {...}, status, can_start,
    can_sign_off}`
  - `notes.list()` (exists) → `{{n, title, text, icon, color, sticky}}`
  - `card.facts()` → the shown card's `career.evaluate` result plus `{open = bool}`, nil when none
- `companion.lua` sends `strip`, `ontime`, `board`, `card` when changed and every 5 s, and
  `notes` paged (key `notes`), each within 8000 bytes.

- [ ] **Step 1: Failing tests** in `tests/companion.rs` (existing helpers that read datagrams):
  `the_duty_strip_is_sent` (a company duty with a relief → a `strip` datagram with
  `next_relief`); `on_time_is_sent`; `the_board_parts_are_sent` (`parts.strip == true` after
  `omsi.data.board_parts` has it); `thirty_notes_arrive_paged_and_whole` (`notes.say` 30 times →
  `notes` pages, joined in order, the kept ones - `notes.lua` keeps 8 - all there, none over
  8000 bytes); `the_trip_card_is_sent_when_it_opens`.
- [ ] **Step 2: Run** `cargo test -p hub-plugin-harness` → FAIL.
- [ ] **Step 3: Implement.** Each `facts()` is the data its drawing reads today; the drawing calls
  `facts()` so both stay the same until Task 6. Raise `VERSION` (1.19.0 in both files).
- [ ] **Step 4: Run** `cargo test -p hub-plugin-harness -p hub-game` → PASS (budget test too).
- [ ] **Step 5: Commit** `feat(plugin): report the duty strip, on time, the board, notes and the trip card`

### Task 4: The plugin carries out the device's commands

**Files:**
- Modify: `plugin/companion.lua` (`carry_out`), `messages.lua` (split `answer` from `click`),
  `duty_strip.lua`, `notes.lua`, `card.lua`, `main.lua` (`hook`s for start and sign-off, parts)
- Test: `plugin/harness/tests/companion.rs`

**Interfaces:**
- Consumes: Task 2's command names.
- Produces: `messages.answer(kind, id, key)` no longer goes through `M.click`;
  `duty_strip.note(text)`, `duty_strip.check(ok, items, text)`, `notes.close(n)`, `card.close()`,
  `hook.start_duty()`, `hook.sign_off()`, `hook.part(id, on)` (stores `omsi.data.board_parts`,
  `strip` calls `strip.set_enabled`).

- [ ] **Step 1: Failing tests:** one per command - `start_duty` starts the duty (as the panel's
  `start` button did); `sign_off` runs `strip.sign_off`; `part` turns `strip` off and the next
  `board` datagram says so; `note_close` removes note n; `strip_note` with 150 characters is cut
  to `text_max` and reported; `strip_check` with `ok=false` and an item reports a defect;
  `card_close` closes the card; each acked `ok`. Keep
  `a_message_is_answered_from_the_phone_as_from_the_panel` passing.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** in `carry_out`'s style (`pcall`, ack).
- [ ] **Step 4: Run** `cargo test -p hub-plugin-harness` → PASS.
- [ ] **Step 5: Commit** `feat(plugin): carry out the on-board computer's commands`

### Task 5: Signing on comes back

**Files:** Modify `plugin/main.lua:207` (`SIGN_ON = true`); Test `tests/companion.rs` (`signs_on`).

- [ ] **Step 1:** Remove `#[ignore]` from `signs_on`; add `a_company_duty_with_a_pass_starts_at_sign_on`
  (assignment with `personnel {number, code}` → `live.signon == "signon"`, `set_auto_ibis` not yet
  on) and `a_regular_drive_is_at_work_at_once`.
- [ ] **Step 2: Run** → FAIL. **Step 3:** flip the flag; fix what the tests show.
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** `feat(plugin): sign on again, on the on-board computer`

### Task 6: The plugin stops drawing while driving

**Files:**
- Delete: `plugin/navigator.lua`, `plugin/layout.lua`
- Modify: `notes.lua` (no `put`, no toast), `messages.lua` (no panel, no F9), `board.lua`,
  `ontime.lua`, `duty_strip.lua`, `card.lua`, `main.lua` (no company panel, no F10, no
  `navigator.*`, `layout.*`), `company.lua` (drop `elements`), `signon.lua` (drop `heading`,
  `children`, input ids)
- Modify: `crates/hub-game/src/install.rs` (`FILES`, `TEXTS`), `crates/hub-game/src/compat.rs`
  (`live_link` essential; `overlay`/`messages` no longer need `ui.*`), `plugin/README.md`
- Test: `plugin/harness/tests/plugin.rs`, `crates/hub-game` tests

**Interfaces:**
- Consumes: Tasks 3-5 (the facts and commands that replace each panel).
- Produces: a plugin that calls no `omsi.ui.*` except `set_theme`/`theme`; `negotiate` reports
  `Unsupported` for a game without `send`.

- [ ] **Step 1: Rewrite the tests that assert on panels** - each keeps its intent through the
  datagrams instead: `the_duty_is_taken_its_trips_reported_and_the_notices_shown` asserts the
  `notes` datagram, not toasts; `an_older_openomsi_without_files_panels_or_game_functions_still_works`
  becomes `an_openomsi_without_send_is_refused` (compat); delete
  `the_driver_sizes_and_moves_the_panels_and_they_stay_so` and
  `in_an_openomsi_with_the_navigator_s_board_everything_is_in_the_navigator...`; add
  `nothing_is_drawn_while_driving` (a full company duty → harness `panels` empty, `toasts` empty).
- [ ] **Step 2: Run** `cargo test -p hub-plugin-harness -p hub-game` → FAIL.
- [ ] **Step 3: Remove the drawing**; prune `TEXTS` to the `tr()` literals left; `FILES` without
  the two deleted files; `compat.rs`'s `live_link` gets `essential: true` and the note
  "Omsi-Hub's on-board computer needs it".
- [ ] **Step 4: Run** the CI trio → PASS.
- [ ] **Step 5: Commit** `feat(plugin)!: nothing drawn in openOMSI while driving - the on-board computer shows it`

## Part 3 - Companion app: the `/device` screen

All in the Companion repository. Base design size of the device: **960 × 653** (the bezel
included, ratio 0.68); it is drawn at that size and scaled uniformly to the window's width.

### Task 7: Types, demo and the device's pure helpers

**Files:**
- Modify: `src/api/types.ts` (`State.strip/ontime/board/notes/card`, new `Command`s as Task 2)
- Modify: `src/api/demo.ts` (the demo fills all five, a relief at 08:50, two notes, a card after
  each trip)
- Create: `src/lib/onboard.ts`; Test: `tools/onboard.test.mts`

**Interfaces:**
- Produces in `src/lib/onboard.ts`: `keyFromHash(hash: string): string | null`;
  `startPanel(s: State): 'signon' | 'map'` (`'signon'` while `signon` is `"signon"` or `"order"`);
  `unread(s: State): number` (messages unread + notes); `scaleFor(width: number): number`
  (`width / 960`); `DESIGN = { w: 960, h: 653 }`; `type Panel = 'map' | 'stops' | 'duty' |
  'messages' | 'more' | 'signon'`.

- [ ] **Step 1: Failing tests** in `tools/onboard.test.mts`: `keyFromHash('#key=ab12')` →
  `'ab12'`, `''` → null; `startPanel` for stages `signon`/`order`/`work`/absent →
  `signon`/`signon`/`map`/`map`; `unread` 2 + 1 notes → 3; `scaleFor(480) === 0.5`.
- [ ] **Step 2: Run** `npm test` → FAIL. **Step 3: Implement.** **Step 4:** `npm test`,
  `npm run typecheck` → PASS.
- [ ] **Step 5: Commit** `feat: the on-board computer's types, demo and helpers`

### Task 8: The device shell and the map (Navigatie)

**Files:**
- Create: `src/app/device.tsx`; `src/components/onboard/Shell.tsx`, `Dock.tsx`, `NextStop.tsx`
- Modify: `src/app/_layout.tsx` (`<Stack.Screen name="device" />`; `Intro` not on `/device`),
  `src/components/Live.tsx` (no `router.push` while on `/device`),
  `src/components/MapView.web.tsx` (prop `flat?: boolean` forcing the 2D `MapCanvas`)

**Interfaces:**
- Consumes: Task 7's helpers; `store.useServer({host: location.origin, port: 0, key, name:
  'Omsi-Hub'})`; `MapView`, `TurnCard`, `Speed`, `LineBadge`, `Delay`, `C`, `t`.
- Produces: `Shell({panel, onPanel, children})` (bezel, 16:10 screen, scaled by `scaleFor`);
  `Dock({panel, onPanel, unread})` - five buttons Navigatie / Haltes / Dienst / Berichten / Meer
  (labels `t('Navigation')`, `t('Stops')`, `t('Duty')`, `t('Messages')`, `t('More')`);
  `NextStop({trip})`; when `window.__TAURI__` exists, the top edge is a drag handle
  (`getCurrentWindow().startDragging()`) and the bottom-right corner a size grip that calls
  `invoke('boordcomputer_size', {width})` while dragging and `invoke('boordcomputer_drop')` on
  release; in a plain browser neither is shown.

- [ ] **Step 1:** In the demo (`npm run web`, `/device#key=demo` → `store.useDemo()` when the key
  is `demo`): the map, turn card top left, next stop and delay top right, speed bottom right, the
  dock; screenshot at 960 and at 420 wide - identical layout, scaled.
- [ ] **Step 2: Implement** (layout A of `Overig\boordcomputer\...\indeling-companion.html`; the
  map with `flat`, `showStops`).
- [ ] **Step 3:** `npm run typecheck && npm run lint && npm test` → PASS; phone and tablet tabs
  unchanged (check `/` in the demo).
- [ ] **Step 4: Commit** `feat: the on-board computer's shell and map`

### Task 9: The panels

**Files:**
- Create: `src/components/onboard/StopsPanel.tsx`, `DutyPanel.tsx`, `MessagesPanel.tsx`,
  `MorePanel.tsx`, `SignOnPanel.tsx`, `NumberPad.tsx`, `Keyboard.tsx`, `TripCard.tsx`,
  `NoteBanner.tsx`
- Modify: `src/app/device.tsx`

**Interfaces:**
- Consumes: `State.strip/ontime/board/notes/card`, `store.send`, `StopList`, `TripsTable`,
  `IbisDevice`, `PrinterDevice`, the messages' card from `(tabs)/messages.tsx` (export it).
- Produces (each a panel over the map, closed by Navigatie):
  - `StopsPanel` - `StopList` with each stop's delay, headway ahead/behind, protected
    connections, closed stops struck through.
  - `DutyPanel` - the duty's trips (`TripsTable`), the running one, next relief and break, fuel
    with "return to the depot", notes read at sign-on, **Start duty** / **Sign off**, "Note for
    the next driver" (`Keyboard`, `strip_note`), the walk-around at sign-off (checks, `Keyboard`
    for a defect, `strip_check`).
  - `MessagesPanel` - the control room's messages with their answers, then the notes with a
    close button (`note_close`).
  - `MorePanel` - the IBIS, the ticket printer, the five board parts with switches (`part`),
    size S / M / L (`invoke('boordcomputer_size', {width: 420 | 560 | 760})`, only in Tauri).
  - `SignOnPanel` - personnel number and code on a `NumberPad`, then the duty order with an
    accept button (`sign_on`, `accept_order`); shown first per `startPanel`.
  - `TripCard` - the end-of-trip card over the map, closed with a tap (`card_close`).
  - `NoteBanner` - a new note for 6 s at the top; Berichten's button shows `unread`.
  - `Keyboard({value, max, onChange, onDone})` - QWERTY with the language's letters, backspace,
    space, done; `max` from `strip.text_max`.
- [ ] **Step 1:** Demo walk-through, one screenshot per panel; sign on with the demo's pass;
  type a 150-character note → stops at `text_max`.
- [ ] **Step 2: Implement.** **Step 3:** typecheck, lint, test → PASS.
- [ ] **Step 4: Commit** `feat: the on-board computer's panels`

### Task 10: Translations and the protocol text

**Files:** `src/lib/locales/{nl,de,es}.ts` (every new `t()` string, sorted);
`docs/PROTOCOL.md` (same text as Omsi-Hub's `docs/COMPANION.md` from Task 2).

- [ ] **Step 1:** add the strings; **Step 2:** `npm run typecheck && npm run lint && npm test`;
  demo in Dutch shows no English on `/device`.
- [ ] **Step 3: Commit** `feat: the on-board computer in Dutch, German and Spanish`; open the PR
  into `claude/pensive-planck-lan958`.

## Part 4 - Omsi-Hub: the window

### Task 11: The device window

**Files:**
- Create: `app/src/boordcomputer.rs` (placement, window, following the game)
- Create: `app/capabilities/boordcomputer.json`
- Modify: `app/src/main.rs` (module, commands, listen `game-started` / `game-ended`),
  `app/Cargo.toml` (`windows` 0.62 under `cfg(windows)`, features `Win32_Foundation`,
  `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Gdi`; tauri feature `macos-private-api`),
  `app/tauri.conf.json` (`macOSPrivateApi: true`)

**Interfaces:**
- Consumes: `hub_api::companion::device_url()`, `drive::state()` (`pid`), settings `other`.
- Produces:
  - pure: `enum Corner {TopLeft, TopRight, BottomLeft, BottomRight}`;
    `struct Rect {x, y, w, h: i32}`; `fn place(monitor: Rect, corner: Corner, offset: (i32, i32),
    width: i32, scale: f64) -> Rect` (height = width × 0.68, clamped into `monitor`, width
    clamped 360..=1200 logical); `fn snap(monitor: Rect, at: Rect) -> (Corner, (i32, i32))`
    (nearest corner; offset 0 when within 48 px); `fn visible(foreground_pid: Option<u32>,
    game_pid: Option<u32>, game_minimised: bool) -> bool`.
  - commands (also allowed to the remote page by the capability): `boordcomputer_size(width:
    f64)`, `boordcomputer_drop()`.
  - settings keys: `boordcomputer_corner` (`"bottom-right"`), `boordcomputer_offset`
    (`[24, 24]`), `boordcomputer_width` (`560`).
- [ ] **Step 1: Failing unit tests** in `boordcomputer.rs`: `the_default_is_bottom_right_24_in`;
  `a_place_off_the_monitor_is_clamped_into_it` (offset 5000 on a 1920×1080 monitor → inside);
  `widths_are_held_between_360_and_1200`; `a_drop_near_a_corner_snaps_to_it` (top-left at 30,30
  → `TopLeft`, (0,0)); `a_drop_in_the_middle_keeps_its_offset_from_the_nearest_corner`;
  `hidden_unless_the_game_is_in_front` (other pid → false; game minimised → false).
- [ ] **Step 2: Run** `cargo test -p omsi-hub boordcomputer` → FAIL.
- [ ] **Step 3: Implement.** The window: label `boordcomputer`,
  `WebviewUrl::External(device_url)`, `.decorations(false).transparent(true).always_on_top(true)
  .skip_taskbar(true).focused(false).shadow(false).resizable(false)` and `.focusable(false)`
  where Tauri 2.12 has it; on Windows also `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` on its HWND. Open
  on `game-started`, close on `game-ended`; a 250 ms thread applies `visible(...)` (Windows:
  `GetForegroundWindow` + `GetWindowThreadProcessId` + `IsIconic`; elsewhere: shown while the
  game runs) and moves it to the game window's monitor. On Linux with `WAYLAND_DISPLAY` set it is
  not opened; `drive.rs` reports a note instead (Task 12). The capability: `windows:
  ["boordcomputer"]`, `remote: {urls: ["http://127.0.0.1:*"]}`, the two commands and
  `core:window:allow-start-dragging`.
- [ ] **Step 4: Run** → PASS; CI trio.
- [ ] **Step 5: Focus check (manual, Windows, with openOMSI):** drive, tap every dock button, the
  number pad and the keyboard; after each, a bus key (doors, indicator) still works without
  clicking the game. If not, fix before going on (e.g. refuse `WM_MOUSEACTIVATE` with
  `MA_NOACTIVATE` by subclassing the window).
- [ ] **Step 6: Commit** `feat(app): the on-board computer's window over the game`

### Task 12: The game's own navigator off, full screen checked

**Files:**
- Modify: `crates/hub-api/src/drive.rs` (before starting the game), `crates/hub-game/src/gamesettings.rs`
  (if a helper is needed), `ui/js/hub/gamesettings.js` caller (`hide: ['navigator',
  'navigator_corner', 'notes']`), `crates/hub-locales` (the notes)

**Interfaces:**
- Produces: `drive::prepare_settings(home: &Path) -> Vec<String>` (the notes said to the player)
  - sets `navigator=0`, `notes=0` in `settings.cfg` with `gamesettings::set`; full screen per
  step 1; on Wayland the note "The on-board computer cannot be shown over the game here: use the
  Omsi-Hub Companion on your phone or tablet."

- [ ] **Step 1: Find out (manual)** what openOMSI's `fullscreen=1` is: start it with `fullscreen=1`
  and the device window open. If the window shows over it, full screen stays the player's; if not,
  `prepare_settings` also sets `fullscreen=0` and says "Omsi-Hub runs openOMSI in a window so the
  on-board computer can show over it." Write the finding into the spec's "Left to the builder".
- [ ] **Step 2: Failing tests** in `drive.rs`: `the_games_navigator_and_notes_are_switched_off`
  (a temp `settings.cfg` with `navigator=1` → `0`, other lines kept); plus the full-screen test
  matching step 1.
- [ ] **Step 3: Implement**, called just before `process::start`. **Step 4:** CI trio → PASS.
- [ ] **Step 5: Commit** `feat(drive): switch off openOMSI's own navigator for the on-board computer`

### Task 13: The web build, the docs and the changelog

**Files:**
- Regenerate: `assets/companion-web/` (`node tools/companion-web.mjs <Companion worktree>`), its
  `SOURCE.txt` naming the merged Companion commit
- Modify: `plugin/README.md`, `docs/COMPANION.md`, `docs/ARCHITECTURE.md`, `CHANGELOG.md`
  (`## Unreleased` → `### Changed`: the on-board computer replaces the panels in openOMSI's
  navigator; signing on is back; openOMSI's own navigator is switched off; needs an openOMSI with
  `send`)

- [ ] **Step 1:** Only after the Companion PR is merged: regenerate the web build; `cargo build -p
  hub-companion`.
- [ ] **Step 2:** `node tools/test-build.mjs`; drive a company duty in `Builds\test`: sign on with
  the number pad, every panel, a relief note, the walk-around, the trip card, S/M/L, drag to each
  corner, alt-tab (hidden), restart Omsi-Hub (same corner and size).
- [ ] **Step 3:** CI trio + `node --check` on `ui/js` and `tools` → PASS.
- [ ] **Step 4: Commit** `docs: the on-board computer` and open the PR into `main`.
