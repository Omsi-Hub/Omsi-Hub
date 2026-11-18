//! The plugin run on the harness's mock of openOMSI's plugin API: a fake bus with a duty, a
//! few seconds of game, and what reaches Omsi-Hub through the bridge. Ported from the old
//! `omsihub-plugin/tests/plugin.rs`, which ran the plugin inside openOMSI's own runtime.

use hub_game::bridge::{self, Assignment, Inbox, InboxAction, InboxMsg, Notice, ReportReader};
use hub_game::{compat, install};
use hub_plugin_harness::{GameState, Harness, Trip, FUNCTIONS};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A plugins folder of its own for a test, the plugin installed in it as Omsi-Hub installs it.
fn installed(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-plugin-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let plugins = d.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    install::plugin_dir(&plugins)
}

/// When the assignment was given: today (one of yesterday is not believed).
fn issued() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 3600 * 3600
}

fn assignment() -> Assignment {
    let mut texts = BTreeMap::new();
    texts.insert("Start duty".to_string(), "Dienst starten".to_string());
    Assignment {
        issued: issued(),
        company: "Stadtbus Berlin".into(),
        duty: "D-12".into(),
        line: "10".into(),
        tour: "2".into(),
        auto_start: true,
        notices: vec![Notice { id: "n1".into(), title: "Depot".into(), text: "Bus 4711 is in the workshop".into(), kind: "info".into() }],
        texts,
        ..Assignment::default()
    }
}

/// The game: the duty's two trips without stops.
fn game() -> GameState {
    let trip = |n: &str| Trip { name: n.into(), line: "10".into(), ..Trip::default() };
    GameState { trips: vec![trip("Zoo - Hbf"), trip("Hbf - Zoo")], ..GameState::default() }
}

/// The port of Omsi-Hub's companion server the tests have the plugin send to.
const PORT: i64 = 47812;

/// Omsi-Hub's companion on, as the on-board computer has it: the plugin sends what it shows to
/// the companion's server (`data/companion.json`).
fn computer_on(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/companion.json"), json!({ "sid": 1, "port": PORT, "seq": 0, "cmds": [] }).to_string()).unwrap();
}

/// What the plugin sent the companion server, as JSON, of kind `t`.
fn sent(h: &Harness, t: &str) -> Vec<Value> {
    h.state.borrow().sent.iter().filter(|(p, _)| *p == PORT).filter_map(|(_, text)| serde_json::from_str::<Value>(text).ok()).filter(|v| v["t"] == t).collect()
}

/// Each `notes` datagram the plugin sent the on-board computer: its notes, as "title: text".
fn notes(h: &Harness) -> Vec<Vec<String>> {
    sent(h, "notes")
        .iter()
        .map(|p| p["notes"].as_array().cloned().unwrap_or_default().iter().map(|n| format!("{}: {}", n["title"].as_str().unwrap_or(""), n["text"].as_str().unwrap_or(""))).collect())
        .collect()
}

/// Whether the on-board computer was sent a note that says `part`, within the next seconds of
/// the game (one part a tick goes out, between the live data and the lists).
fn noted(h: &mut Harness, part: &str) -> bool {
    for _ in 0..40 {
        if notes(h).iter().flatten().any(|n| n.contains(part)) {
            return true;
        }
        h.frame().unwrap();
    }
    false
}

#[test]
fn the_duty_is_taken_its_trips_reported_and_the_notices_shown() {
    let dir = installed("full");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    computer_on(&dir);
    let mut h = Harness::load(&dir, game()).unwrap();

    // the handshake: everything the add-on uses is there
    let hello = bridge::read_hello(&dir).expect("data/hello.json");
    assert_eq!((hello.bridge, hello.plugin.as_str(), hello.api, hello.issued), (bridge::BRIDGE, install::VERSION, 1, issued()));
    let c = compat::negotiate(&hello);
    assert_eq!(c.level, compat::Level::Supported, "{c:?}");

    h.frame().unwrap();
    // (auto_start: the duty is taken as soon as the player is in the bus)
    assert_eq!(h.state.borrow().started, [("10".to_string(), "2".to_string(), 0, 0)]);
    h.frame().unwrap();
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(95.0), json!(88.0), json!(100.0)]).unwrap();
    h.frame().unwrap();
    h.state.borrow_mut().duty = None;
    h.frame().unwrap();

    let reports = ReportReader::default().read_new(&dir);
    let kinds: Vec<&str> = reports.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["duty_start", "trip", "duty_end"]);
    let trip = &reports[1];
    assert_eq!((trip.issued, trip.duty.as_str(), trip.line.as_str(), trip.tour.as_str()), (issued(), "D-12", "10", "2"));
    assert_eq!((trip.trip, trip.trip_name.as_str(), trip.how.as_str()), (1, "Zoo - Hbf", "arrived"));
    assert_eq!((trip.trips, trip.number.as_str()), (2, "10"));
    assert_eq!((trip.driving, trip.comfort, trip.tickets, trip.delay), (Some(95.0), Some(88.0), Some(100.0), Some(42.0)));
    assert_eq!(trip.clock, Some(28_800.0));

    // the notice as a note of the on-board computer, once; the company's board there too
    assert!(noted(&mut h, "Depot: Bus 4711 is in the workshop"), "{:?}", notes(&h));
    assert!(notes(&h).iter().all(|n| n.iter().filter(|n| n.contains("Bus 4711")).count() <= 1), "{:?}", notes(&h));
    for _ in 0..10 {
        h.frame().unwrap();
    }
    assert!(!sent(&h, "board").is_empty(), "the company's board");
    // (nothing of it drawn in the game)
    assert!(h.state.borrow().toasts.is_empty() && h.state.borrow().panels.is_empty());

    // a notice that comes while the game runs
    bridge::write_notices(&dir, &[Notice { id: "n2".into(), text: "Diversion on line 10".into(), ..Notice::default() }]).unwrap();
    assert!(noted(&mut h, "Diversion on line 10"), "{:?}", notes(&h));
}

