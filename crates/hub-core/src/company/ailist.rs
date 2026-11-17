//! The company's own buses and lines alone in the game (Luc, 2026-11-17: "in company mode the
//! game shows only the company's buses and the company's lines, not the map's default").
//!
//! For a company's duty Omsi-Hub writes one Chrono folder into openOMSI's copy of the map in
//! its content folder (`maps/<map>/Chrono/zz_omsihub_company`, [`FOLDER`]): openOMSI reads the
//! Chrono folders of every copy of a map together and applies them by their names, so this one
//! comes after the map's own. Nothing of the OMSI 2 folder is changed, and the folder goes again
//! when the game ends ([`remove_all`]).
//!
//! What it holds, made from the day's plan of the company (which bus runs which tour) and its
//! fleet ([`build`]):
//!
//! * `ailists_#upd.cfg`: a depot group for every tour of the timetable the company runs,
//!   `OH_<line>_<tour>`, holding the bus the plan gives it - its `.bus`, its fleet number, its
//!   plate and its livery (a livery of the paint studio is an ordinary repaint by its name) -
//!   so the game's AI runs each tour with the company's own bus, as the plan has it;
//! * `TTData/<line>.ttl`: every line of the timetable whose bus tours the company runs part of,
//!   cut down to the company's tours (on their groups) and its tours of other traffic (trains,
//!   planes, ships keep theirs);
//! * `Chrono.cfg`: every other bus line switched off (`[deactivate_lines]`) - the map's lines the
//!   company does not run, and the own lines of other companies (the line editor writes every
//!   company's into the map's timetable).
//!
//! A tour pointed at a depot group with no bus is not enough: openOMSI still runs it on its map
//! and its minimap, without a vehicle (tested 2026-11-17). Taken out of the timetable, the game
//! does not know it at all.
//!
//! Omsi-Hub's own readers of the map leave the folder out (`hub_omsi::map::active_chrono_dirs`):
//! it is the company's day for the game, not the map's.

use super::day::TourPlan;
use super::{Company, Vehicle};
use hub_omsi::map::{AiGroup, AiLists};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The folder's name in the map's `Chrono`.
pub const FOLDER: &str = hub_omsi::map::HUB_CHRONO;

const NL: &str = "\r\n";

/// A line of the timetable as the game reads it: its file's name (`<line>.ttl`) and its text.
#[derive(Clone, Debug, PartialEq)]
pub struct LineFile {
    pub file: String,
    pub text: String,
}

/// What goes into the folder: its files (path in the folder, text) and what they do.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Folder {
    pub files: Vec<(String, String)>,
    /// The company's tours kept, the other bus tours taken out, the lines switched off.
    pub tours: usize,
    pub taken_out: usize,
    pub off: Vec<String>,
}

fn clean(s: &str) -> String {
    s.replace(['\t', '\r', '\n'], " ").trim().to_string()
}

fn key(line: &str, tour: &str) -> (String, String) {
    (line.trim().to_lowercase(), tour.trim().to_lowercase())
}

/// A depot group of the AI lists that carries buses: not one of other traffic (a map's
/// delivery robots run as a depot group of their own).
pub fn carries_buses(g: &AiGroup) -> bool {
    let n = g.name.to_lowercase();
    let other = |f: &str| {
        let f = f.to_lowercase();
        f.contains("robot") || f.contains("drohne") || f.contains("drone")
    };
    g.is_depot && !n.contains("drohne") && !n.contains("liefer") && !(!g.typgroups.is_empty() && g.typgroups.iter().all(|t| other(&t.file)))
}

/// The repaint a bus of the fleet is painted in (the house livery once painted).
fn paint_of(v: &Vehicle) -> String {
    clean(v.house_livery.as_deref().unwrap_or(&v.livery))
}

