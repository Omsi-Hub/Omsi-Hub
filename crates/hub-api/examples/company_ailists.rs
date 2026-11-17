//! Prototype: a company's own AI list for the game (Omsi-Hub, "only the company's buses").
//!
//!   cargo run -p hub-api --example company_ailists -- <company id> <date> <map layer folder> [off]
//!
//! For `date` it takes the company's day plan (which bus runs which tour) and its fleet (fleet
//! number, plate, livery) and writes a Chrono folder into openOMSI's layer of the map
//! (`<map layer>/Chrono/zz_omsihub_company/`), which openOMSI applies after the map's own:
//!
//! * `ailists_#upd.cfg`: a depot group for every tour the company runs (`OH_<line>_<tour>`)
//!   holding exactly the bus the plan gives it - its `.bus`, fleet number, plate and livery -
//!   one group `OH_<short>` with the whole fleet (a tour with no bus of its own), and
//!   `OH_Leeg`, a depot group with no bus at all;
//! * `TTData/<line>.ttl`: every timetable file whose tours use a bus depot group, with each
//!   company tour on its own group and every other bus tour on `OH_Leeg` - openOMSI runs no
//!   bus for those ("no vehicles for AI group"). Trains, planes, ships keep their groups.
//!
//! `off` removes the folder again. The map's own files and the OMSI 2 folder are only read.

use hub_core::company as co;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FOLDER: &str = "zz_omsihub_company";
const EMPTY: &str = "OH_Leeg";
const NL: &str = "\r\n";

fn latin1(p: &Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| b.iter().map(|&c| c as char).collect())
}

fn write_latin1(p: &Path, text: &str) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let bytes: Vec<u8> = text.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect();
    std::fs::write(p, bytes).unwrap();
}

/// A name OMSI keeps on one line, without tabs.
fn clean(s: &str) -> String {
    s.replace(['\t', '\r', '\n'], " ").trim().to_string()
}

/// The vehicle file as an ailists.cfg names it (backslashes, relative to the OMSI 2 folder).
fn omsi_path(file: &str) -> String {
    file.replace('/', "\\")
}

