# The plugin

Omsi-Hub's Lua plugin: the only part of Omsi-Hub that runs inside the player's openOMSI, on its
official plugin API. `hub-game` installs these files into openOMSI's `Plugins/omsihub/` and talks to
the plugin through plain files (`crates/hub-game/src/bridge.rs`).

- `main.lua`: the entry point (the duty, the reports, the handshake); the other files are its
  modules (`card`, `career`, `company`, `companion`, `duty_strip`, `messages`, `ontime`, `signon`,
  `start`, `takeover`, `traffic`, `fleet`, `trip`, `watch`, `hubjson`, `notes`, `budget`, `legs`,
  `dash`). openOMSI stops a plugin when one call into it runs longer than 50 ms, the game's time
  inside the API counted, so the plugin keeps its calls short: `budget` times every call (one past
  20 ms goes into the game's log as `slow: <name> <ms> ms`, which Omsi-Hub copies into its own),
  and a long job is done in steps, each a call of its own - the game read, the text made, the file
  written (`traffic`, `dash`), a long list in portions (`fleet`). `start` takes the duty
  (`duty.start`) in a call of its own, as taking a duty can take that long by itself.
- **The plugin shows nothing while driving.** Everything the driver is told and does while
  driving is on Omsi-Hub's on-board computer, a window of Omsi-Hub's over the game, reached
  through the companion's channel (`companion.lua`; docs/COMPANION.md): each driving part reports
  what it knows as plain facts - the duty strip with the handover notes and the walk-around
  (`duty_strip`), on time (`ontime`), the company's board parts and signing on (`company`,
  `signon`), the end-of-trip card (`card`), the notifications (`notes`: the company's notices,
  the control room's messages and instructions, the drive watch's offences, on-time alerts) - and
  the on-board computer's commands (`duty_start`, `sign_on`, `accept_order`, `sign_off`, `part`,
  `msg`, `note_close`, `strip_note`, `strip_check`, `card_close`) are carried out here. What the
  modules work out stays in the plugin: the control room's instructions carried out in the game,
  the closed stops given to openOMSI's own navigator (`duty.set_closed_stops`), the trip's
  evaluation and its report, the duty's parts and reliefs, taking a bus over, the career. An
  openOMSI without `omsi.send` cannot reach the on-board computer, so Omsi-Hub does not support it
  (`crates/hub-game/src/compat.rs`).
- `companion.lua` talks to Omsi-Hub's on-board computer and its phone and tablet companion while
  it is on (`data/companion.json`, datagrams by UDP; docs/COMPANION.md): it presses only the keys
  of the bus's IBIS, ticket printer and change dispenser, and sends what each driving part reports
  of itself (`duty_strip`, `ontime`, `company`, `card` each have `facts()`, `notes` has `list()`).
  `traffic` writes what is on the road (`data/traffic.json`) for the company's fleet map; it only
  reads the game. `fleet` gives the game the trips the company's own buses run in a company's duty
  (`data/fleet.json`, `omsi.timetable.set_ai_trips`): Omsi-Hub no longer writes that file - no
  openOMSI has `set_ai_trips` - and makes the company's own buses alone run in the game with a
  Chrono folder of the map instead (`hub_core::company::ailist`). `takeover` takes a bus over
  halfway along its trip (the assignment's `take_over`): the AI's bus of the tour off the road,
  the player's bus put where it was with the duty at its next stop - else where the fleet map has
  it, else at that stop.
- `theme.json`, `theme-loading.svg`, `theme-logo.svg`: Omsi-Hub's look for openOMSI's own screens,
  an interface theme (openOMSI's docs/THEMES.md) of the plugin's name: the loading screen with the
  logo's ring as the loading bar (`id="progress"`), the pause menu's panels of glass. `main.lua`
  puts it in force once (`ui.set_theme`), over openOMSI's own look only.
- `harness/`: our own test harness (`hub-plugin-harness`, on mlua). It mocks the official API
  functions the plugin uses, so `cargo test` runs the plugin without openOMSI.
  `cargo run -p hub-plugin-harness --example fakegame` runs the plugin on it with the companion's
  server (`crates/hub-companion`), driving a duty of two trips, for trying the Omsi-Hub Companion
  app without openOMSI: it prints the pairing code and the bus's keys the app presses.

A change to a `.lua` file needs the plugin's `VERSION` raised in `main.lua` and in
`crates/hub-game/src/install.rs` (a test keeps the two the same).