#[test]
fn an_openomsi_without_send_is_refused() {
    let dir = installed("no-send");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    // (an openOMSI before `omsi.send`: the on-board computer would show nothing while driving)
    let mut h = Harness::load_with(&dir, game(), "omsi.send = nil").unwrap();
    h.frame().unwrap();
    let hello = bridge::read_hello(&dir).expect("data/hello.json");
    assert_eq!(hello.functions.get("send"), Some(&false));
    let c = compat::negotiate(&hello);
    assert_eq!(c.level, compat::Level::Unsupported, "{c:?}");
    let link = c.features.iter().find(|f| f.id == "live_link").expect("the live link");
    assert!(!link.on);
    assert_eq!(link.fallback, "Omsi-Hub's on-board computer needs it");
}

#[test]
fn an_older_openomsi_without_files_or_game_functions_reports_in_its_log() {
    let dir = installed("old");
    bridge::write_assignment(&dir, &Assignment { auto_start: false, ..assignment() }).unwrap();
    // (an openOMSI before the file functions, the panels and `game.*`)
    let mut h = Harness::load_with(&dir, game(), "omsi.files = nil; omsi.ui = nil; omsi.game = nil; omsi.json = nil").unwrap();
    // the player takes the duty from the game's menu; trips are watched one by one
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 42.0 });
    for trip in [1, 1, 2, 2, 2] {
        h.state.borrow_mut().duty.as_mut().unwrap().trip = trip;
        h.frame().unwrap();
    }
    // nothing could be written: the handshake and the reports are in the log
    assert!(!dir.join(bridge::DATA_DIR).join(bridge::REPORTS_FILE).exists());
    let (hello, reports) = bridge::from_log(&h.log_text());
    let hello = hello.expect("the handshake in the log");
    assert_eq!(hello.functions.get("files.append"), Some(&false));
    let c = compat::negotiate(&hello);
    assert_eq!(c.level, compat::Level::Limited);
    assert!(c.features.iter().any(|f| f.id == "reports" && !f.on));
    let kinds: Vec<&str> = reports.iter().map(|r| r.kind.as_str()).collect();
    assert_eq!(kinds, ["duty_start", "trip"]);
    assert_eq!((reports[1].trip, reports[1].trip_name.as_str()), (1, "Zoo - Hbf"));
    // (the notice is not put on the screen as a line of text: the on-board computer shows it)
    assert!(h.state.borrow().messages.is_empty(), "{:?}", h.state.borrow().messages);
}

#[test]
fn the_plugin_checks_every_function_the_add_on_negotiates() {
    let main = install::FILES[0].1;
    let needs = &main[main.find("local NEEDS = {").unwrap()..];
    let needs = &needs[..needs.find('}').unwrap()];
    let mut in_lua: Vec<&str> = needs.split('"').skip(1).step_by(2).collect();
    let mut in_rust: Vec<&str> = compat::FEATURES.iter().flat_map(|f| f.needs.iter().copied()).collect();
    in_lua.sort();
    in_rust.sort();
    // (a function two features need is looked up once)
    in_rust.dedup();
    assert_eq!(in_lua, in_rust);
    // (and the harness's mock of the API has each of them, so every feature is exercised)
    for n in in_rust {
        assert!(FUNCTIONS.contains(&n), "the mock lacks omsi.{n}");
    }
}

