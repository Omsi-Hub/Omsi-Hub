//! A company's own lines for a player of its session (Luc, 2026-11-07: "eigen lijnen voor andere
//! leden van het bedrijf"): the boss's registry of the map written into the player's content
//! folder before his game starts on the company's duty (`lines::write_company_lines`), in place
//! of his own lines - and his own (or none) written back before his next drive of his own
//! (`lines::write_if_needed`). Against a synthetic OMSI 2 folder and openOMSI in a temporary
//! folder (`OMSIHUB_DATA_DIR`, `OMSIHUB_HOME_DIR`): nothing of the computer's own is touched.

use hub_core::lines::{self, Direction, LaneStep, Leg, Registry, StopRef};
use std::path::{Path, PathBuf};

fn stop(id: i64, name: &str, x: f64) -> StopRef {
    StopRef { tile: [0, 0], id, name: name.into(), at: [x, 0.0], ..Default::default() }
}

fn leg(len: f32, ids: &[i64]) -> Leg {
    Leg {
        steps: ids.iter().map(|i| LaneStep { tile: [0, 0], id: *i, path: 0, reversed: false, length: len / ids.len() as f32 }).collect(),
        length: len,
        ok: true,
        from_s: 2.0,
        to_s: 5.0,
        from_lat: 2.5,
        to_lat: 2.5,
        vias: Vec::new(),
        map_way: None,
    }
}

/// A registry of the map with one line, number `number`: three stops out and three back, every day.
fn registry(number: &str, company: &str) -> Registry {
    let mut reg = Registry { map: "Linedorf".into(), global: "maps/Linedorf/global.cfg".into(), ..Default::default() };
    let l = reg.add_line("Busses");
    l.number = number.into();
    l.name = format!("Line {number}");
    l.company = company.into();
    let out = Direction {
        terminus: "Markt".into(),
        stops: vec![stop(1, "Bahnhof", 0.0), stop(2, "Kirche", 500.0), stop(3, "Markt", 900.0)],
        legs: vec![leg(500.0, &[10, 11]), leg(400.0, &[11, 12])],
        ..Default::default()
    };
    let back = Direction { stops: vec![stop(4, "Markt", 900.0), stop(5, "Kirche", 500.0), stop(6, "Bahnhof", 0.0)], legs: vec![leg(400.0, &[13]), leg(500.0, &[14])], ..Default::default() };
    l.directions = vec![out, back];
    for d in &mut l.directions {
        d.refresh_times();
    }
    assert!(lines::written(l), "{:?}", lines::problems(l));
    reg
}

/// The files the line editor wrote into a `TTData` folder (its own, `oo_…`).
fn own_files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_lowercase()).filter(|n| n.starts_with("oo_")).collect();
    v.sort();
    v
}

#[test]
fn a_company_s_lines_are_written_for_its_duty_and_the_player_s_own_come_back() {
    let base = std::env::temp_dir().join(format!("hub-api-company-lines-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let root = base.join("OMSI 2");
    std::fs::create_dir_all(root.join("maps/Linedorf/TTData")).unwrap();
    std::fs::write(root.join("Omsi.exe"), b"").unwrap();
    std::fs::write(root.join("maps/Linedorf/global.cfg"), "[name]\r\nLinedorf\r\n\r\n[version]\r\n14\r\n\r\n[map]\r\n0\r\n0\r\ntile_0_0.map\r\n").unwrap();
    // (an openOMSI of its own: its folder is the content folder, as one with mods in it)
    let game = base.join("openOMSI");
    std::fs::create_dir_all(game.join("Mods")).unwrap();
    std::fs::write(game.join(if cfg!(windows) { "openomsi.exe" } else { "openomsi" }), b"").unwrap();
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, base.join("data"));
    std::env::set_var(hub_core::paths::HOME_DIR_ENV, base.join("home"));
    hub_core::settings::Settings { omsi2_folder: root.to_string_lossy().into_owned(), openomsi_folder: game.to_string_lossy().into_owned(), ..Default::default() }.save().unwrap();
    let content = hub_core::paths::content_dir().expect("the content folder");
    let map = "maps/Linedorf/global.cfg";
    let map_dir = hub_core::paths::resolve_content(map).expect("the map").parent().unwrap().to_path_buf();
    let ttdata: PathBuf = hub_core::ttstore::ttdata_dir(&content, &map_dir, "Linedorf").unwrap();

    // the player's own line 7, written as the line editor writes it
    let own_path = lines::registry_path("Linedorf");
    let mut own = registry("7", "");
    assert_eq!(lines::write_all(&own_path, &mut own, &content, &map_dir).unwrap(), None);
    assert!(own_files(&ttdata).contains(&"oo_7.ttl".to_string()), "{:?}", own_files(&ttdata));
    assert_eq!(lines::write_if_needed(map), None, "all there");

    // a duty of the boss's company on its own line 42: the boss's lines in place of his
    let company = registry("42", "boss-company");
    let done = lines::write_company_lines(map, &company).expect("written");
    assert!(done.contains("written into"), "{done}");
    let files = own_files(&ttdata);
    assert!(files.contains(&"oo_42.ttl".to_string()) && files.contains(&"oo_42_a.ttp".to_string()), "{files:?}");
    assert!(!files.iter().any(|f| f.starts_with("oo_7")), "{files:?}");
    assert!(lines::company_written("Linedorf"));
    // (his own registry as it was: the boss's lines are not his)
    let kept = lines::load_registry(&own_path);
    assert_eq!(kept.lines.len(), 1);
    assert_eq!(kept.lines[0].number, "7");

    // his next drive of his own: his line back, the company's gone
    let done = lines::write_if_needed(map).expect("written again");
    assert!(done.contains("written into"), "{done}");
    let files = own_files(&ttdata);
    assert!(files.contains(&"oo_7.ttl".to_string()), "{files:?}");
    assert!(!files.iter().any(|f| f.starts_with("oo_42")), "{files:?}");
    assert!(!lines::company_written("Linedorf"));
    assert_eq!(lines::write_if_needed(map), None, "all there again");

    // a player with no lines of his own on the map: the company's taken out again all the same
    std::fs::remove_file(&own_path).unwrap();
    lines::write_company_lines(map, &company).expect("written");
    assert!(own_files(&ttdata).contains(&"oo_42.ttl".to_string()));
    lines::write_if_needed(map).expect("taken out");
    assert!(!own_files(&ttdata).iter().any(|f| f.ends_with(".ttl")), "{:?}", own_files(&ttdata));
    assert!(!lines::company_written("Linedorf"));

    // the boss's side: his registry for the drive only when a line of it is in the timetable
    assert!(lines::for_company_drive("maps/Elsewhere/global.cfg").is_none());

    let _ = std::fs::remove_dir_all(&base);
}