/// An `[aigroup_depot]` of the AI lists with `buses`, one typgroup for each bus file.
fn depot_group(name: &str, hof: &str, buses: &[&Vehicle]) -> String {
    let mut by_file: BTreeMap<String, Vec<&Vehicle>> = BTreeMap::new();
    for v in buses {
        by_file.entry(v.bus.replace('/', "\\")).or_default().push(v);
    }
    let mut out = vec![String::new(), "[aigroup_depot]".to_string(), clean(name), clean(hof), String::new()];
    for (file, vs) in by_file {
        out.push("[aigroup_depot_typgroup_2]".into());
        out.push(file);
        for v in vs {
            let paint = paint_of(v);
            out.push(if paint.is_empty() { format!("{}\t{}", clean(&v.number), clean(&v.plate)) } else { format!("{}\t{}\t{paint}", clean(&v.number), clean(&v.plate)) });
        }
        out.push("[end]".into());
        out.push(String::new());
    }
    out.join(NL)
}

/// The tours of the timetable (line, tour) the plan runs with a bus of the company, and with
/// which (a block of the planning may join trips of several of the timetable's tours: each is
/// the tour it came from). A tour not in service or with no bus today runs nothing.
/// Keyed by line and tour in lower case; each with the names as the timetable writes them.
pub fn planned_buses(tours: &[TourPlan]) -> BTreeMap<(String, String), (String, String, BTreeSet<u32>)> {
    let mut out: BTreeMap<(String, String), (String, String, BTreeSet<u32>)> = BTreeMap::new();
    let mut add = |line: &str, tour: &str, bus: u32| {
        out.entry(key(line, tour)).or_insert_with(|| (line.trim().to_string(), tour.trim().to_string(), BTreeSet::new())).2.insert(bus);
    };
    for tp in tours.iter().filter(|t| !t.tour.unplanned) {
        let Some(bus) = tp.bus else { continue };
        if tp.tour.trips.is_empty() {
            add(&tp.tour.line, &tp.tour.tour, bus);
        }
        for x in &tp.tour.trips {
            let tour = if x.origin.trim().is_empty() { tp.tour.tour.as_str() } else { x.origin.as_str() };
            add(tp.tour.line_of(x), tour, bus);
        }
    }
    out
}

/// The folder for the company `c` on the day of `tours` (its plan), the map's AI lists `ai`
/// and its timetable's lines as the game reads them (`lines`).
pub fn build(c: &Company, tours: &[TourPlan], ai: &AiLists, lines: &[LineFile]) -> Folder {
    let planned = planned_buses(tours);
    let bus_groups: BTreeSet<String> = ai.groups.iter().filter(|g| carries_buses(g)).map(|g| g.name.trim().to_lowercase()).collect();
    // (a group for each tour of the plan, with its buses)
    let mut groups = String::new();
    let mut group_of: BTreeMap<(String, String), String> = BTreeMap::new();
    for (k, (line, tour, buses)) in &planned {
        let vs: Vec<&Vehicle> = buses.iter().filter_map(|b| c.vehicle(*b)).collect();
        if vs.is_empty() {
            continue;
        }
        let name = format!("OH_{}_{}", clean(line), clean(tour)).replace(' ', "_");
        let hof = c.lines.iter().find(|l| l.name.trim().eq_ignore_ascii_case(line)).map(|l| l.hof.trim()).filter(|h| !h.is_empty()).unwrap_or(&c.depot);
        groups.push_str(&depot_group(&name, hof, &vs));
        group_of.insert(k.clone(), name);
    }
    let mut folder = Folder::default();
    // (each line: its bus tours not the company's taken out, the company's on their groups)
    for lf in lines {
        let line = lf.file.rsplit_once('.').map_or(lf.file.as_str(), |(s, _)| s).to_string();
        let rows: Vec<&str> = lf.text.split(NL).collect();
        let starts: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].trim().eq_ignore_ascii_case("[newtour]")).collect();
        let Some(&first) = starts.first() else { continue };
        let mut kept: Vec<Vec<String>> = Vec::new();
        let mut changed = false;
        for (n, &s) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(rows.len());
            let mut block: Vec<String> = rows[s..end].iter().map(|r| r.to_string()).collect();
            let tour = block.get(1).map(|x| x.trim().to_string()).unwrap_or_default();
            let group = block.get(2).map(|x| x.trim().to_lowercase()).unwrap_or_default();
            if !bus_groups.contains(&group) {
                kept.push(block);
                continue;
            }
            changed = true;
            match group_of.get(&key(&line, &tour)) {
                Some(g) if block.len() > 2 => {
                    block[2] = g.clone();
                    kept.push(block);
                    folder.tours += 1;
                }
                _ => folder.taken_out += 1,
            }
        }
        if !changed {
            continue;
        }
        if kept.is_empty() {
            folder.off.push(line);
            continue;
        }
        let mut text: Vec<String> = rows[..first].iter().map(|r| r.to_string()).collect();
        text.extend(kept.into_iter().flatten());
        folder.files.push((format!("TTData/{}", lf.file), text.join(NL)));
    }
    let mut cfg: Vec<String> = ["", "[name]", "Omsi-Hub: the company's own buses", "", "[startdate]", "19000101", "", "[enddate]", "29991231", ""].iter().map(|s| s.to_string()).collect();
    if !folder.off.is_empty() {
        cfg.push("[deactivate_lines]".into());
        cfg.push(folder.off.len().to_string());
        cfg.extend(folder.off.iter().cloned());
        cfg.push(String::new());
    }
    cfg.push(String::new());
    folder.files.push(("Chrono.cfg".into(), cfg.join(NL)));
    folder.files.push(("ailists_#upd.cfg".into(), groups));
    folder
}

