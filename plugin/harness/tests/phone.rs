//! The phone in openOMSI's navigator (`plugin/nav.lua` and its tabs): docked under the map
//! where the game has the navigator's board, the modules' own panels where it has not.

mod common;

use common::*;
use hub_game::bridge;
use hub_plugin_harness::{GameState, Harness, Stop};
use serde_json::json;

#[test]
fn the_phone_is_docked_under_the_map_with_its_status_line_and_five_tabs() {
    let (dir, h) = start("docked", Some(assignment()), docked_game(), 3);
    let p = phone(&h);
    assert_eq!(p["anchor"], "navigator");
    assert_eq!(p["grab"], true, "its tabs and buttons without F10");
    assert!(p.get("background").is_none(), "the navigator's card shows through (a background of its own would be square in its rounded card)");
    // the status line: the trip's line badge, the terminus without its code, trip 1/2, the chip
    let status = p["children"][0].clone();
    let parts = status["children"].as_array().unwrap();
    assert_eq!((parts[0]["type"].as_str(), parts[0]["text"].as_str(), parts[0]["color"].as_str()), (Some("badge"), Some("10"), Some("#FFD23F")));
    assert_eq!(parts[1]["text"], "Hbf");
    assert_eq!((parts[2]["type"].as_str(), parts[2]["text"].as_str()), (Some("badge"), Some("1/2")));
    // (42 s late is on time, as on the navigator's map: a pill of the time's colour, white text)
    assert_eq!((parts[3]["text"].as_str(), parts[3]["color"].as_str(), parts[3]["text_color"].as_str()), (Some("on time"), Some("#2FA84F"), Some("#FFFFFF")));
    let (names, selected) = tabs(&h).expect("the tabs");
    assert_eq!(names, TAB_IDS);
    assert_eq!(selected, 1);
    // (the tab shown in Omsi-Hub's blue with its icon and name, the others their icon on the navy)
    let ride = find(&h, PHONE, |e| e["id"] == "phone_tab_ride").unwrap();
    assert_eq!((ride["text"].as_str(), ride["icon"].as_str(), ride["color"].as_str()), (Some("Ride"), Some("navigation"), Some("#2A75F7")));
    let bus = find(&h, PHONE, |e| e["id"] == "phone_tab_bus").unwrap();
    assert_eq!((bus["text"].as_str(), bus["icon"].as_str(), bus["color"].as_str()), (Some(""), Some("directions_bus"), Some("#2C3240")));
    // the board and the company's panel are in the phone, not panels of their own
    for id in ["omsihub_board", "omsihub", "omsihub_strip", "omsihub_ontime"] {
        assert!(!has_panel(&h, id), "{id}");
    }
    assert!(has_element(&h, PHONE, |e| e["text"] == "Zoo" && e["size"] == 20), "the stop ahead on the Ride tab: {}", phone(&h));
    assert!(has_element(&h, PHONE, |e| e["type"] == "box" && e["color"] == "#2A75F7" && elements(e).iter().any(|c| c["text"] == "Zoo")), "on its blue card: {}", phone(&h));
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
    run(&mut h, 6);
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
    run(&mut h, 7);
    assert!(has_panel(&h, "omsihub"));
    no_ui_errors(&h);
}

