//! The phone and tablet companion's side in the game (`plugin/companion.lua`), on the harness's
//! mock of openOMSI's plugin API: what it sends Omsi-Hub's companion server, and the phone's
//! commands it carries out - only the keys of the bus's IBIS and ticket printer, each command
//! once.

use hub_game::bridge::{self, Assignment, Inbox, InboxMsg, Personnel};
use hub_game::install;
use hub_plugin_harness::{GameState, Harness, Trip};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const PORT: i64 = 47812;

fn installed(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("hub-companion-plugin-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let plugins = d.join("Plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    install::install(&plugins).unwrap();
    install::plugin_dir(&plugins)
}

fn issued() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 3600 * 3600
}

/// A bus with the stock IBIS, a ticket printer and a door; a duty of two trips.
fn game() -> GameState {
    let trip = |n: &str, t: &str| Trip { name: n.into(), line: "29".into(), terminus: t.into(), departure: 28_800.0, arrival: 30_000.0, ..Trip::default() };
    let mut triggers: Vec<String> = (0..10).map(|d| format!("IBIS_{d}")).collect();
    triggers.extend(["IBIS_eingabe", "IBIS_eingabe_off", "ticketprinter_button_ticket_1", "door_1", "wechselgeld_1"].map(String::from));
    let mut g = GameState { trips: vec![trip("Zoo - Hbf", "Hbf"), trip("Hbf - Zoo", "Zoo")], triggers, position: Some((100.0, 200.0, 90.0)), ..GameState::default() };
    g.vars.insert("IBIS_LinieKurs".into(), 29.0);
    g.strs.insert("IBIS_terminus_name".into(), "Hauptbahnhof".into());
    g.tickets = vec![("Single".into(), 2.9), ("Day".into(), 7.0)];
    g.ticket_request = Some(("Single".into(), 2.9));
    g.sales = (3, 8.7);
    g.lines = json!([{ "name": "29", "user_allowed": true, "tours": [{ "number": 1, "today": true }, { "number": 2, "today": false }] }]);
    g
}

/// What the plugin sent the companion server, as JSON, of kind `t`.
fn sent(h: &Harness, t: &str) -> Vec<Value> {
    h.state.borrow().sent.iter().filter(|(p, _)| *p == PORT).filter_map(|(_, text)| serde_json::from_str::<Value>(text).ok()).filter(|v| v["t"] == t).collect()
}

/// Omsi-Hub's queue of commands in the plugin's data folder.
fn queue(dir: &Path, sid: u64, cmds: Value) {
    let seq = cmds.as_array().and_then(|c| c.last()).and_then(|c| c["seq"].as_u64()).unwrap_or(0);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data/companion.json"), json!({ "sid": sid, "port": PORT, "seq": seq, "cmds": cmds }).to_string()).unwrap();
}

fn acks(h: &Harness) -> Vec<(u64, bool, String)> {
    sent(h, "ack").iter().map(|a| (a["seq"].as_u64().unwrap(), a["ok"].as_bool().unwrap(), a["error"].as_str().unwrap_or("").to_string())).collect()
}

/// The keys held down, in order.
fn downs(h: &Harness) -> Vec<String> {
    h.state.borrow().held.iter().filter(|k| k.1).map(|k| k.0.clone()).collect()
}

/// The keys let go, in order.
fn ups(h: &Harness) -> Vec<String> {
    h.state.borrow().held.iter().filter(|k| !k.1).map(|k| k.0.clone()).collect()
}

#[test]
fn without_omsi_hubs_companion_nothing_is_sent() {
    let dir = installed("off");
    let mut h = Harness::load(&dir, game()).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    assert!(h.state.borrow().sent.iter().all(|(p, _)| *p != PORT));
}

