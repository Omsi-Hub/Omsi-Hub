//! The phone at its fullest: a long trip, a long duty, a full inbox and every line of the
//! brief - each tab and mode stays within what openOMSI draws of a panel (200 elements, 500
//! characters a text).

mod common;

use common::*;
use hub_game::bridge::{Assignment, Colleague, Crew, CrewCan, CrewDefect, Fare, RosterDuty, StripRow, TripIbis, WeekDuty};
use hub_plugin_harness::{GameState, Harness, Stop, Trip};
use serde_json::{json, Value};

/// openOMSI's limits for a panel (`omsi-plugin/src/ui.rs`).
const MOST_ELEMENTS: usize = 200;
const MOST_TEXT: usize = 500;

/// A tall screen (2000 points under the map, the panel always short of it: every list grows
/// to its most): ten trips of line 10, the first of forty stops.
fn big_game() -> GameState {
    let trip = |k: usize, stops: usize| {
        let dep = 28_800.0 + 3_600.0 * k as f64;
        Trip {
            name: format!("Trip {k}"),
            line: "10".into(),
            terminus: "Hauptbahnhof Südseite          10".into(),
            departure: dep,
            arrival: dep + 60.0 * stops as f64,
            stops: (0..stops).map(|j| Stop { id: j as i64 + 1, name: format!("Haltestelle Nummer {j} an der langen Straße"), arr: dep + 60.0 * j as f64, dep: dep + 60.0 * j as f64 }).collect(),
        }
    };
    GameState {
        dock: Some((360.0, 2000.0, 100.0, true)),
        trips: (0..10).map(|k| trip(k, if k == 0 { 40 } else { 12 })).collect(),
        passengers: 60,
        standing: 20,
        fuel: Some(20.0),
        sales: (120, 240.5),
        damage: Some((3, 40.0, 90.0)),
        closed: vec!["Haltestelle Nummer 3 an der langen Straße".into()],
        ..docked_game()
    }
}

/// A duty of ten trips with a break and two reliefs; a full week; a bus with many defects and
/// notes; the codes of every trip.
fn big_assignment() -> Assignment {
    let long = "x".repeat(140);
    let mut rows = vec![StripRow { kind: "sign_on".into(), at: "Depot".into(), from: 470, to: 470, ..StripRow::default() }];
    for k in 0..10 {
        let from = 480 + 60 * k;
        rows.push(StripRow { kind: "trip".into(), at: "Zoo".into(), to_place: "Hbf".into(), from, to: from + 40, line: "10".into(), relief: k == 4, ..StripRow::default() });
        if k == 5 {
            rows.push(StripRow { kind: "break".into(), at: "Zoo".into(), from: from + 40, to: from + 60, ..StripRow::default() });
        }
    }
    rows.push(StripRow { kind: "sign_off".into(), at: "Depot".into(), from: 1100, to: 1100, ..StripRow::default() });
    let week = (0..7)
        .map(|d| WeekDuty {
            day: "Mon".into(),
            date: format!("2026-10-{:02}", 20 + d),
            duty: (d + 1).to_string(),
            from: "05:40".into(),
            to: "13:55".into(),
            line: "307".into(),
            now: d == 0,
            swap: (d > 0).then(|| RosterDuty { date: format!("2026-10-{:02}", 20 + d), line: "Linie307".into(), tour: "4".into(), duty: d }),
            ..WeekDuty::default()
        })
        .collect();
    let crew = Crew {
        rows,
        changes: (0..6).map(|k| format!("Change {k}: {long}")).collect(),
        bus: 7,
        bus_number: "107".into(),
        notes: (0..8).map(|k| format!("Note {k}: {long}")).collect(),
        templates: vec![("mirror".into(), "Mirror loose".into())],
        checks: vec![("doors".into(), "Doors".into())],
        text_max: 140,
        low_fuel: 0.2,
        defects: (0..10).map(|k| CrewDefect { part: "doors".into(), label: "Doors".into(), text: format!("{k} {long}"), since: "2026-10-19".into(), held: false }).collect(),
        week,
        colleagues: (1..30).map(|id| Colleague { id, name: format!("Colleague {id}") }).collect(),
        can: CrewCan { sick: true, swap: true, leave: true },
        ..Crew::default()
    };
    let codes = (0..10)
        .map(|k| TripIbis { trip: k + 1, dep: 28_800.0 + 3_600.0 * k as f64, line: "010".into(), route: "01001".into(), dest: "123".into(), text: "Hauptbahnhof".into(), kurs: "2".into() })
        .collect();
    Assignment { crew: Some(crew), ibis_codes: codes, fare: Some(Fare { per_boarding: 1.2, symbol: "€".into(), after: false, places: 2 }), ..assignment() }
}

/// Every text of a panel - texts, badges, buttons, table cells, stops - with where it is.
fn all_texts(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| all_texts(x, out)),
        Value::Object(o) => o.values().for_each(|x| all_texts(x, out)),
        _ => {}
    }
}

/// The phone as openOMSI would take it: at most 200 elements, no text over 500 characters.
fn within_limits(h: &Harness, what: &str) {
    no_ui_errors(h);
    let p = phone(h);
    let n = elements(&p).len();
    assert!(n < MOST_ELEMENTS, "{what}: {n} elements");
    let mut texts = Vec::new();
    all_texts(&p, &mut texts);
    if let Some(t) = texts.iter().find(|t| t.chars().count() > MOST_TEXT) {
        panic!("{what}: a text of {} characters: {t}", t.chars().count());
    }
}

