//! Omsi-Hub's plugin (`plugin/*.lua`) run without openOMSI: a Lua 5.4 state (mlua) with a mock
//! of the official plugin API's functions the plugin uses (INVENTORY §4), and a fake game
//! behind them ([`GameState`]) that a test sets up and looks at.
//!
//! The mock is written from what the plugin calls and what the API's documentation says of
//! each function; it is not openOMSI's runtime. Its rules:
//!
//! * `omsi.on(event, fn)` and `omsi.every(seconds, fn)` keep the handlers; [`Harness::frame`]
//!   is one second of game: it fires `vehicle` and `duty` when they changed, then `frame`,
//!   then the timers that are due. [`Harness::fire`] fires any other event (`trip_done` …);
//! * `omsi.files.*` read and write the plugin's own `data/` folder;
//! * `omsi.json.*` is serde_json; `omsi.log` lines go to [`GameState::log`] as
//!   `[lua omsihub] <text>`, as the game's log has them;
//! * `omsi.duty.get()` is [`GameState::duty`] (its `trip` counted from 1, as the plugin
//!   counts); `omsi.duty.start` records the start and makes it the duty;
//! * `omsi.position()` is [`GameState::position`], `omsi.traffic.list()`,
//!   `omsi.timetable.buses()` and `omsi.others()` are [`GameState::traffic`],
//!   [`GameState::timetable_buses`] and [`GameState::others`] as JSON; `omsi.map.tile_at`
//!   puts the map's coordinates on tiles of [`GameState::tile`] metres from the origin.
//!
//! A test can take functions away before the plugin runs ([`Harness::load_with`]), as an older
//! openOMSI would not have them.

use mlua::{Function, IntoLuaMulti, Lua, LuaSerdeExt, Table, Value as LuaValue};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The duty the game has.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Duty {
    pub line: String,
    pub tour: String,
    /// The trip under way, 1 = the first.
    pub trip: i64,
    pub trips: i64,
    /// Seconds late (negative: early).
    pub delay: f64,
}

/// A stop of a duty's trip.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stop {
    pub id: i64,
    pub name: String,
    pub arr: f64,
    pub dep: f64,
}

/// A trip of the duty.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trip {
    pub name: String,
    pub line: String,
    pub terminus: String,
    pub departure: f64,
    pub arrival: f64,
    pub stops: Vec<Stop>,
}

/// A card shown (`omsi.ui.toast`).
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub text: String,
    pub title: String,
}

/// The fake game.
#[derive(Debug, Clone)]
pub struct GameState {
    pub version: String,
    pub api: i64,
    pub platform: String,
    pub language: String,
    /// The clock, seconds of the day; and the Unix time `omsi.util.now` says.
    pub clock: f64,
    pub now: i64,
    /// The bus the player is in (its name), and its file.
    pub vehicle: Option<String>,
    pub bus_file: String,
    pub map: String,
    /// km/h; the passengers on board (where the bus is: `position`).
    pub speed: f64,
    pub passengers: i64,
    pub duty: Option<Duty>,
    pub trips: Vec<Trip>,
    /// What the plugin did: duties started (line, tour, trip, stop), lines of text, log lines,
    /// cards, panels (by id, as JSON), game actions, display fonts, UDP sends.
    pub started: Vec<(String, String, i64, i64)>,
    pub messages: Vec<String>,
    pub log: Vec<String>,
    pub toasts: Vec<Toast>,
    pub panels: BTreeMap<String, Value>,
    /// How far the player dragged each panel (`omsi.ui.moved`); a panel set again keeps it,
    /// one removed loses it.
    pub moved: BTreeMap<String, (f64, f64)>,
    pub actions: Vec<String>,
    pub fonts: Vec<String>,
    pub sent: Vec<(i64, String)>,
    pub ibis: Vec<String>,
    /// Where the player's bus is (x, y, heading; None on foot), the AI vehicles, the
    /// timetable's buses and the other vehicles round the bus, as the API gives them; a tile's
    /// edge.
    pub position: Option<(f64, f64, f64)>,
    pub traffic: Vec<Value>,
    pub timetable_buses: Vec<Value>,
    pub others: Vec<Value>,
    pub tile: f64,
    /// `omsi.ui.version` (2: an openOMSI before the navigator's board, as 0.3.37; 5: 0.3.38),
    /// the navigator's board `omsi.ui.dock` says (width, room, height, whether it is this
    /// plugin's; None: no board shown) - `omsi.ui.dock` is there from version 3 on -, and
    /// whether the duty's trip under way is done (at its terminus).
    pub ui_version: i64,
    pub dock: Option<(f64, f64, f64, bool)>,
    pub trip_done: bool,
    /// The panels the game refused (`omsi.ui.set` gave false): the panel's id and why, as
    /// openOMSI's own panel parser says it ([`check_panel`]).
    pub ui_errors: Vec<String>,
    /// The fuel in the tank, litres (`omsi.bus.fuel`; None: the bus does not tell), and how
    /// many of the passengers stand (`omsi.bus.passengers` gives total, seated, standing).
    pub fuel: Option<f64>,
    pub standing: i64,
    /// The tickets sold and the money taken this session (`omsi.bus.sales`); the bus's damage
    /// (`omsi.bus.damage`: crashes, the last impact's kJ, the repair's minutes; None: nothing
    /// to repair); how many stops of the trip under way the bus passed and the names of those
    /// closed today (`omsi.duty.stops` marks them `passed` and `closed`).
    pub sales: (i64, f64),
    pub damage: Option<(i64, f64, f64)>,
    pub passed: usize,
    pub closed: Vec<String>,
    /// The plugin's lines in the pause menu (`omsi.ui.set_menu_item`, openOMSI after 0.3.38):
    /// id, label and icon, in the order first set; None: a game without the functions.
    pub menu: Option<Vec<MenuLine>>,
}