#[test]
fn after_a_long_frame_the_companion_waits_until_the_game_runs_smoothly_again() {
    let dir = installed("calm");
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    assert!(!sent(&h, "live").is_empty(), "a game that runs smoothly: at once");
    // (the world loading: a frame two seconds after the one before)
    h.state.borrow_mut().now += 2.0;
    h.frame().unwrap();
    // (all but that the game is there)
    let news = |h: &Harness| h.state.borrow().sent.iter().filter(|(p, text)| *p == PORT && !text.contains("\"alive\"")).count();
    let before = news(&h);
    let smooth = |h: &mut Harness, frames: usize| {
        for _ in 0..frames {
            h.state.borrow_mut().now += 0.1;
            h.frame().unwrap();
        }
    };
    smooth(&mut h, 25);
    assert_eq!(news(&h), before, "nothing to the phone in the first seconds after it");
    assert!(!sent(&h, "alive").is_empty(), "but that the game is there");
    smooth(&mut h, 10);
    assert!(sent(&h, "live").len() > 1, "and again once the frames came smoothly for three seconds");
}

#[test]
fn a_game_that_keeps_hitching_is_heard_all_the_same() {
    let dir = installed("alive");
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    let t0 = h.state.borrow().now;
    let mut last = t0;
    // (a third of a second lost every two and a half seconds: never three seconds of smooth frames)
    for frame in 0..300 {
        {
            let mut s = h.state.borrow_mut();
            s.now += if frame % 25 == 0 { 0.3 } else { 0.1 };
            s.sent.clear();
        }
        h.frame().unwrap();
        let now = h.state.borrow().now;
        if !sent(&h, "alive").is_empty() || !sent(&h, "live").is_empty() {
            last = now;
        }
        assert!(now - last <= 2.0, "silent for {} s, the phone says the game stopped after 5", now - last);
    }
}

#[test]
fn the_game_goes_to_the_companion_and_only_the_devices_keys_are_pressed() {
    let dir = installed("keys");
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "29".into(), tour: "1".into(), trip: 1, trips: 2, delay: 60.0 });
    // (one job a tick: the game and the devices, the keys, the duty's trips, the map's lines -
    // read in one tick, sent in the next - with the game and the devices every other tick)
    for _ in 0..8 {
        h.frame().unwrap();
    }
    let live = sent(&h, "live");
    let last = live.last().expect("the live datagram");
    assert_eq!((last["duty"]["line"].as_str(), last["duty"]["number"].as_str(), last["duty"]["terminus"].as_str()), (Some("29"), Some("29"), Some("Hbf")));
    assert_eq!((last["bus"]["x"].as_f64(), last["ref"]["tile_x"].as_i64()), (Some(100.0), Some(0)));
    assert_eq!(last["signon"], "work");
    let keys = sent(&h, "keys");
    let k = keys.last().unwrap();
    assert_eq!(k["ibis"].as_array().unwrap().len(), 11, "the ten digits and the entry key, not its _off");
    assert_eq!(k["printer"], json!(["ticketprinter_button_ticket_1"]));
    assert_eq!(k["change"], json!(["wechselgeld_1"]));
    assert_eq!(k["tickets"][1]["name"], "Day");
    let dev = sent(&h, "dev");
    let d = dev.last().unwrap();
    assert_eq!((d["ibis"]["IBIS_LinieKurs"].as_f64(), d["strs"]["IBIS_terminus_name"].as_str()), (Some(29.0), Some("Hauptbahnhof")));
    assert_eq!((d["request"]["name"].as_str(), d["sales"]["count"].as_i64()), (Some("Single"), Some(3)));
    let lines = sent(&h, "lines");
    assert_eq!(lines.last().unwrap()["lines"][0]["tours"][1]["number"], "2");
    assert_eq!(sent(&h, "trips").last().unwrap()["trips"].as_array().unwrap().len(), 2);

    // the phone's commands: each once, a key not the devices' refused
    queue(
        &dir,
        5,
        json!([
            { "seq": 1, "do": "key", "key": "IBIS_7" },
            { "seq": 2, "do": "key", "key": "door_1" },
            { "seq": 3, "do": "ticket", "i": 1 },
            { "seq": 4, "do": "ticket", "i": 2 },
            { "seq": 5, "do": "ibis_line", "line": "29" },
            { "seq": 6, "do": "format_disk" },
        ]),
    );
    h.frame().unwrap();
    h.frame().unwrap();
    // (each a tap: held down and let go a moment later, as a hand presses it)
    assert_eq!(downs(&h), ["IBIS_7", "ticketprinter_button_ticket_1"]);
    assert_eq!(ups(&h), ["IBIS_7", "ticketprinter_button_ticket_1"]);
    assert!(h.state.borrow().pressed.is_empty(), "never pressed and let go in one frame");
    assert_eq!(h.state.borrow().bus_actions, ["ticket_give"], "a ticket without its own key: the ticket key");
    assert_eq!(h.state.borrow().lines_typed, ["29"]);
    let a = acks(&h);
    assert_eq!(a.len(), 6, "{a:?}");
    assert_eq!(a[1], (2, false, "unknown_key".into()));
    assert_eq!(a[5], (6, false, "unknown_command".into()));
    assert!(a[0].1 && a[2].1 && a[3].1 && a[4].1);
}