#[test]
fn an_assignment_of_an_earlier_day_is_not_taken() {
    let dir = installed("stale");
    bridge::write_assignment(&dir, &Assignment { issued: issued() - 2 * 86_400, ..assignment() }).unwrap();
    computer_on(&dir);
    let mut h = Harness::load(&dir, game()).unwrap();
    for _ in 0..30 {
        h.frame().unwrap();
    }
    // (no duty taken, no company's board)
    assert!(h.state.borrow().started.is_empty());
    assert!(sent(&h, "board").iter().all(|b| *b == json!({ "t": "board" })), "only cleared, as the drive begins: {:?}", sent(&h, "board"));
    assert_eq!(bridge::read_hello(&dir).map(|h| h.issued), Some(0));
}

/// A duty of parts as Omsi-Hub gives it (`bridge::Leg`).
fn leg(line: &str, tour: &str, first: u32, trips: u32, from_stop: u32, to_stop: u32) -> bridge::Leg {
    bridge::Leg { line: line.into(), tour: tour.into(), first, trips, from_stop, to_stop }
}

#[test]
fn a_shift_s_parts_are_chained_and_the_duty_ends_after_the_last() {
    let dir = installed("legs");
    bridge::write_assignment(&dir, &Assignment { auto_start: false, legs: vec![leg("10", "2", 1, 1, 0, 0), leg("11", "3", 2, 1, 0, 0)], ..assignment() }).unwrap();
    computer_on(&dir);
    let mut h = Harness::load(&dir, game()).unwrap();
    // (the game's command line started the first part's tour)
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 0.0 });
    h.frame().unwrap();
    assert!(h.state.borrow().started.is_empty(), "the first part is the game's");
    // the first part's last trip done: the second part taken with duty.start
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    // (in a call of its own, the next frame's: openOMSI stops a plugin whose one call runs over
    // 50 ms, and duty.start can take that long by itself; its card in the frame after)
    assert!(h.state.borrow().started.is_empty());
    h.frame().unwrap();
    assert_eq!(h.state.borrow().started, [("11".to_string(), "3".to_string(), 2, 0)]);
    assert!(noted(&mut h, "Part 2 of 2: line 11, tour 3"));
    // its last trip done: the duty ended
    h.fire("trip_done", &[json!(2), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    assert!(h.state.borrow().duty.is_none());
    assert!(noted(&mut h, "Your duty is done"));
    let reports = ReportReader::default().read_new(&dir);
    let kinds: Vec<(&str, &str)> = reports.iter().map(|r| (r.kind.as_str(), r.line.as_str())).collect();
    assert_eq!(kinds, [("duty_start", "10"), ("trip", "10"), ("duty_end", "10"), ("duty_start", "11"), ("trip", "11"), ("duty_end", "11")]);
}

#[test]
fn a_duty_with_reliefs_begins_and_ends_on_the_way() {
    let dir = installed("relief");
    // (taken over at the third stop of trip 1, handed over at the fourth stop of trip 2)
    bridge::write_assignment(&dir, &Assignment { auto_start: false, legs: vec![leg("10", "2", 1, 2, 3, 4)], ..assignment() }).unwrap();
    computer_on(&dir);
    let mut h = Harness::load(&dir, game()).unwrap();
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 0.0 });
    h.frame().unwrap();
    assert_eq!(h.state.borrow().started, [("10".to_string(), "2".to_string(), 1, 3)], "taken again from the relief stop");
    h.frame().unwrap();
    // trip 1 done: the part goes on (it ends at a relief, not at a trip's end)
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    assert!(h.state.borrow().duty.is_some());
    h.state.borrow_mut().duty.as_mut().unwrap().trip = 2;
    h.frame().unwrap();
    h.fire("stop_arrive", &[json!(null), json!(null), json!(3), json!(0.0)]).unwrap();
    assert!(h.state.borrow().duty.is_some(), "not yet at the relief stop");
    h.fire("stop_arrive", &[json!(null), json!(null), json!(4), json!(0.0)]).unwrap();
    assert!(h.state.borrow().duty.is_none(), "the relief takes over");
    let reports = ReportReader::default().read_new(&dir);
    assert!(reports.iter().any(|r| r.kind == "trip" && r.trip == 2 && r.how == "relieved"), "{reports:?}");
    assert!(noted(&mut h, "Your relief takes over here"));
}

#[test]
fn a_free_drive_says_its_route() {
    let dir = installed("free");
    bridge::write_assignment(&dir, &Assignment { auto_start: false, duty: String::new(), line: "14".into(), tour: String::new(), free_line: "14_Hbf-Zoo".into(), ..assignment() }).unwrap();
    computer_on(&dir);
    let mut h = Harness::load(&dir, game()).unwrap();
    assert!(noted(&mut h, "Free drive on line 14: 14_Hbf-Zoo"));
    assert!(h.state.borrow().started.is_empty());
}