/// A line of the pause menu: its id, label and icon (`omsi.ui.set_menu_item`).
pub type MenuLine = (String, String, Option<String>);

impl Default for GameState {
    fn default() -> GameState {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        GameState {
            version: "0.2.1500".into(),
            api: 1,
            platform: "windows".into(),
            language: "en".into(),
            clock: 28_800.0,
            now,
            vehicle: Some("MAN SD202".into()),
            bus_file: "Vehicles/MAN_SD202/SD202.bus".into(),
            map: "maps/Grundorf/global.cfg".into(),
            speed: 0.0,
            passengers: 0,
            duty: None,
            trips: Vec::new(),
            started: Vec::new(),
            messages: Vec::new(),
            log: Vec::new(),
            toasts: Vec::new(),
            panels: BTreeMap::new(),
            moved: BTreeMap::new(),
            actions: Vec::new(),
            fonts: Vec::new(),
            sent: Vec::new(),
            ibis: Vec::new(),
            position: None,
            traffic: Vec::new(),
            timetable_buses: Vec::new(),
            others: Vec::new(),
            tile: 300.0,
            ui_version: 2,
            dock: None,
            trip_done: false,
            ui_errors: Vec::new(),
            fuel: None,
            standing: 0,
            sales: (0, 0.0),
            damage: None,
            passed: 0,
            closed: Vec::new(),
            menu: None,
        }
    }
}

/// Every function the mock offers, as `omsi.game.has` names them.
pub const FUNCTIONS: &[&str] = &[
    "on",
    "every",
    "log",
    "message",
    "info",
    "has_vehicle",
    "speed",
    "send",
    "util.now",
    "files.read",
    "files.write",
    "files.append",
    "json.encode",
    "json.decode",
    "ui.set",
    "ui.toast",
    "ui.remove",
    "ui.update",
    "ui.focus",
    "ui.focused",
    "ui.moved",
    "duty.get",
    "duty.trips",
    "duty.stops",
    "duty.trip_stops",
    "duty.start",
    "duty.finish",
    "duty.skip_to",
    "duty.set_auto_ibis",
    "duty.type_ibis",
    "duty.set_closed_stops",
    "bus.file",
    "bus.passengers",
    "bus.fuel",
    "bus.sales",
    "bus.damage",
    "bus.set_display_font",
    "game.version",
    "game.api",
    "game.platform",
    "game.settings",
    "game.action",
    "game.has",
    "map.info",
    "map.tile_at",
    "audio.play",
    "position",
    "others",
    "traffic.list",
    "timetable.buses",
];

/// The handlers and timers, in Lua (the harness's own).
const PRELUDE: &str = r#"
local handlers, timers = {}, {}
function omsi.on(event, fn)
  handlers[event] = handlers[event] or {}
  table.insert(handlers[event], fn)
end
function omsi.every(seconds, fn)
  table.insert(timers, { every = seconds, left = seconds, fn = fn })
end
function omsi._fire(event, ...)
  for _, fn in ipairs(handlers[event] or {}) do fn(...) end
end
function omsi._tick(dt)
  for _, t in ipairs(timers) do
    t.left = t.left - dt
    if t.left <= 1e-9 then
      t.left = t.left + t.every
      t.fn()
    end
  end
end
"#;

/// The plugin running on the mock.
pub struct Harness {
    pub lua: Lua,
    pub state: Rc<RefCell<GameState>>,
    pub plugin_dir: PathBuf,
    seen_vehicle: Option<String>,
    seen_duty: Option<(String, String)>,
}

fn to_lua(lua: &Lua, v: &Value) -> mlua::Result<LuaValue> {
    lua.to_value_with(v, mlua::serde::SerializeOptions::new().serialize_none_to_null(false).serialize_unit_to_null(false))
}

fn from_lua(lua: &Lua, v: LuaValue) -> mlua::Result<Value> {
    lua.from_value_with(v, mlua::serde::DeserializeOptions::new().deny_unsupported_types(false))
}

fn stops_json(stops: &[Stop]) -> Value {
    Value::Array(stops.iter().enumerate().map(|(k, s)| json!({ "id": s.id, "number": k + 1, "name": s.name, "arr": s.arr, "dep": s.dep, "arrival": s.arr, "stops": true })).collect())
}

impl Harness {
    /// The plugin in `plugin_dir` (its `main.lua` and modules) started on `state`.
    pub fn load(plugin_dir: &Path, state: GameState) -> mlua::Result<Harness> {
        Harness::load_with(plugin_dir, state, "")
    }