#[test]
fn commands_in_the_file_when_the_game_starts_are_old_and_a_new_session_starts_again() {
    let dir = installed("old");
    queue(&dir, 5, json!([{ "seq": 1, "do": "key", "key": "IBIS_7" }]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    assert!(h.state.borrow().held.is_empty(), "written before the game started: not done");
    // Omsi-Hub started again: its own numbers from 1
    queue(&dir, 6, json!([{ "seq": 1, "do": "key", "key": "IBIS_1" }]));
    h.frame().unwrap();
    h.frame().unwrap();
    assert_eq!(downs(&h), ["IBIS_1"]);
}

#[test]
#[ignore = "signing on is out for now (main.lua's SIGN_ON)"]
fn the_driver_signs_on_and_accepts_the_duty_on_the_phone() {
    let dir = installed("signon");
    let a = Assignment {
        issued: issued(),
        company: "Stadtbus".into(),
        duty: "14".into(),
        line: "29".into(),
        tour: "1".into(),
        auto_start: true,
        personnel: Some(Personnel { number: "482913".into(), code: "5821".into() }),
        ..Assignment::default()
    };
    bridge::write_assignment(&dir, &a).unwrap();
    queue(&dir, 9, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    assert_eq!(sent(&h, "live").last().unwrap()["signon"], "signon");
    queue(&dir, 9, json!([{ "seq": 1, "do": "sign_on", "number": "482913", "code": "0000" }]));
    h.frame().unwrap();
    h.frame().unwrap();
    assert_eq!(acks(&h), [(1, false, "wrong".to_string())]);
    assert_eq!(sent(&h, "live").last().unwrap()["signon_wrong"], true);
    assert!(h.state.borrow().started.is_empty(), "not signed on: no duty");
    queue(&dir, 9, json!([{ "seq": 2, "do": "sign_on", "number": "482913", "code": "5821" }, { "seq": 3, "do": "accept_order" }]));
    h.frame().unwrap();
    h.frame().unwrap();
    let a = acks(&h);
    assert!(a[1].1 && a[2].1, "{a:?}");
    assert_eq!(sent(&h, "live").last().unwrap()["signon"], "work");
    assert_eq!(h.state.borrow().started.len(), 1, "the duty taken as the order is signed");
    assert!(h.state.borrow().ibis.contains(&"typed".to_string()));
}

#[test]
fn a_message_is_answered_from_the_phone_as_from_the_panel() {
    let dir = installed("msgs");
    let a = Assignment { issued: issued(), company: "Stadtbus".into(), duty: "14".into(), line: "29".into(), tour: "1".into(), ..Assignment::default() };
    bridge::write_assignment(&dir, &a).unwrap();
    let inbox = Inbox {
        line: "29".into(),
        tour: "1".into(),
        messages: vec![InboxMsg { id: 77, text: "Diversion".into(), state: "delivered".into(), priority: "urgent".into(), ..InboxMsg::default() }],
        ..Inbox::default()
    };
    bridge::write_inbox(&dir, &inbox).unwrap();
    queue(&dir, 1, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    for _ in 0..3 {
        h.frame().unwrap();
    }
    queue(&dir, 1, json!([{ "seq": 1, "do": "msg", "kind": "read", "id": 77 }, { "seq": 2, "do": "msg", "kind": "accept", "id": 77 }, { "seq": 3, "do": "msg", "kind": "read", "id": 5 }]));
    h.frame().unwrap();
    h.frame().unwrap();
    let a = acks(&h);
    assert_eq!(a.iter().map(|x| x.1).collect::<Vec<_>>(), [true, false, false], "read; no instruction to accept; no such message: {a:?}");
    let replies = bridge::ReplyReader::default().read_new(&dir);
    assert_eq!(replies.iter().map(|r| (r.kind.as_str(), r.id)).collect::<Vec<_>>(), [("read", 77)]);
}

#[test]
fn the_devices_screens_are_read_ten_times_a_second_and_only_their_triggers_pressed() {
    let dir = installed("screens");
    let mut g = game();
    g.bus_file = "Vehicles\\TestBus\\TestBus.bus".into();
    g.triggers.extend(["IBIS_menu", "IBIS_menu_off"].map(String::from));
    g.vars.insert("IBIS_mode".into(), 1.0);
    g.vars.insert("IBIS_broken".into(), f64::NAN);
    g.strs.insert("IBIS_line_text".into(), "  29/03".into());
    let write = |cmds: Value| {
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let seq = cmds.as_array().and_then(|c| c.last()).and_then(|c| c["seq"].as_u64()).unwrap_or(0);
        let q = json!({
            "sid": 9, "port": PORT, "seq": seq, "cmds": cmds,
            "want": { "nums": ["IBIS_mode", "IBIS_stop", "IBIS_broken"], "strs": ["IBIS_line_text", "IBIS_dest_text"] },
            // (`door_1` is no device's: Omsi-Hub would not list it, and `IBIS_ghost` the bus lacks)
            "press": ["IBIS_menu", "IBIS_7", "IBIS_ghost"],
        });
        std::fs::write(dir.join("data/companion.json"), q.to_string()).unwrap();
    };
    write(json!([]));
    let mut h = Harness::load(&dir, g).unwrap();
    h.frame().unwrap();
    let live = sent(&h, "live");
    assert_eq!(live.last().unwrap()["bus_file"], "Vehicles\\TestBus\\TestBus.bus");
    let scr = sent(&h, "scr");
    assert!(!scr.is_empty(), "the screens' values");
    let items = &scr.last().unwrap()["items"];
    assert_eq!(items[0], json!(["n", "IBIS_mode", 1]));
    assert_eq!(items[1], json!(["o", "IBIS_stop"]), "a number the bus does not have");
    assert_eq!(items[2], json!(["n", "IBIS_broken"]), "no number: none");
    assert_eq!(items[3], json!(["s", "IBIS_line_text", "  29/03"]), "as the bus has it, not trimmed");
    assert_eq!(items[4], json!(["s", "IBIS_dest_text", ""]));
    assert_eq!((scr.last().unwrap()["page"].as_i64(), scr.last().unwrap()["pages"].as_i64()), (Some(1), Some(1)));
    // the same values: the same version; a change: the next
    let v = scr.last().unwrap()["v"].as_i64().unwrap();
    h.frame().unwrap();
    assert_eq!(sent(&h, "scr").last().unwrap()["v"].as_i64(), Some(v));
    h.state.borrow_mut().strs.insert("IBIS_line_text".into(), "  29/04".into());
    h.frame().unwrap();
    assert_eq!(sent(&h, "scr").last().unwrap()["v"].as_i64(), Some(v + 1));

    // the taps: pressed and let go, held, let go; nothing the devices do not have
    write(json!([
        { "seq": 1, "do": "press", "key": "IBIS_menu" },
        { "seq": 2, "do": "press", "key": "IBIS_7", "down": true },
        { "seq": 3, "do": "press", "key": "IBIS_7", "down": false },
        { "seq": 4, "do": "press", "key": "door_1" },
        { "seq": 5, "do": "press", "key": "IBIS_ghost" },
    ]));
    h.frame().unwrap();
    h.frame().unwrap();
    let a = acks(&h);
    assert_eq!(a.iter().map(|x| (x.0, x.1)).collect::<Vec<_>>(), [(1, true), (2, true), (3, true), (4, false), (5, false)]);
    assert_eq!(a[3].2, "unknown_key");
    let st = h.state.borrow();
    assert!(st.pressed.is_empty(), "a tap is held a moment, never pressed and let go in one frame");
    // (the game names its triggers in lower case, the model files as written: the same keys)
    let held = |n: &str, d: bool| (n.to_string(), d);
    assert_eq!(st.held, [held("IBIS_menu", true), held("IBIS_7", true), held("IBIS_7", false), held("IBIS_menu", false)]);
}

// --- the driving parts: the duty strip, on time, the board, the notes and the trip card ---------

/// A company's duty with a crew sheet: sign on at the depot, a trip, a break, a relief at the
/// terminus; the clock is at 08:00 (minute 480).
fn company_assignment() -> Assignment {
    let row = |kind: &str, at: &str, from: i32, to: i32, relief: bool| bridge::StripRow { kind: kind.into(), at: at.into(), from, to, relief, ..bridge::StripRow::default() };
    let crew = bridge::Crew {
        rows: vec![
            row("sign_on", "Depot", 470, 480, false),
            bridge::StripRow { kind: "trip".into(), line: "29".into(), at: "Zoo".into(), to_place: "Hbf".into(), from: 480, to: 500, ..bridge::StripRow::default() },
            row("break", "Hbf", 500, 515, false),
            row("sign_off", "Hbf", 520, 520, true),
        ],
        changes: vec!["Overtime today".into()],
        bus: 12,
        bus_number: "4711".into(),
        notes: vec!["Door 2 sticks".into()],
        templates: vec![("clean".into(), "Please clean the bus".into())],
        checks: vec![("lights".into(), "Lights".into())],
        text_max: 140,
        low_fuel: 0.15,
        ..bridge::Crew::default()
    };
    Assignment { issued: issued(), company: "Stadtbus".into(), duty: "14".into(), line: "29".into(), tour: "1".into(), crew: Some(crew), ..Assignment::default() }
}

fn on_duty(h: &Harness, delay: f64) {
    h.state.borrow_mut().duty = Some(hub_plugin_harness::Duty { line: "29".into(), tour: "1".into(), trip: 1, trips: 2, delay });
}

fn frames(h: &mut Harness, n: usize) {
    for _ in 0..n {
        h.frame().unwrap();
    }
}

#[test]
fn the_duty_strip_is_sent() {
    let dir = installed("strip");
    bridge::write_assignment(&dir, &company_assignment()).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 40);
    let strip = sent(&h, "strip");
    let s = strip.last().expect("a strip datagram");
    assert_eq!(s["next_relief"]["at"], "Hbf");
    assert_eq!(s["next_relief"]["from"], "08:40");
    assert_eq!(s["next_relief"]["left"], 40);
    assert_eq!(s["next_break"]["at"], "Hbf");
    assert_eq!(s["piece"]["text"], "Line 29 to Hbf until 08:20");
    assert_eq!(s["notes_in"], json!(["Door 2 sticks"]));
    assert_eq!(s["templates"], json!([{ "id": "clean", "text": "Please clean the bus" }]));
    assert_eq!(s["checks"], json!([{ "id": "lights", "text": "Lights" }]));
    assert_eq!((s["text_max"].as_i64(), s["can_note"].as_bool(), s["can_check"].as_bool(), s["signed_on"].as_bool()), (Some(140), Some(true), Some(true), Some(true)));
    assert_eq!(s["relief_warning"], false);
}

#[test]
fn without_a_company_duty_the_strip_is_not_sent_and_a_switched_off_one_is_cleared() {
    let dir = installed("strip-none");
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 40);
    assert!(sent(&h, "strip").is_empty(), "no company duty: nothing to say, and nothing to clear");
    // (the strip's own switch on the board: off clears it)
    let dir = installed("strip-off");
    bridge::write_assignment(&dir, &company_assignment()).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 40);
    assert!(sent(&h, "strip").last().unwrap().get("piece").is_some());
    h.lua.load("omsi.data.board_parts.strip = false; require('duty_strip').set_enabled(false)").exec().unwrap();
    frames(&mut h, 40);
    let last = sent(&h, "strip").pop().unwrap();
    assert_eq!(last, json!({ "t": "strip" }), "a datagram with nothing but t clears the part");
}

#[test]
fn on_time_is_sent() {
    let dir = installed("ontime");
    bridge::write_assignment(&dir, &company_assignment()).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 400.0);
    frames(&mut h, 40);
    let o = sent(&h, "ontime");
    let o = o.last().expect("an ontime datagram");
    assert_eq!((o["delay"].as_f64(), o["tone"].as_str()), (Some(400.0), Some("late")));
    // (the driver switched it off for this drive: cleared)
    h.fire("ui_click", &[json!("omsihub_board"), json!("ontime_close")]).unwrap();
    frames(&mut h, 40);
    assert_eq!(sent(&h, "ontime").pop().unwrap(), json!({ "t": "ontime" }));
}

