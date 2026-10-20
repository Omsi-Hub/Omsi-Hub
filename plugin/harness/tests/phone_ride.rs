//! The phone's Ride, Stops and Bus tabs, and the trip's and the duty's summaries
//! (`plugin/phone_ride.lua`, `phone_stops.lua`, `phone_bus.lua`, `phone_summary.lua`).

mod common;

use common::*;
use hub_game::bridge::{self, Assignment, Crew, Fare};
use hub_plugin_harness::{GameState, Harness, Stop};
use serde_json::{json, Value};

/// The big number of the stat labelled `label` (a tile or a row: the number, then its label).
fn stat(h: &Harness, label: &str) -> Value {
    find(h, PHONE, |e| (e["type"] == "box" || e["type"] == "row") && e["children"].as_array().is_some_and(|c| c.len() == 2 && c[1]["text"] == label))
        .map(|e| e["children"][0].clone())
        .unwrap_or_else(|| panic!("no stat {label}: {}", phone(h)))
}

/// A trip of twelve stops from Zoo by Markt to Hbf in place of the first one.
fn long_trip(mut g: GameState) -> GameState {
    let names = ["Zoo", "Markt", "Rathaus", "Kirche", "Schule", "Park", "Post", "Bad", "Feld", "Mühle", "Brücke", "Hbf"];
    g.trips[0].stops = names.iter().enumerate().map(|(k, n)| Stop { id: k as i64 + 1, name: (*n).into(), arr: 28_800.0 + 60.0 * k as f64, dep: 28_800.0 + 60.0 * k as f64 }).collect();
    g
}

/// The plugin's reports to Omsi-Hub, as JSON.
fn reports(dir: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("data").join("reports.jsonl")).unwrap_or_default().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

#[test]
fn the_ride_tab_shows_the_stop_ahead_large_the_three_stats_and_the_trip_s_progress() {
    let mut g = docked_game();
    g.passed = 1;
    g.passengers = 12;
    g.speed = 32.0;
    let (_, h) = start("ride", Some(assignment()), g, 3);
    assert_eq!(tabs(&h).unwrap().1, 1);
    // the stop ahead large, when the bus is there and in how long, the one after it
    assert!(has_element(&h, PHONE, |e| e["text"] == "Markt" && e["size"] == 18), "{}", phone(&h));
    assert!(shows(&h, PHONE, "arr 08:05") && shows(&h, PHONE, "in 6 min"));
    assert!(shows(&h, PHONE, "then Hbf  ·  stop 2/3"));
    // passengers, km/h and the delay (42 s: on time, as on the navigator's map)
    assert_eq!(stat(&h, "PASSENGERS")["text"], "12");
    assert_eq!(stat(&h, "KM/H")["text"], "32");
    assert_eq!(stat(&h, "DELAY")["text"], "on time");
    assert!(has_element(&h, PHONE, |e| e["type"] == "box"), "stat tiles where the game has boxes");
    // the trip's progress: one stop behind, two ahead
    let bar = find(&h, PHONE, |e| e["type"] == "bar" && e.get("segments").is_some()).expect("the progress");
    assert_eq!((bar["segments"][0]["value"].as_i64(), bar["segments"][1]["value"].as_i64()), (Some(1), Some(2)));
    no_ui_errors(&h);
}

#[test]
fn the_delay_is_in_the_time_s_colour_as_on_the_navigator() {
    let (_, mut h) = start("ride-delay", Some(assignment()), docked_game(), 2);
    for (delay, text, color) in [(-90.0, "−1:30", "#2F7CF6"), (30.0, "on time", "#2FA84F"), (59.0, "on time", "#2FA84F"), (200.0, "+3:20", "#FF453A")] {
        h.state.borrow_mut().duty.as_mut().unwrap().delay = delay;
        run(&mut h, 1);
        let s = stat(&h, "DELAY");
        assert_eq!((s["text"].as_str(), s["color"].as_str()), (Some(text), Some(color)), "{delay}");
    }
    // late: when the bus gets to the stop ahead, in the late colour, here and on the Stops tab
    assert!(has_element(&h, PHONE, |e| e["text"] == "arr 08:03" && e["color"] == "#FF453A"), "{}", phone(&h));
    choose_tab(&h, 2);
    let rail = find(&h, PHONE, |e| e["type"] == "stops").expect("the rail");
    assert_eq!((rail["stops"][0]["expected"].as_str(), rail["stops"][0]["expected_color"].as_str()), (Some("08:03"), Some("#FF453A")));
    assert_eq!(rail["stops"][0]["state"], "now");
    no_ui_errors(&h);
}