#[test]
fn every_tab_of_the_fullest_phone_stays_within_the_game_s_panel_limits() {
    let (dir, mut h) = start("budget", Some(big_assignment()), big_game(), 3);
    let now = h.state.borrow().now;
    std::fs::create_dir_all(dir.join("data")).unwrap();
    // every line of the brief
    let brief = json!({
        "issued": issued(), "trip": 1, "at": now,
        "headway": { "ahead": { "bus": "101", "secs": 360 }, "planned": 600, "advice": { "kind": "hold", "secs": 40 } },
        "connections": [
            { "stop": "Haltestelle Nummer 5 an der langen Straße", "what": "train to Erfurt", "time": "07:44", "late": 0, "onward": false, "max_hold": 2, "status": { "kind": "wait", "until": "07:46" } },
            { "stop": "Hbf", "what": "bus 300", "time": "08:44", "late": 0, "onward": true, "max_hold": 2, "status": { "kind": "wait", "until": "08:46" } },
        ],
        "ahead": { "stop": "Markt", "via": "Ringstraße" }, "closed": ["Markt", "Rathaus", "Kirche"],
    });
    std::fs::write(dir.join("data").join("ontime.json"), brief.to_string()).unwrap();
    // eight messages, two instructions waiting, one far too long
    let long = "Bitte ".repeat(90);
    let mut messages: Vec<Value> =
        (0..6).map(|k| json!({ "id": k + 1, "time": format!("08:0{k}"), "from": "Control", "text": format!("Message {k}: {}", "x".repeat(300)), "priority": "info", "state": "sent" })).collect();
    messages.push(json!({ "id": 7, "time": "08:07", "from": "Control", "text": long, "priority": "instruction", "state": "sent", "action": { "kind": "hold", "minutes": 2 } }));
    messages.push(json!({ "id": 8, "time": "08:08", "from": "Control", "text": "Turn back at Markt", "priority": "urgent", "state": "sent", "action": { "kind": "short_turn", "stop": "Markt" } }));
    let quick: Vec<Value> =
        ["traffic", "breakdown", "accident", "aggressive", "closed", "relief", "call"].iter().map(|k| json!({ "key": k, "label": format!("Quick {k}"), "icon": "warning" })).collect();
    let replies: Vec<Value> = ["ok", "late", "no", "call"].iter().map(|k| json!({ "key": k, "label": format!("Reply {k}") })).collect();
    std::fs::write(dir.join("data").join("messages.json"), json!({ "messages": messages, "replies": replies, "quick": quick }).to_string()).unwrap();
    run(&mut h, 6);
    // (each tab long enough for its lists to grow to their most)
    for tab in 1..=5 {
        choose_tab(&h, tab);
        run(&mut h, 70);
        within_limits(&h, &format!("tab {tab}"));
    }
    choose_tab(&h, 2);
    let rail = find(&h, PHONE, |e| e["type"] == "stops").expect("the rail");
    assert!(rail["stops"].as_array().unwrap().len() >= 39, "{rail}");
    // the Messages tab with its quick reports open, the Duty tab asking which day
    choose_tab(&h, 4);
    if has_element(&h, PHONE, |e| e["id"] == "msg_quick") {
        click(&h, "msg_quick");
    }
    within_limits(&h, "quick reports");
    choose_tab(&h, 3);
    click(&h, "duty_swap");
    within_limits(&h, "a swap's days");
    click(&h, "duty_no");
    // the Bus tab's note form
    choose_tab(&h, 5);
    click(&h, "strip_note");
    within_limits(&h, "the note form");
    // the trip done: the break clock
    {
        let mut g = h.state.borrow_mut();
        g.trip_done = true;
        g.clock = 31_000.0;
    }
    run(&mut h, 2);
    choose_tab(&h, 1);
    within_limits(&h, "the break");
}

#[test]
fn the_light_and_the_slim_phone_and_an_openomsi_without_the_board_stay_within_the_limits_too() {
    // the light card (tables where the dark one has the rail) and a board without rails
    for (tag, a, g) in [("budget-light", light(big_assignment()), big_game()), ("budget-v3", big_assignment(), GameState { ui_version: 3, ..big_game() })] {
        let (_, mut h) = start(tag, Some(a), g, 3);
        for tab in 1..=5 {
            choose_tab(&h, tab);
            run(&mut h, 70);
            within_limits(&h, &format!("{tag} tab {tab}"));
        }
    }
    let (_, h) = start("budget-slim", Some(big_assignment()), GameState { dock: Some((360.0, 120.0, 0.0, true)), ..big_game() }, 3);
    within_limits(&h, "slim");
    // 0.3.37: the modules' own panels, each within the limits
    let (_, h) = start("budget-old", Some(big_assignment()), GameState { ui_version: 2, dock: None, ..big_game() }, 3);
    no_ui_errors(&h);
    assert!(!has_panel(&h, PHONE));
    for (id, p) in h.state.borrow().panels.iter() {
        assert!(elements(p).len() < MOST_ELEMENTS, "{id}");
    }
}
