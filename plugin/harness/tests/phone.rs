//! The phone in openOMSI's navigator (`plugin/nav.lua` and its tabs): docked under the map
//! where the game has the navigator's board, the modules' own panels where it has not.

mod common;

use common::*;
use hub_game::bridge;
use serde_json::json;

#[test]
fn the_phone_is_docked_under_the_map_with_its_status_line_and_five_tabs() {
    let (dir, h) = start("docked", Some(assignment()), docked_game(), 3);
    let p = phone(&h);
    assert_eq!(p["anchor"], "navigator");
    assert_eq!(p["grab"], true, "its tabs and buttons without F10");
    assert!(p.get("background").is_none(), "the navigator's card shows through");
    // the status line: the trip's line badge, the terminus without its code, trip 1/2, the chip
    let status = p["children"][0].clone();
    let parts = status["children"].as_array().unwrap();
    assert_eq!((parts[0]["type"].as_str(), parts[0]["text"].as_str(), parts[0]["color"].as_str()), (Some("badge"), Some("10"), Some("#FFD23F")));
    assert_eq!(parts[1]["text"], "Hbf");
    assert_eq!(parts[2]["text"], "1/2");
    // (42 s late is on time, as on the navigator's map)
    assert_eq!((parts[3]["text"].as_str(), parts[3]["text_color"].as_str()), (Some("on time"), Some("#2FA84F")));
    let (names, selected) = tabs(&h).expect("the tabs");
    assert_eq!(names, ["Ride", "Stops", "Duty", "Inbox", "Bus"]);
    assert_eq!(selected, 1);
    // the board and the company's panel are in the phone, not panels of their own
    for id in ["omsihub_board", "omsihub", "omsihub_strip", "omsihub_ontime"] {
        assert!(!has_panel(&h, id), "{id}");
    }
    assert!(has_element(&h, PHONE, |e| e["text"] == "Zoo" && e["size"] == 18), "the stop ahead on the Ride tab: {}", phone(&h));
    no_ui_errors(&h);
    // the handshake says what this openOMSI offers
    let hello = bridge::read_hello(&dir).unwrap();
    assert_eq!((hello.ui, hello.dock), (5, true));
}

#[test]
fn an_openomsi_without_the_navigator_s_board_keeps_the_panels_of_before() {
    let (dir, h) = start("v2", Some(assignment()), old_game(), 3);
    assert!(!has_panel(&h, PHONE));
    assert_eq!(panel(&h, "omsihub_board")["anchor"], "left");
    assert!(has_panel(&h, "omsihub"));
    no_ui_errors(&h);
    let hello = bridge::read_hello(&dir).unwrap();
    assert_eq!((hello.ui, hello.dock), (2, false));
    // (the phone switched off in Omsi-Hub: the board in the navigator, as before)
    let a = hub_game::bridge::Assignment { nav: Some(hub_game::bridge::NavLook { light: false, phone: Some(false) }), ..assignment() };
    let (_, h) = start("phone-off", Some(a), docked_game(), 3);
    assert!(!has_panel(&h, PHONE));
    assert_eq!(panel(&h, "omsihub_board")["anchor"], "navigator");
}

#[test]
fn without_the_navigator_s_board_the_panels_come_back_after_a_grace_and_go_again() {
    let (_, mut h) = start("grace", Some(assignment()), docked_game(), 3);
    assert!(!has_panel(&h, "omsihub"));
    // Shift+N put the board away: nothing changes for a few seconds
    h.state.borrow_mut().dock = None;
    run(&mut h, 3);
    assert!(!has_panel(&h, "omsihub") && !has_panel(&h, "omsihub_board"), "within the grace");
    // then the modules' own panels, the board on the screen
    run(&mut h, 1);
    assert!(has_panel(&h, "omsihub"));
    assert_eq!(panel(&h, "omsihub_board")["anchor"], "left");
    // the board back: the phone, and the panels go
    h.state.borrow_mut().dock = Some((360.0, 420.0, 0.0, true));
    run(&mut h, 1);
    assert!(!has_panel(&h, "omsihub") && !has_panel(&h, "omsihub_board"));
    assert_eq!(phone(&h)["anchor"], "navigator");
    // another plugin has the board: the panels again
    h.state.borrow_mut().dock = Some((360.0, 420.0, 0.0, false));
    run(&mut h, 4);
    assert!(has_panel(&h, "omsihub"));
    no_ui_errors(&h);
}

#[test]
fn a_short_room_under_the_map_gives_the_slim_phone() {
    let mut g = docked_game();
    g.dock = Some((360.0, 120.0, 0.0, true));
    let (_, h) = start("slim", Some(assignment()), g, 3);
    let p = phone(&h);
    let children = p["children"].as_array().unwrap();
    assert_eq!(children.len(), 2, "the status line and one line: {p}");
    assert!(tabs(&h).is_none(), "no tabs");
    assert!(shows(&h, PHONE, "Zoo"), "the next stop: {p}");
    no_ui_errors(&h);
}

#[test]
fn the_navigator_s_light_look_gets_dark_ink() {
    let (_, h) = start("dark", Some(assignment()), docked_game(), 2);
    assert_eq!(phone(&h)["children"][0]["children"][1]["color"], "#F2F2F7");
    let (_, h) = start("light", Some(light(assignment())), docked_game(), 2);
    assert_eq!(phone(&h)["children"][0]["children"][1]["color"], "#1C1C1E");
    assert_eq!(find(&h, PHONE, |e| e["type"] == "tabs").unwrap()["color"], "#1C1C1E");
    no_ui_errors(&h);
}