#[test]
fn the_board_parts_are_sent() {
    let dir = installed("board");
    let mut a = company_assignment();
    a.fare = Some(bridge::Fare { per_boarding: 2.0, symbol: "€".into(), after: false, places: 2 });
    a.contract = Some(bridge::ContractNote { kind: "gross".into(), rag: "green".into(), early: 1, late: 5 });
    bridge::write_assignment(&dir, &a).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    h.lua.load("omsi.data.board_parts.strip = true").exec().unwrap();
    frames(&mut h, 40);
    let b = sent(&h, "board");
    let b = b.last().expect("a board datagram");
    assert_eq!(b["parts"]["strip"], true);
    let parts = &b["parts"];
    assert_eq!((parts["coach"].as_bool(), parts["fares"].as_bool(), parts["contract"].as_bool(), parts["dispatch"].as_bool()), (Some(true), Some(true), Some(true), Some(true)));
    assert_eq!(b["status"], "On duty: trip 1 of 2");
    assert_eq!(b["contract"]["rag"], "green");
    assert!(b["fares"]["text"].as_str().unwrap().starts_with("Fare box €0.00"), "{b}");
    // (a part the driver switched off: said so, and its facts gone)
    h.lua.load("omsi.data.board_parts.contract = false").exec().unwrap();
    frames(&mut h, 40);
    let b = sent(&h, "board").pop().unwrap();
    assert_eq!(b["parts"]["contract"], false);
    assert!(b.get("contract").is_none());
}

