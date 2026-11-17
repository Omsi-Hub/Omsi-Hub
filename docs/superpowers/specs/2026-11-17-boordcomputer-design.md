# The on-board computer (boordcomputer): design

Luc, 2026-11-17. Mockups: `Overig\boordcomputer\.superpowers\brainstorm\9342-1794942698\content\`
(`indeling-companion.html` option A is the chosen layout, `schalen.html` option 1 the chosen
scaling).

## Intent

Omsi-Hub gets its own on-board computer: one screen that the driver uses while driving, the way a
real bus has a ticket/route terminal by the wheel. It is a sat-nav (the route, the next turn), the
sign-on unit (personnel number and code, the duty order) and the driver's companion (the trip's
stops and delay, the duty, the control room's messages, the IBIS and the ticket printer).

It **replaces openOMSI's navigator** as the place Omsi-Hub shows anything while driving. Today the
plugin docks all of its parts under openOMSI's navigator map (`plugin/navigator.lua`). That ties
Omsi-Hub's look and controls to openOMSI's navigator and its UI elements (no map of our own, no
images, typing only with F10). The on-board computer is **not built on openOMSI**: it is a window
of Omsi-Hub's own, drawn over the game, so Omsi-Hub decides what it looks like and what it can do.

It looks like a device of its own - a real on-board computer - in the **Omsi-Hub Companion app's
style** (its colours, map, turn card, stop list, speed sign).

## Goals

- One window over the game, in the corner the driver chooses, controlled with the mouse as if it
  were a touchscreen.
- Layout A: the map fills the screen; floating over it the turn card (top left, with the line's
  plate), the next stop with its delay (top right) and the speed sign with km/h (bottom right);
  along the bottom a row of large touch buttons: **Navigatie, Haltes, Dienst, Berichten, Meer**.
- It scales as one device: dragging the corner (or S / M / L) makes the whole unit bigger or
  smaller, the same layout at every size, down to a smallest size at which its text is still
  readable.
- It becomes **the only** driving UI: the plugin no longer shows anything in openOMSI's navigator
  or in panels of its own while driving.
- **Everything the plugin shows while driving today moves to the device** (Luc, 2026-11-17:
  "alles mee") - nothing a company has now is lost. See "What moves" below.

## Non-goals

- **A tachograph** (driving and rest times). Asked for at first, then dropped by Luc. No Tacho
  button, no driving time on the screen.
- Keeping openOMSI's navigator as an option. There is no setting to go back to the docked parts.
- Drawing inside the game (hooking openOMSI's renderer).
- A different look from the Companion app. It is the Companion's style, laid out as a device.

## Anti-goals (a failure even if it works)

- A click on the on-board computer takes the keyboard away from the game, so the bus stops
  answering to its keys until the driver clicks back into openOMSI.
- It needs "Phone & tablet" switched on, or opens a port on the home network, just to show a
  window on the player's own screen.
- It floats over other programs when the player alt-tabs out of the game.
- It is a phone screen in a box: small text, phone navigation, nothing that feels like a device.

## Constraints

- **The game's data only comes through the plugin.** openOMSI's official plugin API is the only
  way into the game; the plugin keeps reading the game and carrying out commands, as
  `plugin/companion.lua` already does for the phone and tablet (UDP datagrams to
  `crates/hub-companion`, commands through `data/companion.json`, positions through
  `data/pos.json`).
- **A window over a game needs a windowed or borderless game.** In exclusive full screen nothing
  is drawn over the game. Omsi-Hub already edits openOMSI's settings
  (`crates/hub-game/src/gamesettings.rs`, the `fullscreen` switch), so it can see to it - but what
  openOMSI's `fullscreen=1` actually is (exclusive or borderless) still has to be checked.
- **The plugin's 50 ms limit.** openOMSI stops a plugin when one call runs longer than 50 ms
  (`plugin/README.md`); the plugin must stay as light as it is.
- A change to a `.lua` file raises the plugin's `VERSION` in `main.lua` and
  `crates/hub-game/src/install.rs`.
- Two repositories: the screen lives in the Companion app (`Omsi-Hub/Omsi-hub-companion-app`); its
  web build is embedded in Omsi-Hub by `tools/companion-web.mjs` (`assets/companion-web`,
  `crates/hub-companion/build.rs`).

## Approaches

1. **A screen of the Companion app, in a window over the game (chosen).** The on-board computer is
   a new screen (route) in the Companion app, built from what it already has: the map
   (`MapView.web.tsx`, `Map3D`), `TurnCard`, `TurnArrow`, `StopList`, `Speed`, `SignOnScreen`,
   `Ibis`, `Printer`, the theme (`src/theme.ts`). Omsi-Hub serves it with the Companion's web build
   from its own server (`crates/hub-companion/src/web.rs`) and shows it in a window of its own.
   *Trades off:* work in two repositories, and the Companion's server must run on this computer
   even with Phone & tablet off - but nearly everything on the screen exists already, and every
   improvement to the Companion's map reaches the on-board computer too.
2. **A page of Omsi-Hub's own UI (`ui/`).** One repository, data straight from `hub-api`. *Not
   chosen:* the map, the turn card, the stops and signing on would all be built a second time,
   and two looks would have to be kept the same by hand.
3. **Drawn inside the game.** *Ruled out:* that is building on openOMSI again, and fragile.

Luc chose approach 1.

## What was decided

### The window

- Omsi-Hub (the Tauri app, `app/src/main.rs`) opens a second window for the on-board computer:
  no frame, transparent around the device, always on top **of the game**, not shown in the task
  bar, and **not activating** - a click in it does not take the keyboard from openOMSI (on
  Windows a non-activating window, `WS_EX_NOACTIVATE`; on macOS the equivalent panel behaviour).
- It is shown while the driver is in a drive (the plugin connected and the game running) and on
  the monitor the game is on; it hides when the game ends. *(Builder's call, see below: hiding
  while openOMSI is not the foreground window, so it never floats over other programs.)*
- **Moving:** dragged by its top edge (the handle in `schalen.html`).
  **Sizing:** dragged at the corner, or S / M / L. The whole device scales uniformly, aspect
  ratio fixed (16:10 screen inside the bezel), between a smallest and a largest size.
  *Proposed by Claude, not objected to:* it snaps to the nearest screen corner when let go near
  one ("netjes in de hoek"), and its corner, offset and size are remembered for the next drive.

### The screen (layout A)

- **Navigatie** (the start screen once signed on): the map following the bus, the route in blue,
  the turn card top left with distance and street and the line's plate, the next stop with its
  delay top right, the speed sign and km/h bottom right.
- **Haltes:** the trip's stops with their times and the delay at each (as the Companion's stop
  list).
- **Dienst:** the duty's trips, the one running now, when the next begins.
- **Berichten:** the control room's messages and Omsi-Hub's notifications, a red count on the
  button when something is new. Notifications that were plugin toasts/notes come here.
- **Meer:** the IBIS, the ticket printer, and the device's own settings (size, corner).
- A tap on a button opens its panel over the map; **Navigatie** goes back to the map.
- **Signing on comes back.** Luc asked for the on-board computer to be the sign-on unit, so the
  sign-on that `plugin/main.lua` switched off "for now" on 2026-10-25 (`SIGN_ON = false`) is
  switched on again, on the device. With a pass in the duty's assignment the unit starts on the
  sign-on screen - personnel number and code typed on an on-screen number pad (no F10), then the
  duty order accepted with a button - the stages `plugin/signon.lua` has ("signon", "order",
  "work"). Without a pass (sign-on off in Omsi-Hub, or a regular drive) it starts on Navigatie.
  The company panel's **Start duty** and **Sign off** buttons (`main.lua`) come to the device too.
- **Typing text** (*Claude's call*): the window never takes the keyboard, so wherever the driver
  types words (a handover note, a walk-around defect) the device shows an on-screen keyboard of
  its own, as a touchscreen does.
- **The end-of-trip card and the notes** (*Claude's call*, from "the only driving UI"):
  `card.lua`'s end-of-trip card (grade, fines, per stop) opens on the device, over the map, and
  closes with a tap; the drive watch's offences and the other notes that fell back to
  `omsi.ui.toast` arrive in Berichten, with a short banner on the device. Nothing stays drawn in
  the game.
- The labels are the Companion's own translations (nl, de, es, en), so the unit speaks the
  player's language.

### What moves (all of it)

Every part the plugin draws while driving comes to the device; where it goes is *Claude's call*:

- **Dienst** gets the duty strip's contents (`duty_strip.lua`): the trips, the one running now,
  the next relief and the next break, fuel or charge with "return to the depot", the handover
  notes read at sign-on, a note for the next driver (on-screen keyboard), and at sign-off the
  walk-around with defects (on-screen keyboard, up to 140 characters as now).
- **Haltes** gets `ontime.lua`'s view: the delay per stop, headway gaps, protected connections,
  and diversions (closed stops struck through).
- **Meer** gets the company's duty-board parts (`board.lua`: punctuality coach, fare box, line
  contract, dispatch, strip) with the driver's switch for each, next to the IBIS, the printer and
  the device's settings.
- **Berichten** gets the control room (`messages.lua`), the notes (`notes.lua`) and the drive
  watch's offences and fines.
- The company panel (`main.lua`): signing on, **Start duty**, **Sign off** - on the sign-on screen
  and in Dienst. The end-of-trip card (`card.lua`) over the map.

Because the plugin stops drawing all of it at once, the device must carry all of it **before**
the version that removes the plugin's panels ships: one release, not a first part and a rest.

### Linux (*Claude's call, proposed to Luc*)

Linux x64 is a release target. A window that stays over the game without taking the keyboard
works on X11 (best effort) and not at all on Wayland. On X11 the device is shown as on Windows;
on Wayland Omsi-Hub says the device cannot be shown over the game there and points the player to
the Companion on a phone or tablet, which shows the same screens.

### The plugin

- `plugin/navigator.lua` goes, and with it every part's docking into openOMSI's navigator **and**
  the fallback of panels of their own for an openOMSI without the navigator's board: `notes`,
  `messages`, the company's panel in `main.lua`, `board`, `ontime`, `duty_strip`, the sign-on's
  panel. What those modules *know* (the sign-on's stage, the messages read, the on-time figures,
  the duty strip's rows) stays or moves to what the plugin reports; what they *draw* goes.
- The plugin reports what the on-board computer needs and carries out its commands through the
  Companion's channel (`companion.lua`, `data/companion.json`), which already covers the IBIS,
  the ticket printer, signing on and taking a duty.
- Interface theme (`theme.json`, the loading screen and pause menu) is not part of the navigator
  and stays.

- **openOMSI's own navigator and corner notes** (*Claude's call*, from "the only option"): while
  the on-board computer is on, Omsi-Hub switches off openOMSI's own navigator map and its corner
  notes in the game's settings (`gamesettings.rs`: `navigator`, `navigator_corner`, `notes`), so
  the game shows no second map, and says so in the app.
- **The oldest openOMSI** (*Claude's call*): the device needs the plugin's `omsi.send` (today the
  optional "live_link"). It becomes essential: `compat.rs`'s `FIRST_SUPPORTED` goes up to the
  first openOMSI that has it, with the app's message to update, since a game without it would
  show nothing while driving.

### The Companion app

- Built on the Companion repository's **default branch** (`claude/pensive-planck-lan958` on
  GitHub, where `MapView.web.tsx`, `Map3D`, `Speed` and `SignOnScreen` are), and the web build is
  re-embedded from it (`tools/companion-web.mjs`).
- **The map is the flat 2D map** of the mockups (*Claude's call*): the device runs on the same
  graphics card as the game, so it does not use `Map3D`'s textured roads.

### The server

- The on-board computer talks to the same server as the phone and tablet (`crates/hub-companion`,
  the long-polled state, `/api/pos`, the route and the map's roads), but **locally**: it must
  work with Phone & tablet switched off, without a pairing code, and without opening anything to
  the home network. *(How is the builder's call - see below.)*

## Left to the builder

- **openOMSI's full screen.** Check what `fullscreen=1` is in openOMSI (exclusive or borderless)
  and make sure a drive with the on-board computer runs in a mode where the window shows; tell
  the player in the app if Omsi-Hub has to change it.
  *Status:* not checked in the real game yet. `drive::prepare_settings` currently only warns
  (a note with the `game-started` event) when `fullscreen=1` and leaves the setting alone; whether
  the window shows over openOMSI's full screen, and whether Omsi-Hub should set `fullscreen=0`
  itself, still has to be tried with the real game.
- **The local server.** How the window gets data with Phone & tablet off: e.g. the server always
  listening on `127.0.0.1` (a loopback listener needs no firewall rule - 1.2.1 fixed a crash on
  the firewall), and the window given a device key by Omsi-Hub itself instead of a pairing code;
  the LAN listener stays behind the switch as now.
- **Hiding with the game.** Following openOMSI's window: shown only while it is in front (and not
  minimised), on its monitor; what happens on alt-tab and with two monitors.
- **The smallest and largest size**, and S / M / L's sizes, at common resolutions (1080p, 1440p,
  4K) and Windows display scaling.
- **The Companion app's side:** the route for the on-board computer, its device-shaped layout
  (bezel, the bottom button row) and panels, and whether it reads the app mode (company/regular)
  or follows the duty's assignment. It must not change the phone and tablet screens.
- **What the plugin reports** in addition to what `companion.lua` sends today, for everything in
  "What moves" (the on-time figures, the duty strip, the board parts, the notifications, the
  end-of-trip card), and the commands back (the board parts' switches, notes, defects, start
  duty, sign off), within the plugin's 50 ms and 8 KB a datagram.
- Whether the phone and tablet screens also get the new parts is free; they must not lose any.
- **The release order:** the plugin without its navigator parts must not ship before Omsi-Hub
  ships the window, or a player drives with no driving UI at all. One Omsi-Hub version carries
  both (the plugin is installed by `hub-game`), with the Companion's web build re-embedded.
- `CHANGELOG.md`, the docs that describe the navigator (`plugin/README.md`, `docs/COMPANION.md`,
  `docs/ARCHITECTURE.md`) and the app's settings that mention it (`ui/js/hub/navigator.js`,
  the navigator's settings) brought in line.