#[test]
fn omsi_hub_s_brief_gives_the_ride_tab_the_lines_that_apply() {
    let (dir, mut h) = start("ride-brief", Some(assignment()), docked_game(), 1);
    assert!(!shows(&h, PHONE, "ahead"), "no lines without a brief");
    let now = h.state.borrow().now;
    let brief = json!({
        "issued": issued(), "trip": 1, "at": now,
        "headway": { "ahead": { "bus": "101", "secs": 360 }, "planned": 600, "advice": { "kind": "hold", "secs": 40 } },
        "connections": [{ "stop": "Hbf", "what": "train to Erfurt", "time": "07:44", "late": 0, "onward": false, "max_hold": 2, "status": { "kind": "wait", "until": "07:46" } }],
        "ahead": { "stop": "Markt", "via": "Ringstraße" }, "closed": ["Markt"],
    });
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data").join("ontime.json"), brief.to_string()).unwrap();
    run(&mut h, 5);
    assert!(shows(&h, PHONE, "101 ahead 6 min  ·  hold 40 s"), "{}", phone(&h));
    assert!(shows(&h, PHONE, "At Hbf: train to Erfurt 07:44  ·  wait max 2 min"));
    assert!(shows(&h, PHONE, "Diversion: Markt closed, via Ringstraße"));
    assert!(!has_panel(&h, "omsihub_ontime"), "no on-time card of its own");
    no_ui_errors(&h);
}

#[test]
fn the_control_room_s_instruction_points_from_the_ride_tab_to_messages() {
    let (dir, mut h) = start("ride-instruction", Some(assignment()), docked_game(), 1);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let inbox = json!({ "messages": [
        { "id": 7, "time": "08:05", "from": "Control", "text": "Turn back at Markt", "priority": "instruction", "state": "sent", "action": { "kind": "short_turn", "stop": "Markt" } },
    ], "replies": [], "quick": [] });
    std::fs::write(dir.join("data").join("messages.json"), inbox.to_string()).unwrap();
    run(&mut h, 3);
    // (the inbox came up by itself; the driver goes back to the Ride tab)
    choose_tab(&h, 1);
    run(&mut h, 1);
    assert!(shows(&h, PHONE, "Turn back at Markt"), "{}", phone(&h));
    click(&h, "ride_inbox");
    assert_eq!(tabs(&h).unwrap().1, 4);
    no_ui_errors(&h);
}

#[test]
fn the_stops_tab_is_a_rail_in_the_navigator_s_look_and_a_table_elsewhere() {
    let mut g = long_trip(docked_game());
    g.closed = vec!["Markt".into()];
    // the rail: the stop ahead and seven more (the room's), Markt closed, the rest counted
    let (_, h) = start("stops-rail", Some(assignment()), g.clone(), 2);
    choose_tab(&h, 2);
    let rail = find(&h, PHONE, |e| e["type"] == "stops").expect("the rail");
    let list = rail["stops"].as_array().unwrap();
    assert_eq!(list.len(), 8);
    assert_eq!((list[0]["name"].as_str(), list[0]["state"].as_str()), (Some("Zoo"), Some("now")));
    assert_eq!((list[1]["name"].as_str(), list[1]["tag"].as_str(), list[1]["state"].as_str()), (Some("Markt"), Some("closed"), Some("ahead")));
    assert!(shows(&h, PHONE, "and 4 more to Hbf"), "{}", phone(&h));
    no_ui_errors(&h);
    // an openOMSI with the navigator's board but no rails (ui version 3): a table of plain texts
    let (_, h) = start("stops-v3", Some(assignment()), GameState { ui_version: 3, ..g.clone() }, 2);
    choose_tab(&h, 2);
    assert!(!has_element(&h, PHONE, |e| e["type"] == "stops"));
    let table = find(&h, PHONE, |e| e["type"] == "table").expect("the table");
    assert_eq!(table["rows"][0], json!(["▶", "Zoo", "08:00", ""]));
    assert_eq!(table["rows"][1][1], "Markt  · closed");
    no_ui_errors(&h);
    // the navigator's light card (the rail's names are in light ink): a table in dark ink
    let (_, h) = start("stops-light", Some(light(assignment())), g, 2);
    choose_tab(&h, 2);
    assert!(!has_element(&h, PHONE, |e| e["type"] == "stops"));
    let table = find(&h, PHONE, |e| e["type"] == "table").expect("the table");
    assert_eq!((table["rows"][0][1]["text"].as_str(), table["rows"][0][1]["color"].as_str()), (Some("Zoo"), Some("#1C1C1E")));
    no_ui_errors(&h);
    // an openOMSI before the navigator's board: no phone, the board of before
    let (_, h) = start("stops-v2", Some(assignment()), old_game(), 2);
    assert!(!has_panel(&h, PHONE));
    assert!(has_element(&h, "omsihub_board", |e| e["type"] == "table"));
    no_ui_errors(&h);
}

