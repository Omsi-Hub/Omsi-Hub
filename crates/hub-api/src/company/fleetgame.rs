//! The company's buses in the game (Luc, 2026-10-26: "the player starts a company, and when he
//! drives himself he sees only his own buses"; 2026-11-17: "only the company's buses and lines,
//! not the map's default").
//!
//! Before the game of a company's duty starts, Omsi-Hub writes the company's AI list into
//! openOMSI's copy of the map (`hub_core::company::ailist`): each tour the day's plan runs with a
//! bus of the company gets that bus - its fleet number, plate and livery - and every other bus
//! line of the timetable is out of the game, the map's own and other companies' alike. The
//! folder goes again when the game ends ([`game_over`]), with a drive of the player's own, and
//! when Omsi-Hub starts after a game that did not end well ([`startup`]).
//!
//! The plugin's list of trips (`data/fleet.json`, `lua/fleet.lua`) is no longer written: the
//! game has no `omsi.timetable.set_ai_trips`, and the plugin's fallback took the company's own
//! buses off the road (it knows a tour by the line's number, the list by its timetable name). A
//! list left by an older Omsi-Hub is taken away.
//!
//! Nothing is booked here: the folder changes nothing of the company.

use super::Session;
use crate::launcher as core;
use hub_core::company::ailist;
use hub_core::company::day::TourPlan;
use hub_game::bridge;

/// The day's tours of the company for the game on `date`: today's as the company's clock has
/// them, a later day's as planned.
fn tours_of(s: &mut Session, date: &str) -> Option<Vec<TourPlan>> {
    let c = s.company.as_ref()?;
    if c.date == date {
        if let Some(p) = s.plan.as_ref() {
            return Some(p.tours.clone());
        }
    }
    match super::plan_of(s, date)? {
        Ok(dp) => Some(dp.to_plan().tours),
        Err(_) => None,
    }
}

/// The company's AI list for the game of a company's duty about to start (`date` the duty's
/// day, `skip` its block - its timetable line and tour - the player drives; `keep`: the drive
/// takes that block's bus over from the game's AI, which drives it until then).
pub fn for_drive(s: &mut Session, date: &str, skip: (&str, &str), keep: bool) {
    if let Some(dir) = crate::addon::game_plugin_dir() {
        let _ = bridge::clear_fleet(&dir);
    }
    let Some(content) = hub_core::paths::content_dir() else { return };
    let Some(c) = s.company.clone() else { return };
    let (Some(mut tours), Some(vfs)) = (tours_of(s, date), hub_core::paths::vfs()) else {
        core::log_to_file("company: the company's AI list for the game: the day's plan or the game's folders are not there; the game runs the map's timetable");
        let _ = ailist::remove_all(&content);
        return;
    };
    // (the player's duty - its timetable line and tour - stays in the game's timetable however
    // it is planned: it is the tour the game starts; the AI keeps off it, unless the player takes
    // its bus over, `keep`, and then it runs the bus planned)
    let key = |l: &str, t: &str| (l.trim().to_lowercase(), t.trim().to_lowercase());
    let planned = ailist::planned_buses(&tours);
    if !planned.contains_key(&key(skip.0, skip.1)) {
        let bus = planned.values().next().and_then(|p| p.2.iter().next().copied()).or_else(|| c.fleet.first().map(|v| v.id));
        let tour = hub_core::company::network::TourOfDay { line: skip.0.to_string(), tour: skip.1.to_string(), ..Default::default() };
        tours.push(TourPlan { tour, bus, ..Default::default() });
    }
    let _ = keep;
    match ailist::write(&c, &tours, date, &vfs, &content) {
        Ok(f) => core::log_to_file(&format!(
            "company: the company's AI list for the game: {} of its tours on its own buses, {} other bus tours out, {} lines off ({})",
            f.tours,
            f.taken_out,
            f.off.len(),
            ailist::folder_of(&content, &c.map).map(|p| p.display().to_string()).unwrap_or_default()
        )),
        Err(e) => {
            core::log_to_file(&format!("company: the company's AI list for the game: {e}"));
            let _ = ailist::remove_all(&content);
        }
    }
}

/// No AI list of a company in the game's folders (the game ended, or a drive of the player's
/// own): the game runs the map's whole timetable again.
pub fn game_over() {
    let Some(content) = hub_core::paths::content_dir() else { return };
    let n = ailist::remove_all(&content);
    if n > 0 {
        core::log_to_file(&format!("company: the company's AI list taken out of the game ({n})"));
    }
}

/// Omsi-Hub started: an AI list left by a game that did not end well taken away - unless a game
/// runs (Omsi-Hub restarted during a company's duty).
pub fn startup() {
    if core::list_instances().iter().any(|i| i.running) {
        return;
    }
    game_over();
}