#[test]
fn thirty_notes_arrive_paged_and_whole() {
    let dir = installed("notes");
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    h.frame().unwrap();
    // (each long enough that the kept ones do not go in one datagram)
    h.lua.load("local notes = require('notes'); for i = 1, 30 do notes.say(('n%02d '):format(i) .. ('x'):rep(1500), { title = 'T' .. i, sticky = true }) end").exec().unwrap();
    frames(&mut h, 40);
    let pages = sent(&h, "notes");
    let v = pages.iter().map(|p| p["v"].as_i64().unwrap()).max().unwrap();
    let mut last: Vec<&Value> = pages.iter().filter(|p| p["v"].as_i64() == Some(v)).collect();
    last.sort_by_key(|p| p["page"].as_i64().unwrap());
    // (sent again every 5 s: the same pages again)
    last.dedup_by_key(|p| p["page"].as_i64().unwrap());
    assert!(last.len() > 1, "more than a page");
    assert_eq!(last.len() as i64, last[0]["pages"].as_i64().unwrap());
    let all: Vec<&Value> = last.iter().flat_map(|p| p["notes"].as_array().unwrap()).collect();
    // (8 are kept, the newest first: 30 down to 23)
    let titles: Vec<&str> = all.iter().map(|x| x["title"].as_str().unwrap()).collect();
    assert_eq!(titles, ["T30", "T29", "T28", "T27", "T26", "T25", "T24", "T23"]);
    assert_eq!(all[0]["sticky"], true);
    assert!(all[0]["n"].is_number() && all[0]["icon"].is_string() && all[0]["color"].is_string());
    for (_, text) in h.state.borrow().sent.iter().filter(|(p, t)| *p == PORT && t.contains("\"t\":\"notes\"")) {
        assert!(text.len() <= 8000, "a page of {} bytes", text.len());
    }
}