#[test]
fn the_game_s_traffic_reaches_the_fleet_map() {
    let dir = installed("traffic");
    let mut g = game();
    g.position = Some((650.5, 1210.25, 92.0));
    g.duty = Some(hub_plugin_harness::Duty { line: "10".into(), tour: "2".into(), trip: 1, trips: 2, delay: 95.0 });
    g.speed = 31.0;
    g.traffic = vec![
        json!({ "id": 7, "kind": "car", "x": 700.0, "y": 1200.0, "heading": 270.0 }),
        json!({ "id": 9, "kind": "timetable_bus", "x": 100.0, "y": 50.0, "heading": 45.0 }),
        json!({ "id": 11, "kind": "tram", "x": 820.04, "y": 1190.0 }),
    ];
    g.timetable_buses = vec![json!({ "id": 9, "line": 5, "tour": "3", "trip": 2, "terminus": "Zoo", "delay": 125.0, "number": 4711, "x": 100.0, "y": 50.0, "at_stop": true })];
    g.others = vec![json!({ "kind": "ai", "x": 1.0, "y": 2.0 }), json!({ "kind": "player", "name": "Sam", "x": 905.0, "y": 610.0, "heading": 180.0 })];
    let mut h = Harness::load(&dir, g).unwrap();
    // (the game read, the text made, the file written: a call each)
    for _ in 0..3 {
        h.frame().unwrap();
    }
    let t = bridge::read_traffic(&dir).expect("data/traffic.json");
    assert_eq!(t.map, "maps/Grundorf/global.cfg");
    let p = t.player.expect("the player's bus");
    assert_eq!((p.x, p.y, p.heading, p.speed), (650.5, 1210.3, 92.0, 31.0));
    assert_eq!((p.line.as_str(), p.tour.as_str(), p.delay), ("10", "2", Some(95.0)));
    // (the timetable's bus only in its own list, with its heading from the traffic's)
    let v: Vec<_> = t.vehicles.iter().map(|v| (v.id, v.x, v.y, v.heading, v.kind.as_str())).collect();
    assert_eq!(v, [(Some(7), 700.0, 1200.0, 270.0, "car"), (Some(11), 820.0, 1190.0, 0.0, "tram")]);
    let b = &t.buses[0];
    assert_eq!((b.line.as_str(), b.tour.as_str(), b.number.as_str(), b.delay, b.heading, b.at_stop), ("5", "3", "4711", Some(125.0), 45.0, true));
    assert_eq!((t.players.len(), t.players[0].name.as_str()), (1, "Sam"));
    // (where the tiles lie: the player's place on its tile of 300 m)
    let r = t.reference.expect("a place by its tile");
    assert_eq!((r.tile_x, r.tile_y, r.local_x, r.x), (2, 4, 50.5, 650.5));

    // on foot, without traffic: empty lists
    h.state.borrow_mut().position = None;
    h.state.borrow_mut().traffic.clear();
    h.state.borrow_mut().timetable_buses.clear();
    h.state.borrow_mut().others.clear();
    // (the next read two seconds after the last, then its text, then the file)
    for _ in 0..4 {
        h.frame().unwrap();
    }
    let t = bridge::read_traffic(&dir).expect("written again");
    assert!(t.player.is_none() && t.vehicles.is_empty() && t.buses.is_empty() && t.reference.is_none(), "{t:?}");
    // (openOMSI's own encoder writes them as `[]`; the plugin's own, in a game without it, as
    // `{}`: both read as empty)
    let text = std::fs::read_to_string(dir.join("data").join("traffic.json")).unwrap();
    assert!(text.contains("\"vehicles\":[]"), "{text}");
    let old: bridge::Traffic = serde_json::from_str(r#"{"at":1,"map":"m","vehicles":{},"buses":{},"players":{}}"#).unwrap();
    assert!(old.vehicles.is_empty() && old.buses.is_empty() && old.players.is_empty());
}

/// The company's buses Omsi-Hub wrote for a company's duty (`data/fleet.json`).
fn fleet(dir: &std::path::Path, map: &str) {
    let trip = |tour: &str, departure: Option<f64>, delay: f64| bridge::FleetTrip { line: "5".into(), tour: tour.into(), departure, delay };
    let f = bridge::Fleet { at: 1.0, company: "c1".into(), map: map.into(), date: "2026-10-26".into(), trips: vec![trip("1", Some(28_800.0), 120.0), trip("2", None, 0.0)] };
    bridge::write_fleet(dir, &f).unwrap();
}

#[test]
fn a_company_s_duty_has_only_the_company_s_buses_on_the_road() {
    let dir = installed("fleet");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    // (the company names the map with backslashes, the game with slashes: the same map)
    fleet(&dir, r"maps\Grundorf\global.cfg");
    let mut h = Harness::load(&dir, game()).unwrap();
    // (the file read in one call, its trips in the next, given to the game in the one after)
    for _ in 0..2 {
        h.frame().unwrap();
    }
    assert!(h.state.borrow().ai_trips.is_empty());
    h.frame().unwrap();
    // the trips go to the game as they are, each with its delay
    let given = h.state.borrow().ai_trips.clone();
    assert_eq!(given, [json!([{ "line": "5", "tour": "1", "departure": 28_800.0, "delay": 120.0 }, { "line": "5", "tour": "2", "delay": 0.0 }])]);
    // (looked at again: the same, not given again)
    for _ in 0..6 {
        h.frame().unwrap();
    }
    assert_eq!(h.state.borrow().ai_trips.len(), 1);
    // the file gone (Omsi-Hub took it away): the whole timetable again
    std::fs::remove_file(dir.join("data").join("fleet.json")).unwrap();
    for _ in 0..6 {
        h.frame().unwrap();
    }
    assert_eq!(h.state.borrow().ai_trips.last(), Some(&serde_json::Value::Null));

    // a drive of the player's own, or another map: the game's timetable as ever
    let own = installed("fleet-own");
    fleet(&own, "maps/Grundorf/global.cfg");
    let mut h = Harness::load(&own, game()).unwrap();
    h.frame().unwrap();
    assert!(h.state.borrow().ai_trips.is_empty());
    let other = installed("fleet-other");
    bridge::write_assignment(&other, &assignment()).unwrap();
    fleet(&other, "maps/Spandau/global.cfg");
    let mut h = Harness::load(&other, game()).unwrap();
    h.frame().unwrap();
    assert!(h.state.borrow().ai_trips.is_empty());
}

#[test]
fn an_openomsi_without_set_ai_trips_takes_the_other_buses_off_the_road() {
    let dir = installed("fleet-old");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    fleet(&dir, "maps/Grundorf/global.cfg");
    let mut g = game();
    let bus = |id: i64, tour: &str, departure: f64| json!({ "id": id, "line": "5", "tour": tour, "departure": departure, "x": 0.0, "y": 0.0 });
    // tour 1's 8:00 trip (and its 9:00 one, not the company's), tour 2, tour 3, another line
    g.timetable_buses = vec![bus(1, "1", 28_830.0), bus(2, "1", 32_400.0), bus(3, " 2 ", 40_000.0), bus(4, "3", 28_800.0), json!({ "id": 5, "line": "7", "tour": "1", "departure": 28_800.0 })];
    let mut h = Harness::load_with(&dir, g, "omsi.timetable.set_ai_trips = nil").unwrap();
    // (the file read, its trips, kept, the timetable's buses read, those to go found, taken off:
    // a call each)
    for _ in 0..7 {
        h.frame().unwrap();
    }
    let mut removed = h.state.borrow().removed.clone();
    removed.sort();
    assert_eq!(removed, [2, 4, 5]);
    assert!(h.state.borrow().ai_trips.is_empty());
}

#[test]
fn an_openomsi_without_the_traffic_functions_writes_none() {
    let dir = installed("no-traffic");
    let mut h = Harness::load_with(&dir, game(), "omsi.traffic = nil; omsi.timetable = nil; omsi.position = nil").unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    assert!(bridge::read_traffic(&dir).is_none());
}

#[test]
fn every_drive_writes_the_live_dashboard() {
    // a drive of the player's own, no assignment: the bus alone
    let dir = installed("dash-free");
    let mut h = Harness::load(&dir, GameState { speed: 37.5, position: Some((1200.0, -340.0, 0.0)), passengers: 14, ..game() }).unwrap();
    // (the view made in one call, written in the next)
    for _ in 0..4 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    assert_eq!((d.issued, d.bus.speed, d.bus.x, d.bus.y, d.bus.passengers), (0, 37.5, Some(1200.0), Some(-340.0), Some(14.0)));
    assert_eq!(d.bus.file, "Vehicles/MAN_SD202/SD202.bus");
    assert_eq!(d.clock, Some(28_800.0));
    assert!(d.duty.is_none() && d.stops.is_empty(), "{d:?}");
    assert!(d.drive.seconds >= 2.0 && d.drive.metres > 0.0, "{:?}", d.drive);
    // (where the bus is, alone, for the navigator: every frame here, a frame being a second)
    let p = bridge::read_pos(&dir).expect("data/pos.json");
    assert_eq!((p.n, p.t, p.x, p.y, p.heading, p.speed), (4, 4.0, 1200.0, -340.0, Some(0.0), 37.5));

    // a company's duty: the duty, its trip and its stops
    let dir = installed("dash-duty");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    let stop = |id: i64, name: &str, at: f64| hub_plugin_harness::Stop { id, name: name.into(), arr: at, dep: at };
    let mut g = game();
    g.trips[0].stops = vec![stop(1, "Zoo", 28_800.0), stop(2, "Markt", 29_100.0), stop(3, "Hbf", 29_400.0)];
    g.trips[0].terminus = "Hbf".into();
    let mut h = Harness::load(&dir, g).unwrap();
    for _ in 0..4 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    assert_eq!(d.issued, issued());
    let duty = d.duty.expect("the duty under way");
    assert_eq!((duty.line.as_str(), duty.tour.as_str(), duty.trip, duty.trips, duty.delay), ("10", "2", 1, 2, 42.0));
    assert_eq!((duty.trip_name.as_str(), duty.number.as_str(), duty.terminus.as_str()), ("Zoo - Hbf", "10", "Hbf"));
    let names: Vec<&str> = d.stops.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Zoo", "Markt", "Hbf"]);
    assert_eq!((d.stops[1].arrival, d.stops[1].stops), (29_100.0, true));
}

/// What the bus's IBIS is set to (its scripts' variables) and the way it faces go into the live
/// dashboard; a game that answers a variable it does not have with an error still gets the file
/// written, and the rest of the plugin's second goes on.
#[test]
fn the_live_dashboard_has_the_bus_s_ibis_and_heading_and_survives_a_variable_s_error() {
    let dir = installed("dash-ibis");
    let vars = "local v = { IBIS_LinieKurs = 2301, IBIS_RouteIndex = 0, IBIS_TerminusCode = 105 }
        omsi.var = function(n) if v[n] == nil then error('no variable ' .. n) end return v[n] end
        omsi.str = function(n) error('no string variable ' .. n) end";
    let mut h = Harness::load_with(&dir, GameState { position: Some((1200.0, -340.0, 90.0)), ..game() }, vars).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    let i = d.ibis.expect("the IBIS");
    assert_eq!((i.line, i.route_index, i.terminus, i.terminus_name), (Some(2301.0), Some(0.0), Some(105.0), None));
    assert_eq!(d.bus.heading, Some(90.0));

    // (no variable at all: no IBIS, the file still written every time)
    let dir = installed("dash-ibis-none");
    let mut h = Harness::load_with(&dir, game(), "omsi.var = function(n) error('no variable ' .. n) end").unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    let d = bridge::read_dash(&dir).expect("data/dash.json");
    assert!(d.ibis.is_none(), "{d:?}");
}

/// A Lua expression of the plugin's state, as JSON.
fn eval(h: &Harness, lua: &str) -> serde_json::Value {
    let v: mlua::Value = h.lua.load(lua).eval().unwrap();
    serde_json::to_value(&v).unwrap()
}

/// A company's duty as the on-board computer has all of it - the duty strip (the handover notes,
/// the walk-around), the control room's messages, on time, the drive watch, the trip's card and
/// the notes - and nothing of it drawn in the game, whatever openOMSI's look.
#[test]
fn nothing_is_drawn_while_driving() {
    let row = |kind: &str, at: &str, from: i32, to: i32| bridge::StripRow { kind: kind.into(), at: at.into(), from, to, ..bridge::StripRow::default() };
    let crew = bridge::Crew {
        rows: vec![row("sign_on", "Depot", 470, 480), row("break", "Hbf", 485, 500), bridge::StripRow { relief: true, ..row("sign_off", "Hbf", 485, 485) }],
        bus: 12,
        bus_number: "4711".into(),
        notes: vec!["Door 2 sticks".into()],
        checks: vec![("lights".into(), "Lights".into())],
        text_max: 140,
        low_fuel: 0.15,
        ..bridge::Crew::default()
    };
    let instruction = InboxMsg {
        id: 7,
        from: "Control".into(),
        priority: "urgent".into(),
        text: "Hold at the next stop".into(),
        state: "sent".into(),
        action: Some(InboxAction { kind: "hold".into(), minutes: 1, ..InboxAction::default() }),
        ..InboxMsg::default()
    };
    // (an openOMSI with the navigator's board, and one without)
    for (k, look) in ["omsi.ui.version = 5; omsi.ui.dock = function() return 320, 900, 400, true end", ""].into_iter().enumerate() {
        let dir = installed(&format!("nothing-drawn-{k}"));
        bridge::write_assignment(&dir, &Assignment { crew: Some(crew.clone()), ..assignment() }).unwrap();
        bridge::write_inbox(&dir, &Inbox { line: "10".into(), tour: "2".into(), messages: vec![instruction.clone()], ..Inbox::default() }).unwrap();
        computer_on(&dir);
        let mut h = Harness::load_with(&dir, game(), look).unwrap();
        for _ in 0..6 {
            h.frame().unwrap();
        }
        // the instruction accepted on the on-board computer: the hold counted down at the next stop
        let accept = json!({ "sid": 1, "port": PORT, "seq": 1, "cmds": [{ "seq": 1, "do": "msg", "kind": "accept", "id": 7 }] });
        std::fs::write(dir.join("data/companion.json"), accept.to_string()).unwrap();
        for _ in 0..3 {
            h.frame().unwrap();
        }
        h.fire("stop_arrive", &[json!(null), json!(null), json!(1), json!(0.0)]).unwrap();
        // the hold on the on-board computer's board while it runs, its note until it is over (the
        // game's clock going on with the frames)
        for _ in 0..30 {
            h.state.borrow_mut().clock += 1.0;
            h.frame().unwrap();
        }
        let board = sent(&h, "board").pop().unwrap();
        let hold = &board["hold"];
        assert!(hold["until"].is_number() && hold["left"].as_i64().is_some_and(|l| l > 0 && l <= 60), "{board}");
        assert!(notes(&h).last().unwrap().iter().any(|n| n.contains("Hold here for 1 min")), "{:?}", notes(&h).last());
        for _ in 0..40 {
            h.state.borrow_mut().clock += 1.0;
            h.frame().unwrap();
        }
        let board = sent(&h, "board").pop().unwrap();
        assert!(board.get("hold").is_none(), "the hold over: {board}");
        let last = notes(&h).pop().unwrap();
        assert!(last.iter().any(|n| n.contains("Hold over")) && !last.iter().any(|n| n.contains("Hold here")), "{last:?}");
        h.fire("red_light", &[json!(30.0)]).unwrap();
        h.fire("trip_done", &[json!(1), json!("arrived"), json!(95.0), json!(88.0), json!(100.0)]).unwrap();
        // (the keys that once opened the inbox, gave the panels the mouse and closed the card)
        for key in ["F9", "F10", "Enter"] {
            h.fire("key", &[json!(key), json!(true)]).unwrap();
        }
        for _ in 0..40 {
            h.frame().unwrap();
        }
        // all of it on the on-board computer
        for part in ["strip", "ontime", "board", "card", "notes"] {
            assert!(!sent(&h, part).is_empty(), "{part} sent to the on-board computer");
        }
        let all_notes = sent(&h, "notes").iter().map(Value::to_string).collect::<String>();
        for said in ["Hold at the next stop", "Door 2 sticks", "Hold here for 1 min"] {
            assert!(all_notes.contains(said), "{said} in {all_notes}");
        }
        h.fire("stop", &[]).unwrap();
        // nothing in the game: no panel, no toast, no line of text
        let st = h.state.borrow();
        assert!(st.panels.is_empty(), "panels drawn: {:?}", st.panels.keys().collect::<Vec<_>>());
        assert!(st.toasts.is_empty(), "toasts: {:?}", st.toasts.iter().map(|t| t.text.clone()).collect::<Vec<_>>());
        assert!(st.messages.is_empty(), "lines of text: {:?}", st.messages);
    }
}

/// The plugin's folder is an interface theme (Omsi-Hub's look for openOMSI's own screens), put
/// in force once by an openOMSI that knows themes, and only over openOMSI's own look: a theme
/// the player chose, and a second start, keep theirs; an older openOMSI is not asked.
#[test]
fn the_omsi_hub_look_is_put_in_force_once_in_an_openomsi_with_themes() {
    let dir = installed("theme");
    for file in ["theme.json", "theme-loading.svg", "theme-logo.svg"] {
        assert!(dir.join(file).is_file(), "{file} installed");
    }
    let theme: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("theme.json")).unwrap()).unwrap();
    assert_eq!((theme["loading"]["logo"].as_str(), theme["loading"]["progress"].as_str(), theme["logo"].as_str()), (Some("theme-loading.svg"), Some("stroke"), Some("theme-logo.svg")));
    assert!(theme["glass"].is_object());
    // (the loading logo is the brand's, its ring and line the path the progress is drawn along;
    // the bus pops up as the ring is nearly round it and flashes its headlights, the stops pop
    // up as the line reaches them - in that order along the line)
    let loading = std::fs::read_to_string(dir.join("theme-loading.svg")).unwrap();
    let brand = include_str!("../../../assets/brand/logo-light.svg");
    let ring = brand.lines().find(|l| l.contains("stroke=\"#2a75f7\"")).unwrap();
    assert!(loading.contains(&ring.replace("<path d=", "<path id=\"progress\" d=")));
    assert_eq!(loading.matches("id=\"progress\"").count(), 1);
    let at = |id: &str| -> f32 {
        let tag = &loading[loading.find(&format!("id=\"{id}\"")).unwrap_or_else(|| panic!("{id}"))..];
        tag[tag.find("data-at=\"").unwrap() + 9..].split('"').next().unwrap().parse().unwrap()
    };
    let order = [at("bus"), at("stop-1"), at("stop-2"), at("stop-3")];
    assert!(order.windows(2).all(|w| w[0] < w[1]) && order[3] <= 1.0, "{order:?}");
    assert_eq!(at("lights"), at("bus"));
    assert!(loading.contains("id=\"lights\" data-at=\"0.33\" data-delay=\"0.4\" data-blink=\"\""));
    let themes = |now: &str, offered: bool| {
        format!(
            "omsi.ui.version = 6; ASKED = {{}}; omsi.ui.theme = function() return \"{now}\" end; omsi.ui.set_theme = function(id) table.insert(ASKED, id); return true end; omsi.data.theme_offered = {}",
            if offered { "true" } else { "nil" }
        )
    };
    let asked = |h: &Harness| eval(h, "ASKED[1]");
    // the first start with openOMSI's own look: Omsi-Hub's, by the plugin's folder name
    let h = Harness::load_with(&dir, game(), &themes("", false)).unwrap();
    assert_eq!((asked(&h), eval(&h, "#ASKED")), (json!(install::NAME), json!(1)));
    assert_eq!(eval(&h, "omsi.data.theme_offered"), json!(true));
    // a theme the player chose, or a start after the first: theirs stays
    let h = Harness::load_with(&dir, game(), &themes("night", false)).unwrap();
    assert_eq!(asked(&h), json!(null));
    let h = Harness::load_with(&dir, game(), &themes("", true)).unwrap();
    assert_eq!(asked(&h), json!(null));
    // an openOMSI without themes: nothing asked, nothing remembered
    let h = Harness::load(&dir, game()).unwrap();
    assert_eq!(eval(&h, "omsi.data.theme_offered"), json!(null));
}

