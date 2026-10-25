//! The phone's tabs on a duty as a driver sees them, written out as the panels the game gets
//! (`PHONE_LOOK=<file> cargo test -p hub-plugin-harness --test phone_look -- --ignored`), for
//! tools/phone-look.html to draw: a look at the phone's design without openOMSI.

mod common;

use common::*;
use hub_plugin_harness::{Stop, Trip};
use serde_json::{json, Value};

/// A duty of six trips on lines 307 and 314 with an empty run to the depot, the bus on its
/// third trip, three minutes late, 23 aboard.
fn game() -> hub_plugin_harness::GameState {
    let names = [
        "Großweier", "Landwirtschaftsmuseum", "Waldkloster", "Entenstallhütte", "Bergdorf", "Im Elend",
        "Schloss Larnhalt", "Larnhalt, Kirchstraße", "Mühl, Albert-Einstein-Gymnasium", "Waldsee", "Stadtklinik",
        "Alte Ziegelei", "Altstadtring", "Stadttheater",
    ];
    let trip = |line: &str, to: &str, dep: f64, n: usize| {
        let stops: Vec<Stop> = (0..n).map(|k| Stop { id: k as i64 + 1, name: names[k % names.len()].into(), arr: dep + 150.0 * k as f64, dep: dep + 150.0 * k as f64 }).collect();
        Trip { name: format!("{line} {to}"), line: line.into(), terminus: format!("{to}          {line}"), departure: dep, arrival: dep + 150.0 * (n - 1) as f64, stops }
    };
    let h = |hh: f64, mm: f64| hh * 3600.0 + mm * 60.0;
    let mut g = docked_game();
    g.trips = vec![
        trip("X", "Betriebsfahrt", h(5.0, 44.0), 3),
        trip("307", "Bismarckplatz", h(6.0, 7.0), 10),
        trip("314", "Stadttheater", h(9.0, 4.0), 14),
        trip("307", "Markgraf-Berthold-Platz", h(10.7, 0.0), 10),
        trip("307", "Bismarckplatz", h(11.5, 0.0), 10),
        trip("X", "Betriebshof VBBH", h(12.6, 0.0), 3),
    ];
    g.fuel = Some(140.0);
    g.passengers = 23;
    g.standing = 5;
    g.speed = 38.0;
    g.sales = (41, 96.4);
    g
}

#[test]
#[ignore]
fn the_phone_s_tabs_written_out() {
    let out = std::env::var("PHONE_LOOK").unwrap_or_else(|_| "target/phone-look.json".into());
    let mut looks = Vec::new();
    // (the board under the map on the dark and the light card, and the whole frame)
    for (light_card, frame) in [(false, false), (true, false), (false, true)] {
        let a = if light_card { light(assignment()) } else { assignment() };
        let tag = if frame { "look-frame" } else if light_card { "look-light" } else { "look-dark" };
        let (dir, mut h) = start(tag, Some(a), hub_plugin_harness::GameState { frame, ..game() }, 2);
        {
            let mut s = h.state.borrow_mut();
            s.clock = 9.0 * 3600.0 + 12.0 * 60.0;
            if let Some(d) = s.duty.as_mut() {
                d.trip = 3;
                d.delay = 200.0;
            }
            s.passed = 5;
        }
        std::fs::create_dir_all(dir.join("data")).unwrap();
        let inbox = json!({ "messages": [
            { "id": 7, "time": "09:08", "from": "Control", "text": "Turn back at Stadtklinik, the Altstadtring is closed", "priority": "instruction", "state": "sent", "action": { "kind": "short_turn", "stop": "Stadtklinik" } },
            { "id": 6, "time": "08:41", "from": "Control", "text": "Good morning, roadworks on the Altstadtring until noon", "priority": "info", "state": "read" },
        ], "replies": [], "quick": [] });
        std::fs::write(dir.join("data").join("messages.json"), inbox.to_string()).unwrap();
        run(&mut h, 3);
        for (n, tab) in ["ride", "stops", "duty", "messages", "bus"].iter().enumerate() {
            choose_tab(&h, n as i64 + 1);
            run(&mut h, 1);
            looks.push(json!({ "tab": tab, "light": light_card, "frame": frame, "panel": phone(&h) }));
        }
        no_ui_errors(&h);
    }
    std::fs::write(&out, serde_json::to_string_pretty(&Value::Array(looks)).unwrap()).unwrap();
}