#[test]
fn the_trip_card_is_sent_when_it_opens() {
    let dir = installed("card");
    bridge::write_assignment(&dir, &company_assignment()).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 25);
    assert!(sent(&h, "card").is_empty(), "no card yet");
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(95.0), json!(88.0), json!(100.0)]).unwrap();
    // (the card stays 15 s: it is sent within the first seconds)
    frames(&mut h, 8);
    let c = sent(&h, "card");
    let c = c.iter().find(|c| c["open"] == true).expect("a card datagram");
    assert_eq!(c["open"], true);
    assert_eq!(c["run"]["line"], "29");
    assert!(c["grade"].is_string() && c["score"].is_number() && c["stars"].is_number(), "{c}");
    // (the driver closed it: cleared)
    h.fire("ui_click", &[json!("omsihub_trip"), json!("close")]).unwrap();
    frames(&mut h, 40);
    assert_eq!(sent(&h, "card").pop().unwrap(), json!({ "t": "card" }));
}

#[test]
fn a_part_is_sent_again_every_five_seconds_and_never_two_in_a_tick() {
    let dir = installed("every");
    bridge::write_assignment(&dir, &company_assignment()).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 30);
    let count = |h: &Harness| ["strip", "ontime", "board", "card"].iter().map(|k| sent(h, k).len()).sum::<usize>();
    let strips = sent(&h, "strip").len();
    let mut most = 0;
    for _ in 0..80 {
        let before = count(&h);
        h.frame().unwrap();
        most = most.max(count(&h) - before);
    }
    // (a frame is a tick of a quarter of a second here: 80 of them are 20 s of the game)
    assert_eq!(most, 1, "one part at most in a tick");
    let again = sent(&h, "strip").len() - strips;
    assert!((3..=5).contains(&again), "the strip, unchanged, every 5 s: {again} times in 20 s");
}