#[test]
fn the_bus_tab_shows_the_fuel_the_takings_the_damage_the_defects_and_the_notes() {
    let crew = Crew {
        bus: 7,
        bus_number: "1234".into(),
        notes: vec!["Left mirror loose".into()],
        templates: vec![("mirror".into(), "Mirror loose".into())],
        checks: vec![("doors".into(), "Doors".into())],
        text_max: 140,
        low_fuel: 0.2,
        ..Crew::default()
    };
    let a = Assignment { crew: Some(crew), fare: Some(Fare { per_boarding: 1.2, symbol: "€".into(), after: false, places: 2 }), ..assignment() };
    let mut g = docked_game();
    g.fuel = Some(200.0);
    g.sales = (23, 41.5);
    g.damage = Some((2, 30.0, 45.0));
    g.passengers = 9;
    g.standing = 2;
    // (the bus's open defects: the app's `crew.defects`, which this Omsi-Hub does not write yet)
    let dir = installed("bus");
    bridge::write_assignment(&dir, &a).unwrap();
    let file = dir.join("assignment.lua");
    let lua = std::fs::read_to_string(&file).unwrap().replacen("return ", "local a = ", 1);
    std::fs::write(&file, lua + "a.crew.defects = { { part = \"doors\", text = \"Door 2 sticks\", since = \"2026-10-19\" } }\nreturn a\n").unwrap();
    let mut h = Harness::load(&dir, g).unwrap();
    run(&mut h, 2);
    choose_tab(&h, 5);
    assert!(shows(&h, PHONE, "100 %"), "{}", phone(&h));
    assert!(!shows(&h, PHONE, "Return to the depot"));
    assert_eq!(stat(&h, "SEATED")["text"], "7");
    assert_eq!(stat(&h, "STANDING")["text"], "2");
    assert_eq!(stat(&h, "TICKETS SOLD")["text"], "23");
    assert_eq!(stat(&h, "FARES TAKEN")["text"], "€41.50");
    assert!(shows(&h, PHONE, "Collisions 2") && shows(&h, PHONE, "repair 45 min"));
    assert!(shows(&h, PHONE, "Doors: Door 2 sticks") && shows(&h, PHONE, "2026-10-19"));
    assert!(shows(&h, PHONE, "Left mirror loose"));
    // the fuel runs low: back to the depot (under the company's 20 %)
    h.state.borrow_mut().fuel = Some(36.0);
    run(&mut h, 2);
    assert!(shows(&h, PHONE, "18 %") && shows(&h, PHONE, "Return to the depot to refuel"), "{}", phone(&h));
    // a note for the next driver: the strip's form on the Bus tab, sent as the strip sends it
    click(&h, "strip_note");
    assert!(has_element(&h, PHONE, |e| e["id"] == "strip_note_send"), "{}", phone(&h));
    assert!(shows(&h, PHONE, "Typing needs F10"));
    h.fire("ui_change", &[json!(PHONE), json!("strip_tpl_mirror"), json!(true)]).unwrap();
    click(&h, "strip_note_send");
    assert!(reports(&dir).iter().any(|r| r["kind"] == "note" && r["template"] == "mirror" && r["bus_id"] == 7), "{:?}", reports(&dir));
    assert!(has_element(&h, PHONE, |e| e["id"] == "strip_note"), "the form closed again");
    no_ui_errors(&h);
}