#[test]
fn the_board_on_the_screen_lists_as_many_stops_as_the_height_the_player_gave_it() {
    // (an openOMSI whose panels the player resizes, its navigator showing no board; a trip of ten
    // stops)
    let mut g = GameState { ui_version: 6, dock: None, ..docked_game() };
    let names = ["Zoo", "Markt", "Rathaus", "Post", "Kirche", "Schule", "Park", "Bad", "Feld", "Hbf"];
    g.trips[0].stops = names.iter().enumerate().map(|(k, n)| Stop { id: k as i64 + 1, name: n.to_string(), arr: 28_800.0 + 60.0 * k as f64, dep: 28_800.0 + 60.0 * k as f64 }).collect();
    let (_, mut h) = start("board-height", Some(assignment()), g, 8);
    let stops = |h: &Harness| find(h, "omsihub_board", |e| e["type"] == "stops").map_or(0, |e| e["stops"].as_array().unwrap().len());
    let board = panel(&h, "omsihub_board");
    assert_eq!((board["anchor"].as_str(), board["resizable"].as_bool()), (Some("left"), Some(true)));
    assert_eq!(stops(&h), 4, "as before: the next four");
    // made taller: all ten; made short: one
    h.state.borrow_mut().resized.insert("omsihub_board".into(), (340.0, 600.0));
    h.fire("ui_resize", &[json!("omsihub_board"), json!(340.0), json!(600.0)]).unwrap();
    run(&mut h, 1);
    assert_eq!(stops(&h), 10, "{}", panel(&h, "omsihub_board"));
    assert_eq!(panel(&h, "omsihub_board")["height"].as_f64(), Some(600.0));
    h.state.borrow_mut().resized.insert("omsihub_board".into(), (340.0, 190.0));
    h.fire("ui_resize", &[json!("omsihub_board"), json!(340.0), json!(190.0)]).unwrap();
    run(&mut h, 1);
    assert_eq!(stops(&h), 1);
    // the phone in the navigator is never resizable
    h.state.borrow_mut().dock = Some((360.0, 420.0, 0.0, true));
    run(&mut h, 1);
    assert!(phone(&h).get("resizable").is_none());
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
    assert_eq!(phone(&h)["children"][0]["children"][1]["color"], "#E8EBF2");
    let (_, h) = start("light", Some(light(assignment())), docked_game(), 2);
    assert_eq!(phone(&h)["children"][0]["children"][1]["color"], "#1E1C22");
    // (the tab shown in the blue on either card, the others light on the white one)
    assert_eq!(find(&h, PHONE, |e| e["id"] == "phone_tab_ride").unwrap()["color"], "#2A75F7");
    assert_eq!(find(&h, PHONE, |e| e["id"] == "phone_tab_stops").unwrap()["color"], "#EDE9E3");
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
    assert!(trip_under_way(&h).is_some_and(|r| r[2]["text"] == "Hbf"), "{}", phone(&h));
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
    assert_eq!(tabs(&h).unwrap().0[3], "messages 1");
    let inbox = json!({ "messages": [
        { "id": 7, "time": "08:05", "from": "Control", "text": "Turn back at Markt", "priority": "instruction", "state": "sent", "action": { "kind": "short_turn", "stop": "Markt" } },
        { "id": 6, "time": "08:01", "from": "Control", "text": "Good morning", "priority": "info", "state": "read" },
    ], "replies": [], "quick": [] });
    std::fs::write(dir.join("data").join("messages.json"), inbox.to_string()).unwrap();
    run(&mut h, 3);
    assert!(toasted(&h, "Turn back at Markt"), "a card as it comes");
    assert!(!has_panel(&h, "omsihub_msgs"), "no inbox of its own");
    // the instruction brings the inbox up, read: no count
    assert_eq!(tabs(&h).unwrap(), (TAB_IDS.iter().map(|t| t.to_string()).collect(), 4));
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

#[test]
fn the_game_s_menu_hiding_the_board_flips_nothing_and_the_phone_takes_over_as_soon_as_it_is_back() {
    let (_, mut h) = start("menu-hides", Some(assignment()), docked_game(), 3);
    // the game's menu open in a LAN session (the game runs on), the board hidden with it
    {
        let mut g = h.state.borrow_mut();
        g.dock = None;
        g.game_menu = true;
    }
    run(&mut h, 20);
    assert!(!has_panel(&h, "omsihub") && !has_panel(&h, "omsihub_board"), "not counted as the board gone");
    // the city map says nothing: past the grace, the panels of their own
    h.state.borrow_mut().game_menu = false;
    run(&mut h, 7);
    assert!(has_panel(&h, "omsihub") && has_panel(&h, "omsihub_board"));
    // the board back: in that very frame the phone has them, nothing beside it
    h.state.borrow_mut().dock = Some((360.0, 420.0, 0.0, true));
    h.fire("frame", &[json!(0.016)]).unwrap();
    assert!(!has_panel(&h, "omsihub") && !has_panel(&h, "omsihub_board"));
    assert_eq!(phone(&h)["anchor"], "navigator");
    no_ui_errors(&h);
}

#[test]
fn a_count_on_a_tab_shows_beside_its_icon_and_the_tab_shown_says_its_name() {
    let dir = installed("tab-count");
    bridge::write_assignment(&dir, &assignment()).unwrap();
    let texts = std::collections::BTreeMap::from([("Inbox".to_string(), "Posteingang".to_string())]);
    bridge::write_texts(&dir, &texts).unwrap();
    let mut h = hub_plugin_harness::Harness::load(&dir, docked_game()).unwrap();
    run(&mut h, 2);
    std::fs::create_dir_all(dir.join("data")).unwrap();
    let inbox = json!({ "messages": [
        { "id": 6, "time": "08:01", "from": "Control", "text": "Guten Morgen", "priority": "info", "state": "sent" },
        { "id": 5, "time": "08:00", "from": "Control", "text": "Hallo", "priority": "info", "state": "sent" },
        { "id": 4, "time": "07:59", "from": "Control", "text": "Servus", "priority": "info", "state": "sent" },
    ], "replies": [], "quick": [] });
    std::fs::write(dir.join("data").join("messages.json"), inbox.to_string()).unwrap();
    run(&mut h, 3);
    // (a tab not shown is its icon: the count beside it, on the navy that stands out)
    assert_eq!(tabs(&h).unwrap().0[3], "messages 3");
    assert_eq!(find(&h, PHONE, |e| e["id"] == "phone_tab_messages").unwrap()["color"], "#2A3A5C");
    // shown, it says its name in the player's language, whole
    choose_tab(&h, 4);
    assert_eq!(find(&h, PHONE, |e| e["id"] == "phone_tab_messages").unwrap()["text"], "Posteingang");
    no_ui_errors(&h);
}

/// The icons of openOMSI 0.3.37 (`assets/icons/{material,custom}`; 0.3.38 adds build, edit):
/// a name the game has not draws nothing.
const GAME_ICONS: &str = "stop_request ac_unit account_circle add air airline_seat_recline_normal airport_shuttle alt_route apps arrow_back arrow_upward autorenew back_hand badge bolt calendar_month campaign chat check check_circle chevron_left chevron_right close cloud cloud_download commute confirmation_number construction content_copy dark_mode delete departure_board description directions_bus directions_car directions_walk display_settings dns door_front door_sliding download drive_folder_upload emergency_home emoji_events error event exit_to_app expand_less expand_more extension filter_list flag flashlight_on foggy folder folder_open fork_left fork_right forum fullscreen gamepad garage grid_view group groups headphones help highlight history home hub info install_desktop install_mobile inventory_2 key keyboard keyboard_arrow_down keyboard_arrow_up lan language light light_mode link local_car_wash local_gas_station local_parking location_on lock logout map menu military_tech mobile_friendly monitor mouse my_location navigation near_me nights_stay north open_in_new open_with palette pan_tool_alt partly_cloudy_day pause payments person photo_camera play_arrow power_settings_new public radio rainy receipt_long refresh remove restart_alt roundabout_left roundabout_right route save schedule school screen_rotation sd_card search settings signal_cellular_alt smartphone sort south speed sports_esports sports_score star stars stop stop_circle straight sync_alt system_update terminal thermostat thunderstorm timer today touch_app traffic trending_up tune turn_left turn_right turn_slight_left turn_slight_right u_turn_left unfold_more upload vibration videocam view_list visibility volume_up warning water_drop wb_sunny wb_twilight weather_snowy wifi wifi_tethering zoom_in zoom_out";

/// The quoted name right after `at` in `line` (`"name"` with letters, digits and _ only).
fn quoted(line: &str, at: usize) -> Option<&str> {
    let rest = line[at..].strip_prefix('"')?;
    let end = rest.find('"')?;
    let name = &rest[..end];
    (!name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')).then_some(name)
}

#[test]
fn every_icon_the_plugin_names_is_one_the_game_has() {
    let have: Vec<&str> = GAME_ICONS.split_whitespace().collect();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "lua") {
            continue;
        }
        let lua = std::fs::read_to_string(&path).unwrap();
        for (n, line) in lua.lines().enumerate() {
            let mut names = Vec::new();
            // `icon = "x"`, an icon element's `name = "x"`, a card's `notify("x"`, a line's
            // `line("x"` / `line(th, "x"`, and `and "x" or "y"` where an icon is chosen
            for key in ["icon = ", "notify(", "line(", "line(th, ", "line(ctx, ", "line(ctx.theme, "] {
                for (at, _) in line.match_indices(key) {
                    names.extend(quoted(line, at + key.len()));
                }
            }
            if line.contains("type = \"icon\"") {
                for (at, _) in line.match_indices("name = ") {
                    names.extend(quoted(line, at + 7));
                }
            }
            if line.contains("icon") || line.contains("line(") || line.contains("name = ") {
                for (at, _) in line.match_indices(" and \"") {
                    names.extend(quoted(line, at + 5));
                    if let Some(or) = line[at + 5..].find("\" or ") {
                        names.extend(quoted(line, at + 5 + or + 5));
                    }
                }
            }
            if line.trim_start().starts_with("local ICON = {") {
                for (at, _) in line.match_indices("= \"") {
                    names.extend(quoted(line, at + 2));
                }
            }
            for name in names {
                if !have.contains(&name) {
                    missing.push(format!("{}:{}: {name}", path.file_name().unwrap().to_string_lossy(), n + 1));
                }
            }
        }
    }
    assert!(missing.is_empty(), "icons the game has not: {missing:?}");
}