// --- on disk ------------------------------------------------------------------------------------

fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn to_latin1(text: &str) -> Vec<u8> {
    text.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
}

/// The map's folder (`maps/<name>`) of its `global.cfg` as the company names it.
fn map_rel(map: &str) -> Option<String> {
    let m = map.replace('\\', "/");
    m.rsplit_once('/').map(|(d, _)| d.to_string()).filter(|d| !d.is_empty())
}

/// The folder of the company's map in openOMSI's content folder `content`.
pub fn folder_of(content: &Path, map: &str) -> Option<PathBuf> {
    Some(crate::paths::resolve_path(content, &map_rel(map)?).join("Chrono").join(FOLDER))
}

/// The lines of the map's timetable as the game reads them on `date`: the `.ttl` files of every
/// copy of its `TTData` (the content folder's first), then of each active Chrono folder in turn.
fn line_files(vfs: &hub_omsi::Vfs, dir: &str, chrono: &[hub_omsi::map::ChronoDir]) -> Vec<LineFile> {
    let mut by_name: BTreeMap<String, LineFile> = BTreeMap::new();
    let mut take = |p: &Path, name: &str, over: bool| {
        if !name.to_lowercase().ends_with(".ttl") {
            return;
        }
        let k = name.to_lowercase();
        if !over && by_name.contains_key(&k) {
            return;
        }
        if let Ok(b) = std::fs::read(p) {
            by_name.insert(k, LineFile { file: name.to_string(), text: latin1(&b) });
        }
    };
    for (name, is_dir) in vfs.list_dir(&format!("{dir}/TTData")) {
        if let (false, Some(p)) = (is_dir, vfs.resolve(&format!("{dir}/TTData/{name}"))) {
            take(&p, &name, false);
        }
    }
    for c in chrono {
        let Some(tt) = hub_omsi::vfs::find_child(&c.path, "TTData") else { continue };
        for e in std::fs::read_dir(&tt).into_iter().flatten().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            take(&e.path(), &name, true);
        }
    }
    by_name.into_values().collect()
}

/// The folder written for the company `c`'s duty on `date` (`tours`, that day's plan) into
/// openOMSI's content folder `content`: the game runs only the company's buses on its lines.
pub fn write(c: &Company, tours: &[TourPlan], date: &str, vfs: &hub_omsi::Vfs, content: &Path) -> Result<Folder, String> {
    let day = chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").map_err(|e| format!("date {date}: {e}"))?;
    let dir = map_rel(&c.map).ok_or_else(|| format!("no map folder in {}", c.map))?;
    let target = folder_of(content, &c.map).ok_or("no map folder")?;
    let _ = std::fs::remove_dir_all(&target);
    // (the map's AI lists and its Chrono folders as the game reads them, every copy of the map)
    let copies: Vec<PathBuf> = vfs.bases().iter().filter_map(|b| hub_omsi::vfs::resolve_in(b, &dir)).filter(|d| d.is_dir()).collect();
    let chrono = hub_omsi::map::active_chrono_dirs_in(&copies, day);
    let map_dir = vfs.resolve(&format!("{dir}/ailists.cfg")).and_then(|p| p.parent().map(Path::to_path_buf)).ok_or("the map has no ailists.cfg")?;
    let ai = AiLists::with_chrono(&map_dir, &chrono);
    let folder = build(c, tours, &ai, &line_files(vfs, &dir, &chrono));
    for (rel, text) in &folder.files {
        let p = target.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&p, to_latin1(text)).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    Ok(folder)
}