#[test]
fn a_walk_around_of_200_checks_still_arrives() {
    let dir = installed("long-strip");
    let mut a = company_assignment();
    {
        let crew = a.crew.as_mut().unwrap();
        crew.checks = (0..200).map(|i| (format!("check{i}"), format!("Walk-around check number {i}: look at the whole of it, twice"))).collect();
        crew.templates = (0..5).map(|i| (format!("t{i}"), format!("Template {i}"))).collect();
        crew.notes = (0..5).map(|i| format!("Handover note {i}")).collect();
    }
    bridge::write_assignment(&dir, &a).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 40);
    let texts: Vec<String> = h.state.borrow().sent.iter().filter(|(p, t)| *p == PORT && t.contains("\"t\":\"strip\"")).map(|(_, t)| t.clone()).collect();
    let text = texts.last().expect("a strip datagram");
    assert!(text.len() <= 8000, "{} bytes", text.len());
    let s: Value = serde_json::from_str(text).unwrap();
    let checks = s["checks"].as_array().expect("checks are there");
    assert!(!checks.is_empty() && checks.len() < 200);
    assert_eq!(s["more"]["checks"].as_i64(), Some(200 - checks.len() as i64));
    assert_eq!(checks[0]["id"], "check0", "the order kept");
    assert_eq!(s["templates"].as_array().unwrap().len(), 5, "the short lists whole");
    assert!(h.log_text().contains("the strip is too long"), "said in the log once");
    assert_eq!(h.log_text().matches("the strip is too long").count(), 1);
}