#[test]
fn after_the_last_trip_the_duty_s_summary_and_its_totals_in_the_duty_end_report() {
    let mut g = docked_game();
    g.speed = 360.0;
    let (dir, mut h) = start("summary", Some(assignment()), g, 2);
    // each trip: its stops served on time, passengers boarded, tickets sold
    let serve = |h: &mut Harness, boarded: i64| {
        for n in [1, 2] {
            h.fire("stop_arrive", &[json!(0), json!(""), json!(n), json!(30.0)]).unwrap();
            h.fire("stop_depart", &[json!(0), json!(""), json!(n), json!(30.0)]).unwrap();
            h.fire("passenger_board", &[json!(boarded)]).unwrap();
            run(h, 2);
        }
    };
    serve(&mut h, 2);
    {
        let mut s = h.state.borrow_mut();
        s.sales = (10, 20.0);
        s.trip_done = true;
        s.clock = 29_500.0;
    }
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    run(&mut h, 1);
    assert!(shows(&h, PHONE, "Trip done") && shows(&h, PHONE, "Fares 20.00  ·  10 tickets"), "{}", phone(&h));
    assert!(has_element(&h, PHONE, |e| e["type"] == "badge" && e["text"] == "Excellent"), "the trip's grade");
    // the second and last trip
    {
        let mut s = h.state.borrow_mut();
        s.duty.as_mut().unwrap().trip = 2;
        s.trip_done = false;
        s.clock = 30_000.0;
    }
    run(&mut h, 1);
    serve(&mut h, 2);
    {
        let mut s = h.state.borrow_mut();
        s.sales = (25, 50.0);
        s.trip_done = true;
        s.clock = 30_700.0;
    }
    h.fire("trip_done", &[json!(2), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    run(&mut h, 1);
    assert!(shows(&h, PHONE, "Trip done"));
    // the trip's card over: the duty's summary
    h.state.borrow_mut().clock = 30_730.0;
    run(&mut h, 1);
    assert!(shows(&h, PHONE, "Duty done"), "{}", phone(&h));
    assert_eq!(stat(&h, "TRIPS")["text"], "2");
    assert_eq!(stat(&h, "ON TIME")["text"], "100 %");
    assert_eq!(stat(&h, "PASSENGERS")["text"], "8");
    assert_eq!(stat(&h, "FARES")["text"], "50.00");
    let score = stat(&h, "SCORE")["text"].as_str().unwrap().parse::<i64>().unwrap();
    assert!(score >= 90, "{score}");
    assert_ne!(stat(&h, "KM")["text"], "0.0");
    // the duty ends: its totals go with the duty-end report, the summary stays until closed
    h.state.borrow_mut().duty = None;
    run(&mut h, 1);
    let end = reports(&dir).into_iter().find(|r| r["kind"] == "duty_end").expect("the duty-end report");
    assert_eq!((end["line"].as_str(), end["trips"].as_i64(), end["on_time"].as_i64(), end["passengers"].as_i64()), (Some("10"), Some(2), Some(100), Some(8)));
    assert_eq!(end["score"].as_i64(), Some(score));
    assert_eq!(end["fares"].as_f64(), Some(50.0));
    assert!(end["km"].as_f64().unwrap() > 0.0, "{end}");
    assert!(shows(&h, PHONE, "Duty done"));
    click(&h, "ride_summary_close");
    assert!(!shows(&h, PHONE, "Duty done"), "{}", phone(&h));
    no_ui_errors(&h);
}

#[test]
fn the_slim_phone_shows_the_stop_ahead_and_then_the_trip_s_card() {
    let mut g = docked_game();
    g.dock = Some((360.0, 120.0, 0.0, true));
    g.passed = 1;
    let (_, mut h) = start("ride-slim", Some(assignment()), g, 2);
    assert!(shows(&h, PHONE, "Markt") && shows(&h, PHONE, "08:05"), "{}", phone(&h));
    {
        let mut s = h.state.borrow_mut();
        s.trip_done = true;
        s.clock = 29_500.0;
    }
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    run(&mut h, 1);
    assert!(shows(&h, PHONE, "Trip done"), "{}", phone(&h));
    assert_eq!(phone(&h)["children"].as_array().unwrap().len(), 2);
    no_ui_errors(&h);
}