/// A bus taken over halfway along its trip (`takeover.lua`): the AI's bus of the tour off the
/// road and the player's bus put exactly where it was, its next stop the duty's next; with no
/// such bus on the road, where Omsi-Hub's fleet map has it.
#[test]
fn a_bus_is_taken_over_where_the_ai_drives_it() {
    let take = |at: Option<[f64; 3]>| Assignment { take_over: Some(bridge::TakeOver { line: "10".into(), tour: "2".into(), trip: 1, stop: 2, at }), ..assignment() };
    let stop = |id: i64| hub_plugin_harness::Stop { id, ..Default::default() };
    let game = || {
        let mut g = game();
        g.trips[0].stops = vec![stop(31), stop(32), stop(33)];
        g
    };

    // the AI's bus of tour 2 on the road: taken off it, the player's bus where it was
    let dir = installed("takeover-bus");
    bridge::write_assignment(&dir, &take(Some([500.0, 600.0, 90.0]))).unwrap();
    let mut g = game();
    g.timetable_buses = vec![
        json!({ "id": 4, "line": "10", "tour": "3", "trip": 1, "x": 1.0, "y": 1.0 }),
        json!({ "id": 7, "line": "10", "tour": "2", "trip": 1, "x": 100.0, "y": 200.0, "heading": 45.0, "next_stop_id": 33 }),
    ];
    let mut h = Harness::load(&dir, g).unwrap();
    for _ in 0..5 {
        h.frame().unwrap();
    }
    let st = h.state.borrow();
    assert_eq!(st.removed, [7], "only the bus of the tour taken over");
    assert_eq!(st.teleported, [(100.0, 200.0, 45.0)]);
    assert_eq!(st.skipped_to.last(), Some(&3), "its next stop the duty's next");
    drop(st);

    // none on the road: where the fleet map has it, its next stop the one Omsi-Hub said
    let dir = installed("takeover-place");
    bridge::write_assignment(&dir, &take(Some([500.0, 600.0, 90.0]))).unwrap();
    let mut h = Harness::load(&dir, game()).unwrap();
    for _ in 0..4 {
        h.frame().unwrap();
    }
    assert!(h.state.borrow().teleported.is_empty(), "the AI's bus is looked for a while first");
    for _ in 0..10 {
        h.frame().unwrap();
    }
    let st = h.state.borrow();
    assert_eq!(st.teleported, [(500.0, 600.0, 90.0)]);
    assert_eq!(st.skipped_to.last(), Some(&2));
    assert!(st.removed.is_empty());
}
