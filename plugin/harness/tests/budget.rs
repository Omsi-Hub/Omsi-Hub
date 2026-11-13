//! openOMSI gives each call into the plugin 50 ms, the game's own work inside counted, and stops
//! the plugin for the session past them (`plugin/budget.lua`). Its watch looks at the clock
//! every 10,000 Lua instructions: a call is stopped where the game was slow inside it (a read, a
//! file written) and Lua runs on after that. On a large map - many AI vehicles, a company's whole
//! day of trips, the phone and tablet on - each call of the plugin is counted here in Lua
//! instructions: those after its first call into the game, and all of them.

use hub_game::bridge::{self, Assignment};
use hub_game::install;
use hub_plugin_harness::{GameState, Harness, Trip};
use mlua::{HookTriggers, VmState};
use serde_json::json;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

/// The most Lua instructions a call may run after it called the game (openOMSI's watch looks
/// every 10,000), and in all (a millisecond or so on a slow computer).
const AFTER_THE_GAME: f64 = 10_000.0;
const IN_ALL: f64 = 100_000.0;

/// Each call timed by its name (budget.lua's `wrap`), and every function of the game's marked
/// but the clock and the few the game answers from what it has at hand: per name, the most
/// instructions in all and after the first call into the game.
const COUNT: &str = r#"
_calls = {}
local function count() return _instructions and _instructions() or 0 end
local CHEAP = { ["util.now"] = true, ["info"] = true, ["world.date"] = true, ["world.time"] = true, ["has_vehicle"] = true,
  ["speed"] = true, ["log"] = true, ["ui.focused"] = true, ["game.has"] = true, ["json.encode"] = true }
local first
local function mark(t, path, seen)
  for k, v in pairs(t) do
    local name = path .. k
    if type(v) == "function" and not CHEAP[name] and k ~= "on" and k ~= "every" and k ~= "after" and k:sub(1, 1) ~= "_" then
      t[k] = function(...)
        local r = table.pack(v(...))
        first = first or count()
        return table.unpack(r, 1, r.n)
      end
    elseif type(v) == "table" and not seen[v] and k ~= "data" then
      seen[v] = true
      mark(v, name .. ".", seen)
    end
  end
end
mark(omsi, "", {})
local budget = require("budget")
local wrap = budget.wrap
budget.wrap = function(name, fn)
  return wrap(name, function(...)
    local outer = first
    first = nil
    local start = count()
    local r = fn(...)
    local now = count()
    local c = _calls[name] or { all = 0, after = 0 }
    c.all = math.max(c.all, now - start)
    if first then c.after = math.max(c.after, now - first) end
    _calls[name] = c
    first = outer
    return r
  end)
end
"#;

fn installed(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-plugin-budget-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let plugins = d.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    install::plugin_dir(&plugins)
}

/// A large map: `n` AI vehicles (a quarter of them the timetable's buses), 60 lines of 30 tours,
/// a company's day of 3,000 trips on it, a duty of 40 trips and the phone and tablet on.
fn large_map(dir: &std::path::Path, n: usize) -> GameState {
    let issued = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 3600 * 3600;
    bridge::write_assignment(dir, &Assignment { issued, company: "C".into(), duty: "D-1".into(), line: "10".into(), tour: "2".into(), auto_start: true, ..Assignment::default() }).unwrap();
    let trips = (0..3000).map(|i| bridge::FleetTrip { line: format!("{}", i % 40), tour: format!("{}", i % 25), departure: Some(20_000.0 + i as f64 * 7.0), delay: 30.0 }).collect();
    bridge::write_fleet(dir, &bridge::Fleet { at: 1.0, company: "c".into(), map: String::new(), date: "2026-11-13".into(), trips }).unwrap();
    let want = json!({ "nums": (0..60).map(|i| format!("v{i}")).collect::<Vec<_>>(), "strs": (0..20).map(|i| format!("s{i}")).collect::<Vec<_>>() });
    std::fs::write(dir.join("data/companion.json"), json!({ "sid": "s", "port": 47812, "seq": 0, "cmds": [], "want": want }).to_string()).unwrap();
    let traffic = (0..n)
        .map(|i| {
            let kind = if i % 4 == 0 { "timetable_bus" } else { "car" };
            json!({ "id": i, "kind": kind, "name": "Vehicles\\Cars\\car.ovh", "x": 1234.5678 + i as f64, "y": -987.654321 - i as f64, "z": 12.3,
                    "heading": 123.456789, "speed": 13.37, "max_speed": 50.0, "waiting_for": "lead", "standing": 0.0, "braking": false, "blinker": 0, "line": "", "distance": 345.6 })
        })
        .collect();
    let buses = (0..n / 4)
        .map(|i| json!({ "id": i * 4, "x": 1000.0 + i as f64, "y": 2000.0, "line": format!("{}", 100 + i % 40), "tour": "3", "trip": 4, "terminus": "Hauptbahnhof", "delay": 37.2, "number": 1234, "at_stop": false, "departure": 30000 }))
        .collect();
    let lines =
        json!((0..60).map(|l| json!({ "name": format!("{l}"), "user_allowed": true, "tours": (0..30).map(|t| json!({ "number": t, "today": true })).collect::<Vec<_>>() })).collect::<Vec<_>>());
    let trip = |k: usize| Trip { name: format!("trip {k}"), line: "10".into(), ..Trip::default() };
    GameState { trips: (0..40).map(trip).collect(), position: Some((100.0, 200.0, 90.0)), traffic, timetable_buses: buses, lines, ..GameState::default() }
}

