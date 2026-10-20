//! The phone's Duty and Messages tabs (`plugin/phone_duty.lua`, `plugin/phone_msgs.lua`) and the
//! duty's lines in openOMSI's pause menu (`plugin/phone_menu.lua`).

mod common;

use common::*;
use hub_game::bridge::{self, Assignment, Crew, CrewCan, RosterDuty, StripRow, TripIbis, WeekDuty};
use hub_plugin_harness::{GameState, Harness};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The plugin installed for `tag` with the assignment `a` and `more` (Lua run on the
/// assignment's table `a` before it is given: what Omsi-Hub's bridge does not write yet),
/// started on `game` with `before` run ahead of the plugin, `seconds` seconds.
fn start_with(tag: &str, a: Assignment, more: &str, game: GameState, before: &str, seconds: usize) -> (PathBuf, Harness) {
    let dir = installed(tag);
    bridge::write_assignment(&dir, &a).unwrap();
    if !more.is_empty() {
        let file = dir.join("assignment.lua");
        let lua = std::fs::read_to_string(&file).unwrap().replacen("\nreturn ", "\nlocal a = ", 1);
        std::fs::write(&file, format!("{lua}\n{more}\nreturn a\n")).unwrap();
    }
    let mut h = Harness::load_with(&dir, game, before).unwrap();
    run(&mut h, seconds);
    (dir, h)
}

/// A duty strip: signed on at 07:50, line 10 until 08:10 and back, a break at Zoo 08:30-09:00,
/// signed off at 09:20; an extra duty today.
fn crew() -> Crew {
    let row = |kind: &str, at: &str, from: i32, to: i32| StripRow { kind: kind.into(), at: at.into(), from, to, line: "10".into(), to_place: "Hbf".into(), ..StripRow::default() };
    Crew {
        rows: vec![row("sign_on", "Zoo", 470, 470), row("trip", "Zoo", 480, 490), row("trip", "Hbf", 500, 510), row("break", "Zoo", 510, 540), row("sign_off", "Depot", 560, 560)],
        changes: vec!["Extra duty: D-14 at 15:00".into()],
        bus: 7,
        bus_number: "107".into(),
        ..Crew::default()
    }
}

/// The driver's week as Omsi-Hub writes it (`launch::week_of`): today duty 12 (driven now), a
/// day off, duty 14 on Wednesday (to be swapped).
fn week() -> Vec<WeekDuty> {
    let day = |day: &str, date: &str, duty: &str, from: &str, to: &str, line: &str| WeekDuty {
        day: day.into(),
        date: date.into(),
        duty: duty.into(),
        from: from.into(),
        to: to.into(),
        line: line.into(),
        ..WeekDuty::default()
    };
    vec![
        WeekDuty { now: true, begun: true, ..day("Mon", "2026-10-20", "12", "07:50", "09:20", "10") },
        day("Tue", "2026-10-21", "", "", "", ""),
        WeekDuty { swap: Some(RosterDuty { date: "2026-10-22".into(), line: "Linie307".into(), tour: "3".into(), duty: 13 }), ..day("Wed", "2026-10-22", "14", "15:00", "22:10", "307") },
    ]
}

/// The lines the plugin wrote to `file` of its data folder, as JSON.
fn lines(dir: &Path, file: &str) -> Vec<Value> {
    std::fs::read_to_string(dir.join("data").join(file)).unwrap_or_default().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn write_inbox(dir: &Path, inbox: Value) {
    std::fs::create_dir_all(dir.join("data")).unwrap();
    std::fs::write(dir.join("data").join("messages.json"), inbox.to_string()).unwrap();
}

/// The pause menu's lines: (id, label).
fn menu(h: &Harness) -> Vec<(String, String)> {
    h.state.borrow().menu.clone().unwrap_or_default().into_iter().map(|m| (m.0, m.1)).collect()
}

#[test]
fn the_duty_tab_shows_the_strip_the_trips_and_the_ibis_codes() {
    // (the codes as Omsi-Hub writes them: found by the trip's departure)
    let code = |trip: u32, dep: f64, line: &str, route: &str, dest: &str| TripIbis { trip, dep, line: line.into(), route: route.into(), dest: dest.into(), ..TripIbis::default() };
    let a = Assignment { crew: Some(crew()), ibis_codes: vec![code(1, 28_800.0, "30701", "02", "1234"), code(2, 30_000.0, "30702", "01", "1001")], ..assignment() };
    let (_, mut h) = start_with("duty-tab", a, "", docked_game(), "", 3);
    choose_tab(&h, 3);
    // the strip in the phone's look: what is under way, the next break counting down from the
    // sign-on, the duty's end, today's change - and no strip of its own
    assert!(shows(&h, PHONE, "Line 10 to Hbf until 08:10"), "{}", phone(&h));
    assert!(shows(&h, PHONE, "NEXT BREAK") && shows(&h, PHONE, "in 30 min"), "{}", phone(&h));
    let bar = find(&h, PHONE, |e| e["type"] == "bar").expect("the countdown");
    assert!((bar["value"].as_f64().unwrap() - 0.25).abs() < 1e-6, "{bar}");
    assert!(shows(&h, PHONE, "End of duty") && shows(&h, PHONE, "Extra duty: D-14 at 15:00"));
    assert!(!has_panel(&h, "omsihub_strip"));
    // the duty's trips, the one under way marked; the IBIS codes of this trip
    assert!(has_element(&h, PHONE, |e| e["type"] == "table" && e["rows"][0][0]["text"] == "▶"));
    for code in ["30701", "02", "1234"] {
        assert!(has_element(&h, PHONE, |e| e["type"] == "text" && e["text"] == code && e["size"] == 22), "{code}: {}", phone(&h));
    }
    // the strip's note and walk-around, routed to the strip
    assert!(has_element(&h, PHONE, |e| e["id"] == "strip_check"));
    click(&h, "strip_check");
    assert!(has_element(&h, PHONE, |e| e["id"] == "strip_chk_ok"), "{}", phone(&h));
    click(&h, "strip_chk_ok");
    // the trip done: the codes of the next one (the break before it)
    {
        let mut g = h.state.borrow_mut();
        g.trip_done = true;
        g.clock = 29_500.0;
    }
    run(&mut h, 25);
    choose_tab(&h, 3);
    assert!(shows(&h, PHONE, "30702"), "{}", phone(&h));
    no_ui_errors(&h);
}

#[test]
fn the_ibis_switch_is_there_only_where_the_game_types_the_ibis() {
    let (_, h) = start_with("ibis-on", assignment(), "", docked_game(), "", 3);
    choose_tab(&h, 3);
    let switch = find(&h, PHONE, |e| e["id"] == "duty_ibis").expect("the switch");
    assert_eq!(switch["checked"], true, "the game types it, as Omsi-Hub says");
    // the driver types it: the game told so
    h.fire("ui_change", &[json!(PHONE), json!("duty_ibis"), json!(false)]).unwrap();
    assert_eq!(h.state.borrow().ibis.last().map(String::as_str), Some("auto off"));
    assert_eq!(find(&h, PHONE, |e| e["id"] == "duty_ibis").unwrap()["checked"], false);
    // and back: the trip under way typed at once
    h.fire("ui_change", &[json!(PHONE), json!("duty_ibis"), json!(true)]).unwrap();
    let ibis = h.state.borrow().ibis.clone();
    assert_eq!(&ibis[ibis.len() - 2..], ["auto on", "typed"]);
    no_ui_errors(&h);
    // Omsi-Hub says the driver types it: off
    let (_, h) = start_with("ibis-off", Assignment { ibis: Some(false), ..assignment() }, "", docked_game(), "", 3);
    choose_tab(&h, 3);
    assert_eq!(find(&h, PHONE, |e| e["id"] == "duty_ibis").unwrap()["checked"], false);
    assert_eq!(h.state.borrow().ibis.last().map(String::as_str), Some("auto off"));
    // a game that types no IBIS for plugins: no switch
    let (_, h) = start_with("ibis-none", assignment(), "", docked_game(), "omsi.duty.set_auto_ibis = nil", 3);
    choose_tab(&h, 3);
    assert!(!has_element(&h, PHONE, |e| e["id"] == "duty_ibis"));
    no_ui_errors(&h);
}

#[test]
fn the_week_s_roster_is_shown_and_a_sick_call_goes_back_to_the_company() {
    let crew = Crew { week: week(), can: CrewCan { sick: true, swap: true, leave: false }, ..crew() };
    let (dir, h) = start_with("roster", Assignment { crew: Some(crew), ..assignment() }, "", docked_game(), "", 3);
    choose_tab(&h, 3);
    assert!(shows(&h, PHONE, "THIS WEEK"), "{}", phone(&h));
    assert!(has_element(&h, PHONE, |e| e["type"] == "table" && e["rows"][2][1]["text"] == "14" && e["rows"][1][1]["text"] == "Day off"), "{}", phone(&h));
    // only what the company takes
    assert!(has_element(&h, PHONE, |e| e["id"] == "duty_sick") && has_element(&h, PHONE, |e| e["id"] == "duty_swap"));
    assert!(!has_element(&h, PHONE, |e| e["id"] == "duty_leave"));
    // a sick call: asked again, then sent - as Omsi-Hub reads it (bridge::Reply)
    click(&h, "duty_sick");
    assert!(shows(&h, PHONE, "Call in sick for duty 12?"), "{}", phone(&h));
    assert!(lines(&dir, "replies.jsonl").is_empty(), "not before it is confirmed");
    click(&h, "duty_yes");
    let replies = lines(&dir, "replies.jsonl");
    assert_eq!(replies.len(), 1);
    let r: bridge::Reply = serde_json::from_value(replies[0].clone()).expect("a reply Omsi-Hub reads");
    assert_eq!((r.kind.as_str(), r.what.as_str(), r.days, r.duty), ("crew", "sick", 1, None));
    assert!(!r.uid.is_empty());
    assert!(toasted(&h, "Sent to the company"));
    assert!(shows(&h, PHONE, "✓ Sick call"));
    // a swap: the day chosen (only those Omsi-Hub says can be swapped), then cancelled - nothing sent
    click(&h, "duty_swap");
    assert!(has_element(&h, PHONE, |e| e["id"] == "duty_day_3" && e["text"] == "Wed 22"), "{}", phone(&h));
    assert!(!has_element(&h, PHONE, |e| e["id"] == "duty_day_1"), "the duty driven now");
    assert!(!has_element(&h, PHONE, |e| e["id"] == "duty_day_2"), "a day off");
    click(&h, "duty_day_3");
    assert!(shows(&h, PHONE, "Ask to swap duty 14 on Wed 22?"), "{}", phone(&h));
    click(&h, "duty_no");
    assert!(!has_element(&h, PHONE, |e| e["id"] == "duty_yes"));
    assert_eq!(lines(&dir, "replies.jsonl").len(), 1);
    // and again, sent: the day's duty as Omsi-Hub named it, to whoever will
    click(&h, "duty_swap");
    click(&h, "duty_day_3");
    click(&h, "duty_yes");
    let r: bridge::Reply = serde_json::from_value(lines(&dir, "replies.jsonl")[1].clone()).unwrap();
    assert_eq!((r.what.as_str(), r.with), ("swap", 0));
    assert_eq!(r.duty, Some(RosterDuty { date: "2026-10-22".into(), line: "Linie307".into(), tour: "3".into(), duty: 13 }));
    no_ui_errors(&h);
}

#[test]
fn leave_is_asked_for_a_day_after_today() {
    let crew = Crew { week: week(), can: CrewCan { sick: true, swap: false, leave: true }, ..crew() };
    let (dir, h) = start_with("leave", Assignment { crew: Some(crew), ..assignment() }, "", docked_game(), "", 3);
    choose_tab(&h, 3);
    assert!(!has_element(&h, PHONE, |e| e["id"] == "duty_swap"));
    click(&h, "duty_leave");
    assert!(shows(&h, PHONE, "Leave on which day?"), "{}", phone(&h));
    assert!(!has_element(&h, PHONE, |e| e["id"] == "duty_day_1"), "not today");
    click(&h, "duty_day_3");
    assert!(shows(&h, PHONE, "Ask for leave on Wed 22 (duty 14)?"), "{}", phone(&h));
    click(&h, "duty_yes");
    let r: bridge::Reply = serde_json::from_value(lines(&dir, "replies.jsonl")[0].clone()).unwrap();
    assert_eq!((r.what.as_str(), r.from.as_str(), r.until.as_str(), r.reason.as_str()), ("leave", "2026-10-22", "2026-10-22", "personal"));
    no_ui_errors(&h);
}

#[test]
fn an_instruction_is_accepted_or_declined_on_the_messages_tab_and_control_is_told() {
    let (dir, mut h) = start("msgs-answer", Some(assignment()), docked_game(), 1);
    write_inbox(
        &dir,
        json!({ "messages": [
            { "id": 9, "time": "08:06", "from": "Control", "text": "Turn back at Markt", "priority": "urgent", "state": "sent", "action": { "kind": "short_turn" } },
            { "id": 8, "time": "08:05", "from": "Control", "text": "Hold 2 min at the next stop", "priority": "instruction", "state": "sent", "action": { "kind": "hold", "minutes": 2 } },
            { "id": 5, "time": "08:01", "from": "Control", "text": "Good morning", "priority": "info", "state": "sent" },
        ], "replies": [{ "key": "ok", "label": "OK" }], "quick": [{ "key": "traffic", "label": "Delay: traffic", "icon": "traffic" }, { "key": "breakdown", "label": "Breakdown", "icon": "build" }] }),
    );
    run(&mut h, 3);
    // the instructions bring the tab up; the urgent one says so by its icon, not a colour
    assert_eq!(tabs(&h).unwrap().1, 4);
    let urgent = find(&h, PHONE, |e| e["type"] == "icon" && e["name"] == "warning").expect("the urgent one's icon");
    assert_eq!(urgent["color"], "#F2F2F7");
    let accept = find(&h, PHONE, |e| e["id"] == "msg_accept_8").unwrap();
    let decline = find(&h, PHONE, |e| e["id"] == "msg_decline_8").unwrap();
    assert_eq!((accept["text"].as_str(), accept["color"].as_str()), (Some("Accept"), Some("#FFD23F")));
    assert_eq!((decline["text"].as_str(), decline["color"].as_str()), (Some("Can't"), Some("#48484A")));
    click(&h, "msg_decline_9");
    click(&h, "msg_accept_8");
    // a quick report to control
    click(&h, "msg_quick");
    assert!(has_element(&h, PHONE, |e| e["id"] == "msg_q_traffic"), "{}", phone(&h));
    click(&h, "msg_q_traffic");
    let replies = lines(&dir, "replies.jsonl");
    let said = |kind: &str, id: i64| replies.iter().any(|r| r["kind"] == kind && r["id"] == id);
    assert!(said("decline", 9) && said("accept", 8), "{replies:?}");
    assert!(replies.iter().any(|r| r["kind"] == "quick" && r["key"] == "traffic"));
    // the info shown with them told read; the instructions are answered instead
    assert!(said("read", 5) && !said("read", 8) && !said("read", 9), "{replies:?}");
    assert!(!has_element(&h, PHONE, |e| e["id"] == "msg_accept_8"));
    no_ui_errors(&h);
}

#[test]
fn a_message_is_told_read_once_it_is_shown_on_the_messages_tab() {
    let (dir, mut h) = start("msgs-read", Some(assignment()), docked_game(), 1);
    write_inbox(&dir, json!({ "messages": [{ "id": 5, "time": "08:01", "from": "Control", "text": "Good morning", "priority": "info", "state": "sent" }], "replies": [], "quick": [] }));
    run(&mut h, 3);
    // delivered, not read while the driver looks at the Ride tab
    let kinds = |dir: &Path| lines(dir, "replies.jsonl").iter().map(|r| r["kind"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(kinds(&dir), ["delivered"]);
    choose_tab(&h, 4);
    run(&mut h, 3);
    assert_eq!(kinds(&dir), ["delivered", "read"], "once");
    no_ui_errors(&h);
}

#[test]
fn a_shorter_layover_accepted_is_said_at_the_terminus() {
    let (dir, mut h) = start("layover", Some(assignment()), docked_game(), 1);
    write_inbox(
        &dir,
        json!({ "messages": [{ "id": 4, "time": "08:02", "from": "Control", "text": "Take a shorter layover before the 08:20 trip", "priority": "instruction", "state": "sent",
            "action": { "kind": "layover", "departure": 30000.0, "minutes": 3 } }], "replies": [], "quick": [] }),
    );
    run(&mut h, 3);
    click(&h, "msg_accept_4");
    run(&mut h, 2);
    assert!(!toasted(&h, "Shorter layover"), "not on the trip before");
    h.state.borrow_mut().trip_done = true;
    run(&mut h, 2);
    assert!(toasted(&h, "Shorter layover: 3 min less, leave for the 08:20 trip"), "{:?}", h.state.borrow().toasts);
    no_ui_errors(&h);
}

#[test]
fn the_pause_menu_has_the_duty_s_lines_and_they_open_the_phone_s_tabs() {
    let mut g = docked_game();
    g.menu = Some(Vec::new());
    let (dir, mut h) = start("menu", Some(assignment()), g, 1);
    write_inbox(&dir, json!({ "messages": [], "replies": [], "quick": [{ "key": "traffic", "label": "Delay: traffic", "icon": "traffic" }] }));
    run(&mut h, 3);
    assert_eq!(menu(&h), [("duty".to_string(), "Duty D-12 · trip 1/2".to_string()), ("control".into(), "Message to control".into()), ("end".into(), "End the duty".into())]);
    assert_eq!(h.state.borrow().menu.as_ref().unwrap()[0].2.as_deref(), Some("badge"));
    // the duty: its tab
    assert!(h.pick_menu_item("duty").unwrap());
    assert_eq!(tabs(&h).unwrap().1, 3);
    // a message to control: the Messages tab, its quick reports open
    h.pick_menu_item("control").unwrap();
    assert_eq!(tabs(&h).unwrap().1, 4);
    assert!(has_element(&h, PHONE, |e| e["id"] == "msg_q_traffic"), "{}", phone(&h));
    // ending the duty: asked on the Duty tab, then the trip reported as not completed
    h.pick_menu_item("end").unwrap();
    assert_eq!(tabs(&h).unwrap().1, 3);
    assert!(shows(&h, PHONE, "End the duty now?"), "{}", phone(&h));
    click(&h, "duty_yes");
    assert!(h.state.borrow().duty.is_none(), "finished");
    let reports = lines(&dir, "reports.jsonl");
    assert!(reports.iter().any(|r| r["kind"] == "trip" && r["how"] == "ended" && r["trip"] == 1), "{reports:?}");
    // the duty over: its lines go
    run(&mut h, 2);
    assert!(menu(&h).is_empty(), "{:?}", menu(&h));
    assert!(lines(&dir, "reports.jsonl").iter().any(|r| r["kind"] == "duty_end"));
    no_ui_errors(&h);
}

#[test]
fn without_the_phone_the_pause_menu_opens_the_panels_and_asks_twice_to_end() {
    let mut g = old_game();
    g.menu = Some(Vec::new());
    let (dir, mut h) = start("menu-old", Some(assignment()), g, 1);
    write_inbox(&dir, json!({ "messages": [], "replies": [], "quick": [{ "key": "traffic", "label": "Delay: traffic", "icon": "traffic" }] }));
    run(&mut h, 3);
    assert!(!has_panel(&h, PHONE));
    h.pick_menu_item("duty").unwrap();
    assert!(toasted(&h, "Duty D-12 · trip 1/2"));
    // the inbox's own panel, its quick reports open, with the mouse
    h.pick_menu_item("control").unwrap();
    assert!(has_element(&h, "omsihub_msgs", |e| e["id"] == "msg_q_traffic"), "{:?}", h.state.borrow().panels.keys().collect::<Vec<_>>());
    assert!(h.lua.load("return omsi.ui.focused()").eval::<bool>().unwrap());
    // ending the duty: chosen twice
    h.pick_menu_item("end").unwrap();
    assert!(h.state.borrow().duty.is_some());
    assert!(toasted(&h, "again"));
    assert!(menu(&h).iter().any(|m| m.0 == "end" && m.1 == "Choose again to end the duty"), "{:?}", menu(&h));
    h.pick_menu_item("end").unwrap();
    assert!(h.state.borrow().duty.is_none());
    run(&mut h, 2);
    assert!(menu(&h).is_empty());
    no_ui_errors(&h);
}

#[test]
fn an_openomsi_without_pause_menu_lines_is_left_alone() {
    let (_, mut h) = start("menu-none", Some(assignment()), docked_game(), 3);
    assert!(h.state.borrow().menu.is_none());
    assert!(!h.pick_menu_item("duty").unwrap());
    assert!(h.lua.load("return omsi.ui.set_menu_item == nil").eval::<bool>().unwrap());
    run(&mut h, 2);
    no_ui_errors(&h);
}