/// Every such folder in the content folder taken away (the game ended, a drive of the
/// player's own, Omsi-Hub started after a game that did not end well): how many.
pub fn remove_all(content: &Path) -> usize {
    let Some(maps) = hub_omsi::vfs::find_child(content, "maps") else { return 0 };
    let mut n = 0;
    for e in std::fs::read_dir(maps).into_iter().flatten().flatten() {
        let Some(chrono) = hub_omsi::vfs::find_child(&e.path(), "Chrono") else { continue };
        let Some(dir) = hub_omsi::vfs::find_child(&chrono, FOLDER) else { continue };
        if std::fs::remove_dir_all(&dir).is_ok() {
            n += 1;
        }
        // (a Chrono folder that only held this one goes with it: openOMSI's copy as it was)
        let _ = std::fs::remove_dir(&chrono);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::company::day::{DutyPlan, TourPlan};
    use crate::company::network::{PlannedTrip, TourOfDay};
    use crate::company::CompanyLine;

    fn ttl(tours: &[(&str, &str)]) -> String {
        let mut t = vec!["Time Table Line File".to_string(), String::new(), "[userallowed]".into(), String::new()];
        for (n, g) in tours {
            t.extend(["[newtour]".to_string(), n.to_string(), g.to_string(), "31".into(), String::new(), "[addtrip]".into(), format!("trip {n}"), "0".into(), "360.0".into(), String::new()]);
        }
        t.join(NL)
    }

    fn ailists() -> AiLists {
        let text = "[aigroup_2]\r\nCars\r\n\r\nVehicles\\Car\\car.ovh\t1\r\n[end]\r\n\r\n[aigroup_depot]\r\nStadtbus\r\nCity Depot\r\n\r\n[aigroup_depot_typgroup_2]\r\nVehicles\\Citaro\\Citaro.bus\r\n1\tCB 1\tCity\r\n[end]\r\n\r\n[aigroup_depot]\r\nLieferdrohne\r\nCity Depot\r\n\r\n[aigroup_depot_typgroup_2]\r\nVehicles\\Robot\\DeliveryRobot.bus\r\n1\tDP 1\tPost\r\n[end]\r\n";
        AiLists::parse(&hub_omsi::text::CfgFile::from_str("ailists.cfg", text))
    }

    fn tour(line: &str, tour: &str, trips: Vec<PlannedTrip>, bus: Option<u32>) -> TourPlan {
        TourPlan { tour: TourOfDay { line: line.into(), tour: tour.into(), trips, ..Default::default() }, bus, duties: vec![DutyPlan::default()], ..Default::default() }
    }

    fn trip(origin: &str) -> PlannedTrip {
        PlannedTrip { name: "t".into(), origin: origin.into(), ..Default::default() }
    }

    fn company() -> Company {
        use crate::company::market::{self, MarketBus, Payment};
        let mut c = crate::company::found(&crate::company::Founding { name: "APV".into(), map: "maps/Town/global.cfg".into(), date: "2026-11-12".into(), ..Default::default() }, "Luc");
        c.depot = "Our Depot".into();
        c.cash = 100_000_000_000;
        for _ in 0..2 {
            market::buy_new(&mut c, &MarketBus { file: "Vehicles/MAN/A20.bus".into(), name: "A20".into(), ..Default::default() }, Payment::Cash, "APV Gruen").unwrap();
        }
        for (v, n) in c.fleet.iter_mut().zip(["101", "102"]) {
            (v.number, v.plate) = (n.into(), format!("APV {n}"));
        }
        c.lines.push(CompanyLine { name: "oo_77".into(), number: "77".into(), own: true, ..Default::default() });
        c
    }

    fn file<'a>(f: &'a Folder, name: &str) -> &'a str {
        f.files.iter().find(|x| x.0 == name).map(|x| x.1.as_str()).unwrap_or_else(|| panic!("no {name}"))
    }

    /// The company's tours keep a bus of their own - the planned one, with its number, plate
    /// and livery - and every other bus tour is out of the game: a line the company does not
    /// run at all switched off, one it runs part of cut down; other traffic stays.
    #[test]
    fn only_the_company_s_buses_run_its_own_tours() {
        let c = company();
        let plan = vec![
            tour("oo_77", "1", vec![trip("")], Some(1)),
            // (a block of the map's line joining trips of its tours 3 and 4)
            tour("Linie5", "B1", vec![trip("3"), trip("4")], Some(2)),
            // (dropped today: no bus)
            tour("oo_77", "2", vec![trip("")], None),
        ];
        let lines = vec![
            LineFile { file: "oo_77.ttl".into(), text: ttl(&[("1", "Stadtbus"), ("2", "Stadtbus")]) },
            LineFile { file: "Linie5.ttl".into(), text: ttl(&[("3", "Stadtbus"), ("4", "Stadtbus"), ("5", "Stadtbus")]) },
            LineFile { file: "Linie9.ttl".into(), text: ttl(&[("1", "Stadtbus")]) },
            LineFile { file: "oo_12.ttl".into(), text: ttl(&[("1", "Stadtbus")]) },
            LineFile { file: "Post.ttl".into(), text: ttl(&[("1", "Lieferdrohne")]) },
            LineFile { file: "Bahn.ttl".into(), text: ttl(&[("ICE", "Fernverkehr")]) },
        ];
        let f = build(&c, &plan, &ailists(), &lines);
        assert_eq!((f.tours, f.taken_out), (3, 4));
        assert_eq!(f.off, ["Linie9", "oo_12"]);
        let groups = file(&f, "ailists_#upd.cfg");
        assert!(groups.contains("[aigroup_depot]\r\nOH_oo_77_1\r\nOur Depot\r\n\r\n[aigroup_depot_typgroup_2]\r\nVehicles\\MAN\\A20.bus\r\n101\tAPV 101\tAPV Gruen\r\n[end]"), "{groups}");
        assert!(groups.contains("OH_Linie5_3") && groups.contains("102\tAPV 102\tAPV Gruen"));
        let own = file(&f, "TTData/oo_77.ttl");
        assert!(own.contains("[newtour]\r\n1\r\nOH_oo_77_1") && !own.contains("[newtour]\r\n2\r\n"), "{own}");
        let map_line = file(&f, "TTData/Linie5.ttl");
        assert!(map_line.starts_with("Time Table Line File") && map_line.contains("OH_Linie5_3") && map_line.contains("OH_Linie5_4") && !map_line.contains("[newtour]\r\n5\r\n"));
        assert!(f.files.iter().all(|x| x.0 != "TTData/Post.ttl" && x.0 != "TTData/Bahn.ttl"), "other traffic left as it is");
        assert!(file(&f, "Chrono.cfg").contains("[deactivate_lines]\r\n2\r\nLinie9\r\noo_12\r\n"));
    }

    /// The folder lies in the content folder's copy of the map, and goes with the game.
    #[test]
    fn the_folder_is_in_the_content_folder_and_goes_again() {
        let content = std::env::temp_dir().join(format!("hub-core-ailist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&content);
        let f = folder_of(&content, "maps/Town/global.cfg").unwrap();
        assert!(f.ends_with(Path::new("maps").join("Town").join("Chrono").join(FOLDER)));
        std::fs::create_dir_all(f.join("TTData")).unwrap();
        std::fs::create_dir_all(content.join("maps").join("Town").join("Chrono").join("Fest")).unwrap();
        assert_eq!(remove_all(&content), 1);
        assert!(!f.exists() && content.join("maps").join("Town").join("Chrono").join("Fest").is_dir(), "the map's own Chrono stays");
        // (a Chrono folder that held only the company's goes too)
        let alone = folder_of(&content, "maps/Dorf/global.cfg").unwrap();
        std::fs::create_dir_all(&alone).unwrap();
        assert_eq!(remove_all(&content), 1);
        assert!(!content.join("maps").join("Dorf").join("Chrono").exists());
        let _ = std::fs::remove_dir_all(&content);
    }
}