#[test]
fn a_card_of_60_stops_keeps_its_stops() {
    let dir = installed("card-60");
    bridge::write_assignment(&dir, &company_assignment()).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 25);
    // (a trip as the game measures it: every time with its fractions, each stop served)
    h.lua
        .load(
            "local visits = {} for i = 1, 60 do local arr = 28800.3333333 + 60 * i local dep = arr + 30.7777777 visits[i] = { index = i - 1, name = 'Stop number ' .. i .. ' of the long line', arr = arr, dep = dep, arrived = arr + 12.3456789, left = dep + 14.1234567 } end              require('card').hold({ line = '29', tour = '1', terminus = 'Hbf', trip = 1, trips = 2, departure = 28800.5, arrival = 32400.25, free = false, completed = true, planned = 60, stops = 60, early = 0, late = 2, worst = 301.123456, average = 40.5678901, seconds = 3600.123456, metres = 18000.987654, jolts = 1, crashes = 0, passengers = 40, visits = visits }, true, true)",
        )
        .exec()
        .unwrap();
    frames(&mut h, 8);
    let texts: Vec<String> = h.state.borrow().sent.iter().filter(|(p, t)| *p == PORT && t.contains("\"t\":\"card\"") && t.contains("open")).map(|(_, t)| t.clone()).collect();
    let text = texts.last().expect("a card datagram");
    assert!(text.len() <= 8000, "{} bytes", text.len());
    let c: Value = serde_json::from_str(text).unwrap();
    let visits = c["run"]["visits"].as_array().expect("the stops are there");
    assert_eq!(visits.len(), 60);
    assert!(visits[1]["arr"].as_i64().is_some() && visits[1]["arrived"].as_i64().is_some(), "whole seconds: {}", visits[1]);
}

#[test]
fn a_cut_text_stays_whole_characters() {
    let dir = installed("cut-utf8");
    let mut a = company_assignment();
    a.crew.as_mut().unwrap().checks = (0..200).map(|i| (format!("check{i}"), format!("x{}", "ä".repeat(100)))).collect();
    bridge::write_assignment(&dir, &a).unwrap();
    queue(&dir, 5, json!([]));
    let mut h = Harness::load(&dir, game()).unwrap();
    on_duty(&h, 0.0);
    frames(&mut h, 40);
    let texts: Vec<String> = h.state.borrow().sent.iter().filter(|(p, t)| *p == PORT && t.contains("\"t\":\"strip\"")).map(|(_, t)| t.clone()).collect();
    let s: Value = serde_json::from_str(texts.last().expect("a strip datagram")).expect("valid JSON");
    let text = s["checks"][0]["text"].as_str().unwrap();
    assert!(text.len() < 200 && !text.is_empty() && text.chars().skip(1).all(|c| c == 'ä'), "cut on a whole character: {text}");
}