    /// The same, with `before` (Lua) run ahead of the plugin's code: `omsi.files = nil` takes
    /// the file functions away, as an older openOMSI.
    pub fn load_with(plugin_dir: &Path, state: GameState, before: &str) -> mlua::Result<Harness> {
        let lua = Lua::new();
        let state = Rc::new(RefCell::new(state));
        let package: Table = lua.globals().get("package")?;
        package.set("path", format!("{}/?.lua", plugin_dir.to_string_lossy().replace('\\', "/")))?;
        let omsi = api(&lua, &state, plugin_dir)?;
        lua.globals().set("omsi", omsi)?;
        lua.load(PRELUDE).set_name("harness").exec()?;
        if !before.is_empty() {
            lua.load(before).set_name("before").exec()?;
        }
        let main = std::fs::read_to_string(plugin_dir.join("main.lua")).map_err(mlua::Error::external)?;
        lua.load(main).set_name("main.lua").exec()?;
        Ok(Harness { lua, state, plugin_dir: plugin_dir.to_path_buf(), seen_vehicle: None, seen_duty: None })
    }

    /// An event to the plugin's handlers, its arguments as JSON (`null` is `nil`).
    pub fn fire(&self, event: &str, args: &[Value]) -> mlua::Result<()> {
        let omsi: Table = self.lua.globals().get("omsi")?;
        let fire: Function = omsi.get("_fire")?;
        let mut all = vec![LuaValue::String(self.lua.create_string(event)?)];
        for a in args {
            all.push(to_lua(&self.lua, a)?);
        }
        fire.call::<()>(mlua::MultiValue::from_vec(all))
    }

    /// One second of game.
    pub fn frame(&mut self) -> mlua::Result<()> {
        let vehicle = self.state.borrow().vehicle.clone();
        if vehicle != self.seen_vehicle {
            self.seen_vehicle = vehicle.clone();
            self.fire("vehicle", &[vehicle.map_or(Value::Null, Value::String)])?;
        }
        self.check_duty()?;
        self.fire("frame", &[json!(1.0)])?;
        self.check_duty()?;
        let omsi: Table = self.lua.globals().get("omsi")?;
        omsi.get::<Function>("_tick")?.call::<()>(1.0)?;
        self.check_duty()
    }

    fn check_duty(&mut self) -> mlua::Result<()> {
        let duty = self.state.borrow().duty.as_ref().map(|d| (d.line.clone(), d.tour.clone()));
        if duty != self.seen_duty {
            self.seen_duty = duty.clone();
            match duty {
                Some((l, t)) => self.fire("duty", &[json!(l), json!(t)])?,
                None => self.fire("duty", &[Value::Null, Value::Null])?,
            }
        }
        Ok(())
    }

    /// The player chooses the plugin's line `id` of the pause menu: the menu closes and the
    /// plugin hears `menu_item` (in its next frame in the game; here at once). Whether it had
    /// that line.
    pub fn pick_menu_item(&self, id: &str) -> mlua::Result<bool> {
        let there = self.state.borrow().menu.as_ref().is_some_and(|m| m.iter().any(|l| l.0 == id));
        if there {
            self.fire("menu_item", &[json!(id)])?;
        }
        Ok(there)
    }

    /// The game's log, as one text.
    pub fn log_text(&self) -> String {
        self.state.borrow().log.join("\n")
    }
}