#[test]
fn where_the_game_has_the_whole_frame_the_phone_is_the_whole_navigator() {
    // an openOMSI that lets a panel fill the navigator's frame: the Ride tab with the map
    // under the tabs, the other tabs the whole frame for themselves
    let g = hub_plugin_harness::GameState { frame: true, ..docked_game() };
    let (_, mut h) = start("frame", Some(assignment()), g, 3);
    let p = phone(&h);
    assert_eq!((p["anchor"].as_str(), p["frame"].as_bool()), (Some("navigator"), Some(true)));
    let children = p["children"].as_array().unwrap();
    assert_eq!(children[1]["id"], "phone_tab", "the status line, then the tabs: {p}");
    assert_eq!((children[2]["type"].as_str(), children[2]["height"].as_f64()), (Some("map"), Some(210.0)), "{p}");
    assert!(shows(&h, PHONE, "Zoo"), "the stop ahead under the map: {p}");
    // (the navigator's card in Omsi-Hub's panel colour: the navigator paints it, rounded)
    assert_eq!(p["background"], "#141A26");
    for n in [2, 3, 4, 5] {
        choose_tab(&h, n);
        run(&mut h, 1);
        assert!(!has_element(&h, PHONE, |e| e["type"] == "map"), "tab {n}: {}", phone(&h));
    }
    choose_tab(&h, 1);
    assert!(has_element(&h, PHONE, |e| e["type"] == "map"));
    no_ui_errors(&h);
    // a game without it: the panel asks for the frame all the same (a key it leaves alone),
    // gets the board under the map, and no map element goes with it
    let (_, h) = start("no-frame", Some(assignment()), docked_game(), 3);
    assert_eq!(phone(&h)["frame"], true);
    assert!(!has_element(&h, PHONE, |e| e["type"] == "map"), "{}", phone(&h));
    assert!(phone(&h).get("background").is_none(), "under the map see-through: a square of its own would show");
    no_ui_errors(&h);
    // the light card: Omsi-Hub's light panel
    let g = hub_plugin_harness::GameState { frame: true, ..docked_game() };
    let (_, h) = start("frame-light", Some(light(assignment())), g, 3);
    assert_eq!(phone(&h)["background"], "#FFFEFC");
}