/// A depot group of the given buses: one typgroup per bus file.
fn depot_group(name: &str, hof: &str, buses: &[&co::Vehicle]) -> String {
    let mut by_file: BTreeMap<String, Vec<&co::Vehicle>> = BTreeMap::new();
    for v in buses {
        by_file.entry(v.bus.clone()).or_default().push(v);
    }
    let mut out = vec![String::new(), "[aigroup_depot]".into(), clean(name), clean(hof), String::new()];
    for (file, vs) in by_file {
        out.push("[aigroup_depot_typgroup_2]".into());
        out.push(omsi_path(&file));
        for v in vs {
            let paint = v.house_livery.clone().unwrap_or_else(|| v.livery.clone());
            out.push(format!("{}\t{}\t{}", clean(&v.number), clean(&v.plate), clean(&paint)));
        }
        out.push("[end]".into());
        out.push(String::new());
    }
    out.join(NL)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (id, date, layer) = (&args[1], &args[2], PathBuf::from(&args[3]));
    let target = layer.join("Chrono").join(FOLDER);
    if std::fs::metadata(&target).is_ok() {
        std::fs::remove_dir_all(&target).expect("the old folder");
    }
    if args.get(4).map(String::as_str) == Some("off") {
        println!("removed {}", target.display());
        return;
    }

    let data = hub_core::paths::data_dir();
    let c = co::store::load(&data, id).expect("the company");
    let root = PathBuf::from(hub_core::settings::Settings::load().omsi2_folder);
    let map_rel = Path::new(&c.map).parent().expect("the map's folder").to_path_buf();
    let map_dir = root.join(&map_rel);

    // (the day as the company runs it: which bus on which tour)
    let lines = hub_core::list_lines(&c.map, date).expect("the day's timetable");
    let tours = co::network::tours_of_day(&c, &lines, date);
    let plan = co::plan::day_plan(&c, date, tours, &[], &[], false);

    // (the map's AI lists: its depot groups, and which of them carry buses)
    let ai = hub_omsi::map::AiLists::load(&map_dir.join("ailists.cfg")).expect("the map's ailists.cfg");
    let not_bus = |g: &hub_omsi::map::AiGroup| {
        let n = g.name.to_lowercase();
        n.contains("drohne") || n.contains("liefer") || g.typgroups.iter().all(|t| t.file.to_lowercase().contains("robot"))
    };
    let bus_groups: Vec<String> = ai.groups.iter().filter(|g| g.is_depot && !not_bus(g)).map(|g| g.name.to_lowercase()).collect();

    // (the groups: one per planned tour with its bus, the whole fleet, the empty one)
    let fleet: Vec<&co::Vehicle> = c.fleet.iter().collect();
    let whole = format!("OH_{}", clean(&c.short));
    let mut text = String::new();
    text.push_str(&depot_group(&whole, &c.depot, &fleet));
    let mut tour_group: BTreeMap<(String, String), String> = BTreeMap::new();
    for t in &plan.tours {
        let key = (t.tour.line.to_lowercase(), t.tour.tour.to_lowercase());
        let bus = match t.bus {
            Some(co::plan::BusOf::Own(b)) => c.vehicle(b),
            _ => None,
        };
        let name = match bus {
            Some(v) => {
                let g = format!("OH_{}_{}", clean(&t.tour.line), clean(&t.tour.tour)).replace(' ', "_");
                text.push_str(&depot_group(&g, &c.depot, &[v]));
                println!("tour {} / {}: bus {} ({}, {})", t.tour.line, t.tour.tour, v.number, v.plate, v.house_livery.clone().unwrap_or(v.livery.clone()));
                g
            }
            None => {
                println!("tour {} / {}: no bus of its own -> {whole}", t.tour.line, t.tour.tour);
                whole.clone()
            }
        };
        tour_group.insert(key, name);
    }
    text.push_str(&[String::new(), "[aigroup_depot]".into(), EMPTY.into(), clean(&c.depot), String::new(), String::new()].join(NL));
    write_latin1(&target.join("ailists_#upd.cfg"), &text);
    write_latin1(&target.join("Chrono.cfg"), &["", "[name]", "Omsi-Hub company", "", "[startdate]", "19000101", "", "[enddate]", "29991231", "", ""].join(NL));

    // (every timetable file the game reads: the map's, the layer's copy winning)
    let mut files: BTreeMap<String, PathBuf> = BTreeMap::new();
    for dir in [map_dir.join("TTData"), layer.join("TTData")] {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ttl")) {
                files.insert(p.file_name().unwrap().to_string_lossy().to_lowercase(), p);
            }
        }
    }
    // (a bus tour not the company's is taken out of the timetable altogether - pointed at an
    // empty group the game still runs it on its map without a vehicle; a line left with no tour
    // at all is switched off by the Chrono folder, as the map's own Chrono folders do)
    let (mut ours, mut gone) = (0, 0);
    let mut off: Vec<String> = Vec::new();
    for p in files.values() {
        let Some(src) = latin1(p) else { continue };
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let line = stem.to_lowercase();
        let rows: Vec<String> = src.split(NL).map(str::to_string).collect();
        // (the file cut into its head and its tours, each from its [newtour] to the next)
        let starts: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].trim().eq_ignore_ascii_case("[newtour]")).collect();
        if starts.is_empty() {
            continue;
        }
        let mut head: Vec<String> = rows[..starts[0]].to_vec();
        let mut kept: Vec<Vec<String>> = Vec::new();
        let mut changed = false;
        for (n, &s) in starts.iter().enumerate() {
            let end = starts.get(n + 1).copied().unwrap_or(rows.len());
            let mut block = rows[s..end].to_vec();
            let tour = block.get(1).map(|x| x.trim().to_lowercase()).unwrap_or_default();
            let group = block.get(2).map(|x| x.trim().to_lowercase()).unwrap_or_default();
            if !bus_groups.contains(&group) {
                kept.push(block);
                continue;
            }
            changed = true;
            match tour_group.get(&(line.clone(), tour)) {
                Some(g) => {
                    ours += 1;
                    block[2] = g.clone();
                    kept.push(block);
                }
                None => gone += 1,
            }
        }
        if !changed {
            continue;
        }
        if kept.is_empty() {
            off.push(stem);
            continue;
        }
        for b in kept {
            head.extend(b);
        }
        write_latin1(&target.join("TTData").join(p.file_name().unwrap()), &head.join(NL));
    }
    let mut cfg: Vec<String> = ["", "[name]", "Omsi-Hub company", "", "[startdate]", "19000101", "", "[enddate]", "29991231", ""].iter().map(|s| s.to_string()).collect();
    if !off.is_empty() {
        cfg.push("[deactivate_lines]".into());
        cfg.push(off.len().to_string());
        cfg.extend(off.iter().cloned());
        cfg.push(String::new());
    }
    cfg.push(String::new());
    write_latin1(&target.join("Chrono.cfg"), &cfg.join(NL));
    println!("{ours} company tours on their own bus, {gone} other bus tours taken out, {} lines switched off: {:?}", off.len(), off);
    println!("written to {}", target.display());
}