/// The `omsi` table: the mock API on `state`.
fn api(lua: &Lua, state: &Rc<RefCell<GameState>>, plugin_dir: &Path) -> mlua::Result<Table> {
    let omsi = lua.create_table()?;
    omsi.set("data", lua.create_table()?)?;
    let data_dir = plugin_dir.join("data");

    macro_rules! f {
        ($t:expr, $name:expr, |$lua:ident, $s:ident, $args:ident : $ty:ty| $body:expr) => {{
            let st = Rc::clone(state);
            $t.set(
                $name,
                lua.create_function(move |$lua, $args: $ty| {
                    #[allow(unused_variables)]
                    let $s = &st;
                    $body
                })?,
            )?;
        }};
    }

    // the top level
    f!(omsi, "log", |_l, s, text: String| {
        s.borrow_mut().log.push(format!("[lua omsihub] {text}"));
        Ok(())
    });
    f!(omsi, "message", |_l, s, a: (String, Option<f64>)| {
        s.borrow_mut().messages.push(a.0);
        Ok(())
    });
    f!(omsi, "info", |l, s, _a: ()| {
        let g = s.borrow();
        let mut v = json!({ "version": g.version, "clock": g.clock });
        if let Some(d) = &g.duty {
            v["line"] = json!(d.line);
            v["tour"] = json!(d.tour);
            v["delay"] = json!(d.delay);
        }
        to_lua(l, &v)
    });
    f!(omsi, "has_vehicle", |_l, s, _a: ()| Ok(s.borrow().vehicle.is_some()));
    f!(omsi, "position", |l, s, _a: ()| match s.borrow().position {
        Some((x, y, h)) => (x, y, 0.0, h).into_lua_multi(l),
        None => Ok(mlua::MultiValue::new()),
    });
    f!(omsi, "others", |l, s, _a: Option<f64>| to_lua(l, &Value::from(s.borrow().others.clone())));
    let traffic = lua.create_table()?;
    f!(traffic, "list", |l, s, _a: Option<f64>| to_lua(l, &Value::from(s.borrow().traffic.clone())));
    omsi.set("traffic", traffic)?;
    let timetable = lua.create_table()?;
    f!(timetable, "buses", |l, s, _a: ()| to_lua(l, &Value::from(s.borrow().timetable_buses.clone())));
    omsi.set("timetable", timetable)?;
    f!(omsi, "speed", |_l, s, _a: ()| Ok(s.borrow().speed));
    f!(omsi, "send", |_l, s, a: (i64, String)| {
        s.borrow_mut().sent.push(a);
        Ok(true)
    });

    let util = lua.create_table()?;
    f!(util, "now", |_l, s, _a: ()| Ok(s.borrow().now));
    omsi.set("util", util)?;

    // the plugin's own files
    let files = lua.create_table()?;
    {
        let dir = data_dir.clone();
        files.set("read", lua.create_function(move |_, name: String| Ok(std::fs::read_to_string(dir.join(name)).ok()))?)?;
        let dir = data_dir.clone();
        files.set(
            "write",
            lua.create_function(move |_, (name, text): (String, String)| {
                let _ = std::fs::create_dir_all(&dir);
                Ok(std::fs::write(dir.join(name), text).is_ok())
            })?,
        )?;
        let dir = data_dir.clone();
        files.set(
            "append",
            lua.create_function(move |_, (name, text): (String, String)| {
                use std::io::Write;
                let _ = std::fs::create_dir_all(&dir);
                let ok = std::fs::OpenOptions::new().create(true).append(true).open(dir.join(name)).and_then(|mut f| f.write_all(text.as_bytes())).is_ok();
                Ok(ok)
            })?,
        )?;
    }
    omsi.set("files", files)?;

    let jsn = lua.create_table()?;
    jsn.set("encode", lua.create_function(|l, v: LuaValue| Ok(serde_json::to_string(&from_lua(l, v)?).unwrap_or_default()))?)?;
    jsn.set("decode", lua.create_function(|l, text: String| serde_json::from_str::<Value>(&text).map_or(Ok(LuaValue::Nil), |v| to_lua(l, &v)))?)?;
    omsi.set("json", jsn)?;

    // the plugin's panels
    let ui = lua.create_table()?;
    let version = state.borrow().ui_version;
    ui.set("version", version)?;
    if version >= 3 {
        f!(ui, "dock", |l, s, _a: ()| match s.borrow().dock {
            Some(d) => d.into_lua_multi(l),
            None => Ok(mlua::MultiValue::new()),
        });
    }
    // (a panel the game's parser refuses is not shown: false and why, as the game answers)
    f!(ui, "set", |l, s, a: (String, LuaValue)| {
        let v = from_lua(l, a.1)?;
        let version = s.borrow().ui_version;
        if let Err(why) = check_panel(&v, version) {
            s.borrow_mut().ui_errors.push(format!("{}: {why}", a.0));
            return (false, why).into_lua_multi(l);
        }
        s.borrow_mut().panels.insert(a.0, v);
        true.into_lua_multi(l)
    });
    f!(ui, "toast", |l, s, a: (String, Option<LuaValue>)| {
        let opts = a.1.map(|o| from_lua(l, o)).transpose()?.unwrap_or(Value::Null);
        let title = opts.get("title").and_then(Value::as_str).unwrap_or("").to_string();
        s.borrow_mut().toasts.push(Toast { text: a.0, title });
        Ok(true)
    });
    f!(ui, "remove", |_l, s, id: String| {
        let mut g = s.borrow_mut();
        g.moved.remove(&id);
        Ok(g.panels.remove(&id).is_some())
    });
    f!(ui, "moved", |_l, s, id: String| {
        let g = s.borrow();
        // (no panel: nothing, as the game answers)
        let m = g.panels.contains_key(&id).then(|| g.moved.get(&id).copied().unwrap_or((0.0, 0.0)));
        Ok((m.map(|m| m.0), m.map(|m| m.1)))
    });
    f!(ui, "update", |_l, _s, _a: mlua::MultiValue| Ok(true));
    // (the pause menu's lines: as openOMSI's `ui/menu.rs` takes them - 3 a plugin, a label of
    // one line and 40 characters at most)
    if state.borrow().menu.is_some() {
        f!(ui, "set_menu_item", |l, s, a: (String, String, Option<String>)| {
            let label = a.1.split(|c: char| c.is_control() || c.is_whitespace()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
            let why = if a.0.is_empty() || a.0.chars().count() > MAX_ID {
                Some(format!("the line's id is 1 to {MAX_ID} characters"))
            } else if label.is_empty() {
                Some("the label is empty".to_string())
            } else if label.chars().count() > MAX_MENU_LABEL {
                Some(format!("the label is longer than {MAX_MENU_LABEL} characters"))
            } else {
                None
            };
            if let Some(why) = why {
                return (false, why).into_lua_multi(l);
            }
            let icon = a.2.filter(|i| !i.trim().is_empty());
            let mut g = s.borrow_mut();
            let menu = g.menu.get_or_insert_with(Vec::new);
            match menu.iter().position(|m| m.0 == a.0) {
                Some(i) => menu[i] = (a.0, label, icon),
                None if menu.len() >= MAX_MENU_ITEMS => return (false, format!("a plugin has {MAX_MENU_ITEMS} lines in the menu at most")).into_lua_multi(l),
                None => menu.push((a.0, label, icon)),
            }
            true.into_lua_multi(l)
        });
        f!(ui, "remove_menu_item", |_l, s, id: String| {
            let mut g = s.borrow_mut();
            let menu = g.menu.get_or_insert_with(Vec::new);
            let n = menu.len();
            menu.retain(|m| m.0 != id);
            Ok(menu.len() != n)
        });
    }
    {
        let focused = Rc::new(RefCell::new(false));
        let f2 = Rc::clone(&focused);
        ui.set(
            "focus",
            lua.create_function(move |_, on: bool| {
                *f2.borrow_mut() = on;
                Ok(())
            })?,
        )?;
        ui.set("focused", lua.create_function(move |_, ()| Ok(*focused.borrow()))?)?;
    }
    omsi.set("ui", ui)?;

    // the duty
    let duty = lua.create_table()?;
    f!(duty, "get", |l, s, _a: ()| match &s.borrow().duty {
        Some(d) => to_lua(l, &json!({ "line": d.line, "tour": d.tour, "trip": d.trip, "trips": d.trips, "delay": d.delay, "trip_done": s.borrow().trip_done })),
        None => Ok(LuaValue::Nil),
    });
    f!(duty, "trips", |l, s, _a: ()| {
        let trips: Vec<Value> =
            s.borrow().trips.iter().map(|t| json!({ "name": t.name, "line": t.line, "terminus": t.terminus, "departure": t.departure, "arrival": t.arrival, "stops": stops_json(&t.stops) })).collect();
        to_lua(l, &Value::Array(trips))
    });
    f!(duty, "stops", |l, s, _a: ()| {
        let g = s.borrow();
        let k = g.duty.as_ref().map_or(0, |d| d.trip.max(1) as usize - 1);
        let mut stops = g.trips.get(k).map_or(json!([]), |t| stops_json(&t.stops));
        // (the stops the bus passed, and those closed today)
        for (i, st) in stops.as_array_mut().into_iter().flatten().enumerate() {
            st["passed"] = json!(i < g.passed);
            if g.closed.iter().any(|c| st["name"] == c.as_str()) {
                st["closed"] = json!(true);
            }
        }
        to_lua(l, &stops)
    });
    f!(duty, "trip_stops", |l, s, n: i64| {
        let g = s.borrow();
        let t = usize::try_from(n - 1).ok().and_then(|k| g.trips.get(k));
        to_lua(l, &t.map_or(json!([]), |t| stops_json(&t.stops)))
    });
    f!(duty, "start", |_l, s, a: (String, String, Option<i64>, Option<i64>)| {
        let mut g = s.borrow_mut();
        let (trip, stop) = (a.2.unwrap_or(0), a.3.unwrap_or(0));
        g.started.push((a.0.clone(), a.1.clone(), trip, stop));
        let trips = g.trips.len() as i64;
        // (trip 0: the tour's first, or the one under way)
        g.duty = Some(Duty { line: a.0, tour: a.1, trip: trip.max(1), trips, delay: 42.0 });
        Ok(true)
    });
    f!(duty, "finish", |_l, s, _a: ()| {
        s.borrow_mut().duty = None;
        Ok(true)
    });
    f!(duty, "skip_to", |_l, s, n: i64| {
        if let Some(d) = s.borrow_mut().duty.as_mut() {
            d.trip = n;
        }
        Ok(true)
    });
    f!(duty, "set_auto_ibis", |_l, s, on: bool| {
        s.borrow_mut().ibis.push(if on { "auto on" } else { "auto off" }.into());
        Ok(true)
    });
    f!(duty, "type_ibis", |_l, s, _a: ()| {
        s.borrow_mut().ibis.push("typed".into());
        Ok(true)
    });
    f!(duty, "set_closed_stops", |_l, _s, _a: mlua::MultiValue| Ok(true));
    omsi.set("duty", duty)?;

    // the bus
    let bus = lua.create_table()?;
    f!(bus, "file", |_l, s, _a: ()| Ok(s.borrow().bus_file.clone()));
    f!(bus, "passengers", |_l, s, _a: ()| {
        let g = s.borrow();
        Ok((g.passengers, g.passengers - g.standing, g.standing))
    });
    f!(bus, "fuel", |_l, s, _a: ()| Ok(s.borrow().fuel));
    f!(bus, "sales", |_l, s, _a: ()| Ok(s.borrow().sales));
    f!(bus, "damage", |l, s, _a: ()| match s.borrow().damage {
        Some((crashes, kj, minutes)) => to_lua(l, &json!({ "crashes": crashes, "last_impact_kj": kj, "repair_minutes": minutes })),
        None => Ok(LuaValue::Nil),
    });
    f!(bus, "set_display_font", |_l, s, font: String| {
        s.borrow_mut().fonts.push(font);
        Ok(1)
    });
    omsi.set("bus", bus)?;

    // the game
    let game = lua.create_table()?;
    f!(game, "version", |_l, s, _a: ()| Ok(s.borrow().version.clone()));
    f!(game, "api", |_l, s, _a: ()| Ok(s.borrow().api));
    f!(game, "platform", |_l, s, _a: ()| Ok(s.borrow().platform.clone()));
    f!(game, "settings", |l, s, _a: ()| to_lua(l, &json!({ "language": s.borrow().language, "units": "metric" })));
    f!(game, "action", |_l, s, name: String| {
        s.borrow_mut().actions.push(name);
        Ok(true)
    });
    {
        // (whether a function is there: looked up in the `omsi` table as it is now)
        game.set(
            "has",
            lua.create_function(|l, name: String| {
                let mut t: LuaValue = l.globals().get("omsi")?;
                for part in name.split('.') {
                    t = match t {
                        LuaValue::Table(tb) => tb.get(part)?,
                        _ => return Ok(false),
                    };
                }
                Ok(matches!(t, LuaValue::Function(_)))
            })?,
        )?;
    }
    omsi.set("game", game)?;

    let map = lua.create_table()?;
    f!(map, "info", |l, s, _a: ()| to_lua(l, &json!({ "path": s.borrow().map })));
    f!(map, "tile_at", |_l, s, a: (f64, f64)| {
        let t = s.borrow().tile;
        let (tx, ty) = ((a.0 / t).floor(), (a.1 / t).floor());
        Ok((tx as i64, ty as i64, a.0 - tx * t, a.1 - ty * t))
    });
    omsi.set("map", map)?;

    let audio = lua.create_table()?;
    f!(audio, "play", |_l, _s, _a: mlua::MultiValue| Ok(true));
    omsi.set("audio", audio)?;

    Ok(omsi)
}

// --- openOMSI's panel parser, as far as the plugin can get it wrong ------------------------------

/// The limits of openOMSI's panels (`omsi-plugin/src/ui.rs`).
const MAX_ELEMENTS: usize = 200;
const MAX_TEXT: usize = 500;
const MAX_ID: usize = 64;
const MAX_DEPTH: usize = 8;
const MAX_POINTS: usize = 512;
/// The pause menu's lines a plugin has at most, and a label's characters (`ui/menu.rs`).
const MAX_MENU_ITEMS: usize = 3;
const MAX_MENU_LABEL: usize = 40;

/// Whether openOMSI's `omsi.ui.set` takes the panel `t` (as JSON) in a game whose
/// `omsi.ui.version` is `version`, and else why not - its own words where they matter. Written
/// from openOMSI's parser (`parse_panel`, upstream/main and v0.3.37): unknown keys are left
/// alone and elements of a type the game does not know are skipped (boxes and stop lists below
/// version 4), but a key of the wrong kind refuses the whole panel.
pub fn check_panel(t: &Value, version: i64) -> Result<(), String> {
    let r = Check { t, path: String::new() };
    if !t.is_object() {
        return Err("the panel is a table".into());
    }
    let mut anchors = vec!["top_left", "top", "top_right", "left", "center", "right", "bottom_left", "bottom", "bottom_right"];
    if version >= 3 {
        anchors.push("navigator");
    }
    r.one_of("anchor", &anchors)?;
    for k in ["x", "y", "width", "padding", "gap", "radius"] {
        r.number(k)?;
    }
    for k in ["background", "accent"] {
        r.color(k)?;
    }
    for k in ["visible", "clickable", "draggable", "grab"] {
        r.boolean(k)?;
    }
    let mut count = 0;
    r.children("children", 0, &mut count, version)
}

struct Check<'a> {
    t: &'a Value,
    path: String,
}