#[test]
fn a_tab_chosen_shows_its_content() {
    let mut g = docked_game();
    g.fuel = Some(150.0);
    g.passengers = 12;
    g.standing = 4;
    let (_, mut h) = start("tabs", Some(assignment()), g, 3);
    // Stops: the trip's stops
    choose_tab(&h, 2);
    assert_eq!(tabs(&h).unwrap().1, 2);
    assert!(has_element(&h, PHONE, |e| e["type"] == "stops" && e["stops"].as_array().is_some_and(|s| s.len() == 3)), "{}", phone(&h));
    // Duty: the company's panel's elements and the duty's trips, the one under way marked
    choose_tab(&h, 3);
    assert!(shows(&h, PHONE, "Duty D-12"), "{}", phone(&h));
    assert!(has_element(&h, PHONE, |e| e["type"] == "table" && e["rows"][0][0]["text"] == "▶"));
    // Bus: the fuel, the passengers seated and standing
    choose_tab(&h, 5);
    assert!(has_element(&h, PHONE, |e| e["type"] == "bar" && e["value"] == 1.0), "{}", phone(&h));
    assert!(shows(&h, PHONE, "8") && shows(&h, PHONE, "4"));
    // the tab stays chosen
    run(&mut h, 2);
    assert_eq!(tabs(&h).unwrap().1, 5);
    no_ui_errors(&h);
}

#[test]
fn the_control_room_s_instruction_opens_the_inbox_and_is_answered_there() {
    let (dir, mut h) = start("inbox", Some(assignment()), docked_game(), 1);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let inbox = json!({ "messages": [
        { "id": 6, "time": "08:01", "from": "Control", "text": "Good morning", "priority": "info", "state": "sent" },
    ], "replies": [], "quick": [] });
    std::fs::write(dir.join("data").join("messages.json"), inbox.to_string()).unwrap();
    run(&mut h, 3);
    // a message come in: its count on the tab
    assert_eq!(tabs(&h).unwrap().0[3], "Inbox 1");
    let inbox = json!({ "messages": [
        { "id": 7, "time": "08:05", "from": "Control", "text": "Turn back at Markt", "priority": "instruction", "state": "sent", "action": { "kind": "short_turn", "stop": "Markt" } },
        { "id": 6, "time": "08:01", "from": "Control", "text": "Good morning", "priority": "info", "state": "read" },
    ], "replies": [], "quick": [] });
    std::fs::write(dir.join("data").join("messages.json"), inbox.to_string()).unwrap();
    run(&mut h, 3);
    assert!(toasted(&h, "Turn back at Markt"), "a card as it comes");
    assert!(!has_panel(&h, "omsihub_msgs"), "no inbox of its own");
    // the instruction brings the inbox up, read: no count
    assert_eq!(tabs(&h).unwrap(), (vec!["Ride".into(), "Stops".into(), "Duty".into(), "Inbox".into(), "Bus".into()], 4));
    assert!(has_element(&h, PHONE, |e| e["id"] == "msg_accept_7"), "{}", phone(&h));
    click(&h, "msg_accept_7");
    let replies = std::fs::read_to_string(dir.join("data").join("replies.jsonl")).unwrap();
    assert!(replies.contains("\"accept\""), "{replies}");
    assert!(!has_element(&h, PHONE, |e| e["id"] == "msg_accept_7"));
    // the driver goes back to the Ride tab
    choose_tab(&h, 1);
    run(&mut h, 2);
    assert_eq!(tabs(&h).unwrap().1, 1);
    no_ui_errors(&h);
}

#[test]
fn at_the_terminus_the_trip_s_card_and_then_the_break_clock_take_the_ride_tab_over() {
    let (_, mut h) = start("break", Some(assignment()), docked_game(), 3);
    choose_tab(&h, 2);
    // the trip done at the terminus with time to spare
    {
        let mut g = h.state.borrow_mut();
        g.trip_done = true;
        g.clock = 29_500.0;
    }
    h.fire("trip_done", &[json!(1), json!("arrived"), json!(90.0), json!(90.0), json!(100.0)]).unwrap();
    run(&mut h, 1);
    assert_eq!(tabs(&h).unwrap().1, 1, "the Ride tab comes up");
    assert!(shows(&h, PHONE, "Trip done"), "{}", phone(&h));
    assert!(!has_panel(&h, "omsihub_trip"), "no card of its own");
    // the card's time over: the break clock, counting down to the next departure
    h.state.borrow_mut().clock = 29_540.0;
    run(&mut h, 1);
    assert!(has_element(&h, PHONE, |e| e["size"] == 40 && e["text"] == "7:40"), "{}", phone(&h));
    // the status line looks at the next trip, its chip at the break
    assert_eq!(phone(&h)["children"][0]["children"][2]["text"], "2/2");
    assert!(shows(&h, PHONE, "break, 8 min left"));
    no_ui_errors(&h);
}

#[test]
fn the_duty_tab_s_buttons_do_what_the_company_panel_s_did() {
    let a = hub_game::bridge::Assignment { auto_start: false, ..assignment() };
    let (_, h) = start("duty-start", Some(a), docked_game(), 2);
    choose_tab(&h, 3);
    assert!(has_element(&h, PHONE, |e| e["id"] == "start" && e["visible"] != false), "{}", phone(&h));
    click(&h, "start");
    assert_eq!(h.state.borrow().started, [("10".to_string(), "2".to_string(), 0, 0)]);
    no_ui_errors(&h);
}

#[test]
fn f9_opens_the_inbox_and_goes_back() {
    let (_, h) = start("f9", Some(assignment()), docked_game(), 2);
    h.fire("key", &[json!("F9"), json!(true)]).unwrap();
    assert_eq!(tabs(&h).unwrap().1, 4);
    h.fire("key", &[json!("F9"), json!(true)]).unwrap();
    assert_eq!(tabs(&h).unwrap().1, 1);
}
