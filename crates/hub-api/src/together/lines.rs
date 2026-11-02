//! A company's own line reaches the player who drives it: the boss's launcher sends the line with
//! the duty's start (`own_line`), and the player's writes it into his registry, his timetable and
//! his depot files as the line editor does, before his game starts. His own lines stay his
//! (`lines::take_shared`).

use crate::company::words::tr;
use crate::launcher as core;
use crate::launcher::company::together::wire::DriveStart;
use hub_core::lines::{self as reg, LineDesign};
use hub_core::{linehof, owndepot};
use serde_json::{json, Value};
use std::path::Path;

/// A line larger than this stays home: a message of the session is at most 1 MB.
const MAX_LINE: usize = 512 << 10;

/// The answer to a player's start gets the line it asks, when it is one of the boss's own.
pub(super) fn attach(body: &mut Value) {
    let Some((map, line)) = body.get("start").map(|s| (s["map"].as_str().unwrap_or("").to_string(), s["line"].as_str().unwrap_or("").to_string())) else { return };
    let path = reg::registry_path(&reg::map_folder(&map));
    let Some(design) = reg::shared_line(&reg::load_registry(&path), &line) else { return };
    let size = serde_json::to_string(&design).map_or(usize::MAX, |t| t.len());
    if size > MAX_LINE {
        core::log_to_file(&format!("company session: line {line} not sent, {size} bytes"));
        return;
    }
    body["own_line"] = json!(design);
}

/// The map's path as the boss's launcher sent it: within the game's folders.
fn map_is_plain(map: &str) -> bool {
    !map.contains("..") && !map.contains(':') && !map.starts_with(['/', '\\'])
}

/// The boss's line written on this computer for the game about to start.
pub(super) fn install(start: &DriveStart, line: LineDesign) -> Result<(), String> {
    if !map_is_plain(&start.map) {
        return Err(tr(reg::SHARED_NOT_SAFE));
    }
    let content = core::content_dir().ok_or_else(|| "no content folder".to_string())?;
    let root = core::root_dir().unwrap_or_default();
    let global = core::resolve_content(&start.map).unwrap_or_else(|| hub_core::paths::resolve_path(&root, &start.map));
    if !global.is_file() {
        return Err(tr("The duty's map is not installed on this computer."));
    }
    let map_dir = global.parent().unwrap_or(Path::new("")).to_path_buf();
    let path = reg::registry_path(&reg::map_folder(&start.map));
    let mut r = reg::load_registry(&path);
    let name = line.number.clone();
    reg::take_shared(&mut r, &start.company, line).map_err(tr)?;
    if r.map.is_empty() {
        r.map = reg::map_folder(&start.map);
        r.global = start.map.clone();
    }
    // (as the line editor saves: the codes first, then the registry, the timetable, the depot files)
    let groups = linehof::group_depots(&map_dir);
    let (bases, original) = core::depot_roots();
    let plan = linehof::prepare(&mut r, &groups, &bases);
    owndepot::assign(&mut r, &owndepot::dir(), &groups, &bases);
    reg::save_registry(&path, &r)?;
    reg::export_to_map(&content, &map_dir, &r)?;
    let (_, err) = linehof::write(&content, original.as_deref(), &r, &groups, &plan);
    let (_, own_err) = owndepot::write_lines(&r, &owndepot::dir(), Some(&content));
    if let Some(e) = err.or(own_err) {
        core::log_to_file(&format!("company session: the depot files of line {name}: {e}"));
    }
    core::log_to_file(&format!("company session: line {name} of the boss's written for this duty"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_maps_path_stays_within_the_games_folders() {
        assert!(map_is_plain("maps/Grundorf/global.cfg"));
        assert!(map_is_plain("maps\\Grundorf\\global.cfg"));
        for bad in ["../../x/global.cfg", "/etc/global.cfg", "\\\\host\\global.cfg", "C:\\x\\global.cfg"] {
            assert!(!map_is_plain(bad), "{bad}");
        }
    }
}