impl Check<'_> {
    fn get(&self, key: &str) -> &Value {
        self.t.get(key).unwrap_or(&Value::Null)
    }

    fn at(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn number(&self, key: &str) -> Result<(), String> {
        match self.get(key) {
            Value::Null | Value::Number(_) => Ok(()),
            _ => Err(format!("{}: a number is expected", self.at(key))),
        }
    }

    fn boolean(&self, key: &str) -> Result<(), String> {
        match self.get(key) {
            Value::Null | Value::Bool(_) => Ok(()),
            _ => Err(format!("{}: true or false is expected", self.at(key))),
        }
    }

    /// A text (a number is written as a text), at most `MAX_TEXT` characters.
    fn text(&self, key: &str) -> Result<Option<String>, String> {
        let s = match self.get(key) {
            Value::Null => return Ok(None),
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => return Err(format!("{}: a text is expected", self.at(key))),
        };
        if s.chars().count() > MAX_TEXT {
            return Err(format!("{}: longer than {MAX_TEXT} characters", self.at(key)));
        }
        Ok(Some(s))
    }

    fn color(&self, key: &str) -> Result<(), String> {
        let Some(s) = self.text(key)? else { return Ok(()) };
        let hex = s.strip_prefix('#').unwrap_or("");
        if !s.starts_with('#') || !(hex.len() == 6 || hex.len() == 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("{}: \"{s}\" is no colour (\"#RRGGBB\" or \"#RRGGBBAA\")", self.at(key)));
        }
        Ok(())
    }

    fn one_of(&self, key: &str, allowed: &[&str]) -> Result<(), String> {
        match self.text(key)? {
            Some(s) if !allowed.contains(&s.as_str()) => Err(format!("{}: \"{s}\" is not one of {allowed:?}", self.at(key))),
            _ => Ok(()),
        }
    }

    /// A list (an empty Lua table comes as either).
    fn list(&self, key: &str) -> Result<Vec<Value>, String> {
        match self.get(key) {
            Value::Null => Ok(Vec::new()),
            Value::Array(a) => Ok(a.clone()),
            Value::Object(m) if m.is_empty() => Ok(Vec::new()),
            _ => Err(format!("{}: a list is expected", self.at(key))),
        }
    }

    fn texts(&self, key: &str) -> Result<(), String> {
        for (i, v) in self.list(key)?.iter().enumerate() {
            if !(v.is_string() || v.is_number()) {
                return Err(format!("{}[{}]: a text is expected", self.at(key), i + 1));
            }
        }
        Ok(())
    }

    fn numbers(&self, key: &str) -> Result<(), String> {
        let l = self.list(key)?;
        if l.len() > MAX_POINTS {
            return Err(format!("{}: at most {MAX_POINTS} values", self.at(key)));
        }
        match l.iter().position(|v| !v.is_number()) {
            Some(i) => Err(format!("{}[{}]: a number is expected", self.at(key), i + 1)),
            None => Ok(()),
        }
    }

    /// Each of the list `key`'s tables read with `f`.
    fn tables(&self, key: &str, most: usize, what: &str, f: impl Fn(&Check) -> Result<(), String>) -> Result<(), String> {
        let l = self.list(key)?;
        if l.len() > most {
            return Err(format!("{}: at most {most} {what}s", self.at(key)));
        }
        for (i, t) in l.iter().enumerate() {
            let c = Check { t, path: format!("{}[{}]", self.at(key), i + 1) };
            if !t.is_object() {
                return Err(format!("{}: a {what} is a table", c.path));
            }
            f(&c)?;
        }
        Ok(())
    }

    fn children(&self, key: &str, depth: usize, count: &mut usize, version: i64) -> Result<(), String> {
        let list = match self.get(key) {
            Value::Null => return Ok(()),
            Value::Array(a) => a.clone(),
            Value::Object(m) if m.is_empty() => Vec::new(),
            _ => return Err(format!("{}: a list of elements is expected", self.at(key))),
        };
        if depth >= MAX_DEPTH {
            return Err(format!("{}: rows go at most {MAX_DEPTH} deep", self.at(key)));
        }
        for (i, t) in list.iter().enumerate() {
            let c = Check { t, path: format!("{}[{}]", self.at(key), i + 1) };
            if !t.is_object() {
                return Err(format!("{}: an element is a table", c.path));
            }
            *count += 1;
            if *count > MAX_ELEMENTS {
                return Err(format!("a panel has at most {MAX_ELEMENTS} elements"));
            }
            c.element(depth, count, version)?;
        }
        Ok(())
    }

    fn element(&self, depth: usize, count: &mut usize, version: i64) -> Result<(), String> {
        let ty = self.text("type")?.ok_or_else(|| format!("{}: no type", self.path))?;
        let id = self.text("id")?;
        if let Some(id) = &id {
            if id.is_empty() || id.chars().count() > MAX_ID {
                return Err(format!("{}: an id is 1 to {MAX_ID} characters", self.at("id")));
            }
        }
        let needs_id = |what: &str| if id.is_none() { Err(format!("{}: a {what} needs an id", self.path)) } else { Ok(()) };
        let weights = ["regular", "medium", "bold"];
        match ty.as_str() {
            "text" => {
                self.text("text")?;
                self.number("size")?;
                self.one_of("weight", &weights)?;
                self.one_of("align", &["left", "center", "right"])?;
                self.boolean("wrap")?;
            }
            "icon" => {
                self.text("name")?;
                self.number("size")?;
            }
            "row" => {
                self.children("children", depth + 1, count, version)?;
                self.number("gap")?;
                self.one_of("align", &["start", "center", "end", "between"])?;
            }
            "bar" => {
                self.number("value")?;
                self.number("height")?;
                self.color("background")?;
                if version >= 4 {
                    self.tables("segments", 16, "part", |c| {
                        c.number("value")?;
                        c.color("color")
                    })?;
                }
            }
            "badge" => {
                self.text("text")?;
                self.color("text_color")?;
                self.one_of("weight", &weights)?;
            }
            // (boxes and stop lists: version 4; an older game skips them)
            "box" if version >= 4 => {
                self.children("children", depth + 1, count, version)?;
                self.number("gap")?;
            }
            "stops" if version >= 4 => {
                self.tables("stops", MAX_POINTS / 4, "stop", |c| {
                    for k in ["name", "time", "expected", "tag"] {
                        c.text(k)?;
                    }
                    c.color("expected_color")?;
                    c.one_of("state", &["done", "now", "ahead"])
                })?;
                self.number("size")?;
            }
            "divider" => {}
            "space" => self.number("size")?,
            "button" => {
                needs_id("button")?;
                self.text("text")?;
                self.text("icon")?;
            }
            "image" => {
                self.text("src")?;
                self.number("width")?;
                self.number("height")?;
            }
            "checkbox" => {
                needs_id("checkbox")?;
                self.text("text")?;
                self.boolean("checked")?;
            }
            "slider" => {
                needs_id("slider")?;
                for k in ["min", "max", "value", "step"] {
                    self.number(k)?;
                }
            }
            "input" => {
                needs_id("text field")?;
                self.text("text")?;
                self.text("placeholder")?;
                self.number("max")?;
            }
            "tabs" => {
                needs_id("tabs element")?;
                self.texts("tabs")?;
                self.number("selected")?;
            }
            "chart" => {
                self.numbers("values")?;
                for k in ["min", "max", "height"] {
                    self.number(k)?;
                }
                self.one_of("style", &["line", "bars"])?;
                self.boolean("fill")?;
            }
            "table" => {
                for (i, row) in self.list("rows")?.iter().enumerate() {
                    let path = format!("{}[{}]", self.at("rows"), i + 1);
                    let Value::Array(cells) = row else { return Err(format!("{path}: a row is a list of texts")) };
                    for (j, cell) in cells.iter().enumerate() {
                        if cell.is_object() && version >= 4 {
                            let c = Check { t: cell, path: format!("{path}[{}]", j + 1) };
                            c.text("text")?;
                            c.color("color")?;
                        }
                    }
                }
                self.texts("columns")?;
                self.numbers("widths")?;
                self.number("size")?;
                if version >= 4 {
                    for a in self.list("aligns")? {
                        if !matches!(a.as_str(), Some("left" | "center" | "right")) {
                            return Err(format!("{}: {a} is no alignment (left, center, right)", self.at("aligns")));
                        }
                    }
                }
            }
            // (an element of a later version of the API, or one this game skips)
            _ => return Ok(()),
        }
        self.color("color")?;
        for k in ["grow", "clickable", "visible"] {
            self.boolean(k)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mock_offers_every_function_it_lists() {
        let dir = std::env::temp_dir().join(format!("hub-harness-self-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.lua"), "seen = 0\nomsi.every(2, function() seen = seen + 1 end)\nomsi.on('duty', function(l) last = l end)").unwrap();
        let mut h = Harness::load(&dir, GameState::default()).unwrap();
        let has: Function = h.lua.load("return omsi.game.has").eval().unwrap();
        for f in FUNCTIONS {
            assert!(has.call::<bool>(*f).unwrap(), "omsi.{f}");
        }
        for _ in 0..4 {
            h.frame().unwrap();
        }
        assert_eq!(h.lua.globals().get::<i64>("seen").unwrap(), 2);
        h.state.borrow_mut().duty = Some(Duty { line: "10".into(), ..Duty::default() });
        h.frame().unwrap();
        assert_eq!(h.lua.globals().get::<String>("last").unwrap(), "10");
        h.state.borrow_mut().duty = None;
        h.frame().unwrap();
        assert_eq!(h.lua.globals().get::<Option<String>>("last").unwrap(), None);
    }
    #[test]
    fn a_panel_is_checked_as_openomsi_reads_it() {
        let ok = json!({ "anchor": "navigator", "grab": true, "children": [
            { "type": "row", "align": "between", "children": [{ "type": "badge", "text": "307", "color": "#FFD23F", "weight": "bold" }, { "type": "text", "text": 12.5, "grow": true }] },
            { "type": "tabs", "id": "t", "tabs": ["A", "B"], "selected": 2 },
            { "type": "box", "children": [] },
            { "type": "stops", "stops": [{ "name": "Zoo", "state": "now" }] },
            { "type": "table", "rows": [["a", { "text": "+1:00", "color": "#FF453A" }]], "aligns": ["left", "right"] },
            { "type": "later_element", "anything": {} },
        ] });
        assert_eq!(check_panel(&ok, 5), Ok(()));
        // (no navigator before version 3; boxes and stop lists skipped before 4)
        assert!(check_panel(&ok, 2).unwrap_err().contains("anchor"));
        let old = json!({ "children": [{ "type": "stops", "stops": [{ "state": "gone" }] }] });
        assert_eq!(check_panel(&old, 2), Ok(()));
        for (bad, why) in [
            (json!({ "children": [{ "type": "text", "size": "12" }] }), "children[1].size: a number is expected"),
            (json!({ "children": [{ "type": "button", "text": "Go" }] }), "children[1]: a button needs an id"),
            (json!({ "children": [{ "type": "row", "children": [{ "type": "badge", "color": "yellow" }] }] }), "children[1].children[1].color"),
            (json!({ "children": [{ "type": "text", "weight": "heavy" }] }), "children[1].weight"),
            (json!({ "children": [{ "type": "tabs", "id": "t", "selected": "1" }] }), "children[1].selected"),
            (json!({ "children": [{ "type": "stops", "stops": [{ "state": "gone" }] }] }), "children[1].stops[1].state"),
            (json!({ "grab": 1 }), "grab: true or false"),
        ] {
            let e = check_panel(&bad, 5).unwrap_err();
            assert!(e.contains(why), "{e} / {why}");
        }
    }
}
