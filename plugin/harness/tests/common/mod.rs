//! What the phone's tests share (`mod common;`): a plugin installed as Omsi-Hub installs it, a
//! company's duty, a game whose navigator has its board (openOMSI 0.3.38) and one without it
//! (0.3.37), and ways to look into the panels the plugin set.
#![allow(dead_code)]

use hub_game::bridge::{self, Assignment, NavLook};
use hub_game::install;
use hub_plugin_harness::{GameState, Harness, Stop, Trip};
use serde_json::{json, Value};
use std::path::PathBuf;

/// The phone's panel.
pub const PHONE: &str = "omsihub_nav";

/// A plugins folder of its own for a test, the plugin installed in it as Omsi-Hub installs it.
pub fn installed(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-phone-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let plugins = d.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    install::plugin_dir(&plugins)
}

/// When the assignment was given: today.
pub fn issued() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 3600 * 3600
}

/// A company's duty D-12 on line 10, tour 2, taken as soon as the player is in the bus.
pub fn assignment() -> Assignment {
    Assignment { issued: issued(), company: "Stadtbus Berlin".into(), duty: "D-12".into(), line: "10".into(), tour: "2".into(), auto_start: true, ..Assignment::default() }
}

/// The same on the navigator's light look.
pub fn light(a: Assignment) -> Assignment {
    Assignment { nav: Some(NavLook { light: true, phone: None }), ..a }
}

/// A game whose navigator has its board (`omsi.ui.version` 5, 360 wide with 420 points of room
/// under the map): two trips of line 10 of three stops each, their termini padded with the
/// line's code as timetables have them.
pub fn docked_game() -> GameState {
    let stop = |id: i64, name: &str, t: f64| Stop { id, name: name.into(), arr: t, dep: t };
    let trip = |name: &str, from: &str, to: &str, dep: f64, arr: f64| Trip {
        name: name.into(),
        line: "10".into(),
        terminus: format!("{to}          10"),
        departure: dep,
        arrival: arr,
        stops: vec![stop(1, from, dep), stop(2, "Markt", dep + 300.0), stop(3, to, arr)],
    };
    GameState {
        ui_version: 5,
        dock: Some((360.0, 420.0, 0.0, true)),
        trips: vec![trip("Zoo - Hbf", "Zoo", "Hbf", 28_800.0, 29_400.0), trip("Hbf - Zoo", "Hbf", "Zoo", 30_000.0, 30_600.0)],
        ..GameState::default()
    }
}

/// The same trips in an openOMSI without the navigator's board (0.3.37: ui version 2).
pub fn old_game() -> GameState {
    GameState { ui_version: 2, dock: None, ..docked_game() }
}

/// The plugin installed for `tag` with the assignment `a` (none: a drive of the player's own),
/// started on `game` and run `seconds` seconds.
pub fn start(tag: &str, a: Option<Assignment>, game: GameState, seconds: usize) -> (PathBuf, Harness) {
    let dir = installed(tag);
    if let Some(a) = a {
        bridge::write_assignment(&dir, &a).unwrap();
    }
    let mut h = Harness::load(&dir, game).unwrap();
    run(&mut h, seconds);
    (dir, h)
}

/// `seconds` seconds of game.
pub fn run(h: &mut Harness, seconds: usize) {
    for _ in 0..seconds {
        h.frame().unwrap();
    }
}

/// Whether the game shows the panel `id`; the panel.
pub fn has_panel(h: &Harness, id: &str) -> bool {
    h.state.borrow().panels.contains_key(id)
}

pub fn panel(h: &Harness, id: &str) -> Value {
    h.state.borrow().panels.get(id).cloned().unwrap_or_else(|| panic!("no panel {id}: {:?}", h.state.borrow().panels.keys().collect::<Vec<_>>()))
}

/// The phone's panel.
pub fn phone(h: &Harness) -> Value {
    panel(h, PHONE)
}

/// Every element of a panel (or an element), those in rows and boxes too.
pub fn elements(v: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for c in v["children"].as_array().into_iter().flatten() {
        out.push(c.clone());
        out.extend(elements(c));
    }
    out
}

/// Whether the panel `id` has an element `f` likes.
pub fn has_element(h: &Harness, id: &str, f: impl Fn(&Value) -> bool) -> bool {
    h.state.borrow().panels.get(id).is_some_and(|p| elements(p).iter().any(f))
}

/// The first element of the panel `id` that `f` likes.
pub fn find(h: &Harness, id: &str, f: impl Fn(&Value) -> bool) -> Option<Value> {
    h.state.borrow().panels.get(id).and_then(|p| elements(p).into_iter().find(|e| f(e)))
}

/// Every text of the panel `id` (texts, badges, buttons, table cells).
pub fn texts(h: &Harness, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    for e in h.state.borrow().panels.get(id).map(elements).unwrap_or_default() {
        if let Some(t) = e["text"].as_str() {
            out.push(t.to_string());
        }
        for row in e["rows"].as_array().into_iter().flatten() {
            for cell in row.as_array().into_iter().flatten() {
                out.extend(cell.as_str().or(cell["text"].as_str()).map(String::from));
            }
        }
    }
    out
}

/// Whether a text of the panel `id` has `part` in it.
pub fn shows(h: &Harness, id: &str, part: &str) -> bool {
    texts(h, id).iter().any(|t| t.contains(part))
}

/// The phone's tabs element: the names on the tabs and the one chosen (from 1).
pub fn tabs(h: &Harness) -> Option<(Vec<String>, i64)> {
    let row = find(h, PHONE, |e| e["type"] == "row" && e["id"] == "phone_tab")?;
    let buttons = row["children"].as_array().unwrap();
    let mut names = Vec::new();
    let mut selected = 0;
    for (k, b) in buttons.iter().enumerate() {
        let id = b["id"].as_str().unwrap().strip_prefix("phone_tab_").unwrap().to_string();
        let text = b["text"].as_str().unwrap_or("");
        // (the tab shown grows and says its name; another says its count, where it has one)
        if b["grow"] == true {
            selected = k as i64 + 1;
            names.push(id);
        } else if text.is_empty() {
            names.push(id);
        } else {
            names.push(format!("{id} {text}"));
        }
    }
    Some((names, selected))
}

/// The Duty tab's trip under way: its row's elements (departure, plate, terminus, arrival), on
/// its blue card.
pub fn trip_under_way(h: &Harness) -> Option<Vec<Value>> {
    let card = find(h, PHONE, |e| e["type"] == "box" && e["color"] == "#2A75F7" && e["children"][0]["type"] == "row" && e["children"][0]["children"][1]["type"] == "badge")?;
    card["children"][0]["children"].as_array().cloned()
}

/// The tabs' ids in their order on the phone.
pub const TAB_IDS: [&str; 5] = ["ride", "stops", "duty", "messages", "bus"];

/// The player chooses the phone's tab `n` (from 1), or clicks its element `element`.
pub fn choose_tab(h: &Harness, n: i64) {
    click(h, &format!("phone_tab_{}", TAB_IDS[n as usize - 1]));
}

pub fn click(h: &Harness, element: &str) {
    h.fire("ui_click", &[json!(PHONE), json!(element)]).unwrap();
}

/// No panel was refused by the game's parser.
pub fn no_ui_errors(h: &Harness) {
    let errors = h.state.borrow().ui_errors.clone();
    assert!(errors.is_empty(), "panels the game refused: {errors:?}");
}

/// Whether a card with `part` in its text was shown.
pub fn toasted(h: &Harness, part: &str) -> bool {
    h.state.borrow().toasts.iter().any(|t| t.text.contains(part))
}