/// A minute of game on `g`; per named call the most instructions (in all, after the game).
fn calls(dir: &std::path::Path, g: GameState, before: &str) -> (Harness, Vec<(String, f64, f64)>) {
    let mut h = Harness::load_with(dir, g, &format!("{before}\n{COUNT}")).unwrap();
    let count = Rc::new(Cell::new(0u64));
    let c = count.clone();
    h.lua
        .set_hook(HookTriggers::new().every_nth_instruction(100), move |_, _| {
            c.set(c.get() + 100);
            Ok(VmState::Continue)
        })
        .unwrap();
    let c = count.clone();
    let f = h.lua.create_function(move |_, ()| Ok(c.get() as f64)).unwrap();
    h.lua.globals().set("_instructions", f).unwrap();
    for _ in 0..60 {
        h.frame().unwrap();
    }
    let rows: Vec<mlua::Table> = h.lua.load("local out = {} for k, c in pairs(_calls) do out[#out + 1] = { k, c.all, c.after } end return out").eval().unwrap();
    let mut rows: Vec<(String, f64, f64)> = rows.into_iter().map(|t| (t.get(1).unwrap(), t.get(2).unwrap(), t.get(3).unwrap())).collect();
    rows.sort_by(|a, b| b.2.total_cmp(&a.2));
    (h, rows)
}

fn too_long(rows: &[(String, f64, f64)]) -> Vec<&(String, f64, f64)> {
    rows.iter().filter(|(_, all, after)| *after > AFTER_THE_GAME || *all > IN_ALL).collect()
}

#[test]
fn no_call_does_much_on_a_large_map() {
    let dir = installed("large");
    let (h, rows) = calls(&dir, large_map(&dir, 600), "");
    eprintln!("the calls, in Lua instructions (in all, after the game): {rows:?}");
    assert!(too_long(&rows).is_empty(), "calls too long: {:?}", too_long(&rows));
    // (and all of it done: the company's trips given, the traffic and the dashboard written, the
    // map's lines sent to the phone)
    assert_eq!(h.state.borrow().ai_trips.len(), 1);
    assert_eq!(bridge::read_traffic(&dir).expect("data/traffic.json").vehicles.len(), 450);
    assert!(bridge::read_dash(&dir).is_some());
    assert!(h.state.borrow().sent.iter().any(|(_, t)| t.contains("\"t\":\"lines\"")));
    for name in ["traffic", "fleet", "companion", "dash", "board"] {
        assert!(rows.iter().any(|(n, ..)| n == name), "{name} ran");
    }
}

#[test]
fn an_openomsi_without_set_ai_trips_takes_the_buses_off_a_few_at_a_time() {
    let dir = installed("large-old");
    let (h, rows) = calls(&dir, large_map(&dir, 600), "omsi.timetable.set_ai_trips = nil");
    eprintln!("the calls, in Lua instructions (in all, after the game): {rows:?}");
    assert!(too_long(&rows).is_empty(), "calls too long: {:?}", too_long(&rows));
    // (each of the 150 timetable buses runs a line the company does not: all of them gone)
    assert_eq!(h.state.borrow().removed.len(), 150);
}

#[test]
fn a_slow_call_is_written_into_the_game_s_log_by_its_name() {
    let dir = installed("slow");
    let (mut h, _) = calls(&dir, large_map(&dir, 600), "");
    // (budget.lua's clock counting instructions: the traffic's text of 600 vehicles is "slow")
    h.lua.load("require('budget').clock = function() return _instructions() / 1e6 end").exec().unwrap();
    for _ in 0..6 {
        h.frame().unwrap();
    }
    let log = h.log_text();
    let line = log.lines().find(|l| l.contains("slow: traffic")).unwrap_or_else(|| panic!("{log}"));
    assert!(line.starts_with("[lua omsihub] slow: traffic ") && line.ends_with(" ms"), "{line}");
}
