# The plugin

Omsi-Hub's Lua plugin: the only part of Omsi-Hub that runs inside the player's openOMSI, on its
official plugin API. `hub-game` installs these files into openOMSI's `Plugins/omsihub/` and talks to
the plugin through plain files (`crates/hub-game/src/bridge.rs`).

- `main.lua`: the entry point (the duty, the reports, the handshake); the other files are its
  modules (`board`, `card`, `career`, `company`, `duty_strip`, `messages`, `ontime`, `signon`,
  `traffic`, `trip`, `watch`, `hubjson`, `layout`). `layout` gives every draggable panel the
  driver's size and place (a size row under it with F10; kept in `omsi.data.layout`). `traffic` writes what is on the road (`data/traffic.json`)
  for the company's fleet map; it only reads the game.
- `nav.lua`: the phone, docked under openOMSI's navigator map where the game has the navigator's
  board (`omsi.ui.version` 3 and on, `omsi.ui.dock`): the status line, the tabs and what they
  share (`ctx`, documented at its top). Each tab is a module of its own: `phone_ride`,
  `phone_stops`, `phone_duty`, `phone_msgs`, `phone_bus`; `phone_theme` has the inks (dark and
  light navigator) and the time colours. Without the board the modules keep panels of their own.
- `phone_menu.lua`: the company's duty in openOMSI's pause menu (`omsi.ui.set_menu_item`, after
  0.3.38): the duty, a message to control and ending the duty, each opening the phone's tab.
- `harness/`: our own test harness (`hub-plugin-harness`, on mlua). It mocks the official API
  functions the plugin uses, so `cargo test` runs the plugin without openOMSI; a panel openOMSI's
  parser would refuse is refused there too (`check_panel`, in `GameState::ui_errors`). The
  phone's tests are in `harness/tests/phone.rs`, their helpers in `harness/tests/common/`.

A change to a `.lua` file needs the plugin's `VERSION` raised in `main.lua` and in
`crates/hub-game/src/install.rs` (a test keeps the two the same).
