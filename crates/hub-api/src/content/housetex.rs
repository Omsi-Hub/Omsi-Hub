//! The houses of the companion's 3D navigator as the game dresses them: their own models with
//! their textures. openOMSI's picture of the map (`hub_companion::ground`) keeps each house's
//! model untextured and without the object it came from; this finds that object again - the
//! map's tiles placed as the game's map generator places them (openOMSI's
//! `omsi-scene/src/mapgen.rs`), an instance known by its object, its place and its heading - and
//! reads the object's model as the generator read it, now with its texture coordinates and the
//! textures it names.

use glam::{DVec2, Vec2, Vec3};
use hashbrown::HashMap;
use hub_companion::dress::{Dressed, Face, Look, SKIN};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The faces a house's model keeps at most, the biggest first (the generator's own).
pub const FACES: usize = 4000;

/// An object of the map placed as the generator places it: its file's key (the path in lower
/// case, `/`), its file, where (relative to the layers' origin) and its compass heading.
#[derive(Clone, Debug)]
pub struct Placed {
    pub key: String,
    pub file: String,
    pub at: [f32; 2],
    pub heading: f32,
}

fn key_of(file: &str) -> String {
    file.trim().replace('\\', "/").to_ascii_lowercase()
}

/// The map's tiles as the generator read them: where they lie, the folder its objects are named
/// from, the chrono folders of the day.
pub struct MapTiles {
    pub root: PathBuf,
    pub origin: DVec2,
    size: f64,
    tiles: HashMap<(i32, i32), PathBuf>,
    chrono: Vec<PathBuf>,
    /// The objects of the tiles read so far.
    placed: HashMap<(i32, i32), Vec<Placed>>,
}

impl MapTiles {
    /// The map of `global` (its `global.cfg`) on `date` (YYYY-MM-DD, "" for none).
    pub fn open(global: &Path, date: &str) -> Option<MapTiles> {
        let cfg = omsi_map::GlobalCfg::load(global).ok()?;
        omsi_map::configure_grid(&cfg);
        let map_dir = global.parent()?.to_path_buf();
        let chrono = omsi_map::date_code(date).map(|c| omsi_map::active_chrono_dirs(&map_dir, c)).unwrap_or_default();
        let root =
            omsi_cfg::content_roots().into_iter().find(|r| map_dir.starts_with(r)).or_else(|| map_dir.parent().and_then(|p| p.parent()).map(Path::to_path_buf)).unwrap_or_else(|| map_dir.clone());
        let size = omsi_map::tile_size();
        let list: Vec<(i32, i32, PathBuf)> = cfg.tiles.iter().map(|t| (t.x, t.y, omsi_cfg::resolve_path(&map_dir, &t.file))).filter(|t| omsi_cfg::vfs::is_file(&t.2)).collect();
        // (the layers count from the first tile's corner, as the generator does)
        let origin = list.first().map(|t| DVec2::new(t.0 as f64 * size, t.1 as f64 * size)).unwrap_or(DVec2::ZERO);
        let tiles = list.into_iter().map(|(x, y, p)| ((x, y), p)).collect();
        Some(MapTiles { root, origin, size, tiles, chrono, placed: HashMap::new() })
    }

    /// The side of a tile (m).
    pub fn tile_size(&self) -> f64 {
        self.size
    }

    /// Tile `t` as the game reads it on the day (its chrono changes), if the map has it.
    pub fn tile(&self, t: (i32, i32)) -> Option<omsi_map::Tile> {
        self.tiles.get(&t).and_then(|p| omsi_world::map_index::read_tile(p, &self.chrono))
    }

    /// The tile a place (relative to the origin) lies in.
    fn tile_at(&self, at: [f32; 2]) -> (i32, i32) {
        let w = self.origin + DVec2::new(at[0] as f64, at[1] as f64);
        ((w.x / self.size).floor() as i32, (w.y / self.size).floor() as i32)
    }

    /// The objects of tile `t`, read once: its own and the rows along its splines.
    pub fn objects(&mut self, t: (i32, i32)) -> &[Placed] {
        if !self.placed.contains_key(&t) {
            let mut out = Vec::new();
            if let Some(tile) = self.tiles.get(&t).and_then(|p| omsi_world::map_index::read_tile(p, &self.chrono)) {
                let o2 = DVec2::new(t.0 as f64 * self.size, t.1 as f64 * self.size);
                let mut put = |file: &str, at: DVec2, heading: f64| {
                    let p = (at - self.origin).as_vec2();
                    out.push(Placed { key: key_of(file), file: file.to_string(), at: [p.x, p.y], heading: heading as f32 });
                };
                for o in tile.objects.iter().filter(|o| !o.file.trim().is_empty()) {
                    put(&o.file, o2 + DVec2::new(o.pos[0], o.pos[1]), o.rot[0]);
                }
                for att in &tile.spline_attachments {
                    for (_, ro) in omsi_world::map_index::tile_row_objects(att, &tile.splines, o2, None) {
                        put(&att.file, ro.pose.pos.truncate(), ro.pose.heading());
                    }
                }
            }
            self.placed.insert(t, out);
        }
        &self.placed[&t]
    }

    /// The object standing at `at` with `heading`, as an instance of the layers has them (the
    /// generator keeps a place to a twentieth of a metre and a heading to a tenth of a degree).
    pub fn object_at(&mut self, at: [f32; 2], heading: f32) -> Option<Placed> {
        let t = self.tile_at(at);
        // (a row's object may stand a little over its tile's edge)
        let near = [t, (t.0 - 1, t.1), (t.0 + 1, t.1), (t.0, t.1 - 1), (t.0, t.1 + 1)];
        for t in near {
            let hit = self.objects(t).iter().find(|p| (p.at[0] - at[0]).abs() < 0.06 && (p.at[1] - at[1]).abs() < 0.06 && ((p.heading - heading + 540.0).rem_euclid(360.0) - 180.0).abs() < 0.11);
            if let Some(p) = hit {
                return Some(p.clone());
            }
        }
        None
    }
}

/// The textures the houses wear, by their id (`Look::skin`): its file and whether its clear parts
/// are holes; the phone never sees a path. An id is the texture's own - made from its file and
/// its cut, not from the order squares were asked for in - and the ids given out are kept in
/// [`SKINS_FILE`]: a phone keeps its squares from one run of the add-on to the next, and the
/// textures they name are still the same ones.
static SKINS: Mutex<Option<HashMap<u32, (PathBuf, bool)>>> = Mutex::new(None);
/// A texture as the phone gets it: its bytes and their type.
pub type Skin = Arc<(Vec<u8>, &'static str)>;
/// The textures as the phone gets them, made once each.
static MADE: Mutex<Option<HashMap<u32, Skin>>> = Mutex::new(None);
/// The ids given out, a line each (`id`, `1` where cut, the file; tab apart), in Omsi-Hub's data
/// folder.
pub const SKINS_FILE: &str = "companion-skins.tsv";

/// A texture's own id: FNV-1a of its file (in lower case, `/`) and its cut, never 0.
fn own_id(path: &Path, cut: bool) -> u32 {
    let key = format!("{}|{}", path.to_string_lossy().replace('\\', "/").to_lowercase(), cut as u8);
    let h = key.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193));
    (h & 0x7fff_ffff).max(1)
}

/// The ids given out so far, read from [`SKINS_FILE`] the first time.
fn skins() -> std::sync::MutexGuard<'static, Option<HashMap<u32, (PathBuf, bool)>>> {
    let mut all = SKINS.lock().unwrap_or_else(|e| e.into_inner());
    if all.is_none() {
        let text = std::fs::read_to_string(hub_core::paths::data_dir().join(SKINS_FILE)).unwrap_or_default();
        *all = Some(read_skins(&text));
    }
    all
}

fn read_skins(text: &str) -> HashMap<u32, (PathBuf, bool)> {
    text.lines()
        .filter_map(|l| {
            let mut f = l.splitn(3, '\t');
            Some((f.next()?.parse().ok()?, f.next()? == "1", PathBuf::from(f.next()?)))
        })
        .map(|(id, cut, p)| (id, (p, cut)))
        .collect()
}

pub(crate) fn skin_id(path: &Path, cut: bool) -> u32 {
    let mut guard = skins();
    let all = guard.get_or_insert_with(HashMap::new);
    // (two files of one id - all but never - the next free one)
    let mut id = own_id(path, cut);
    loop {
        match all.get(&id) {
            Some((p, c)) if p == path && *c == cut => return id,
            Some(_) => id = id.wrapping_add(1).max(1),
            None => break,
        }
    }
    all.insert(id, (path.to_path_buf(), cut));
    let line = format!("{id}\t{}\t{}\n", cut as u8, path.display());
    let file = hub_core::paths::data_dir().join(SKINS_FILE);
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(file).and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
    id
}

/// An object's model as the game shows it: every face of its most detailed level (x, y, z up from
/// the object's place), with its texture coordinates and which of `looks` it wears.
pub struct Worn {
    pub tris: Vec<([Vec3; 3], [Vec2; 3], u16)>,
    pub looks: Vec<Look>,
    /// A piece of road: an object with lanes of its own (a crossing, a bus bay) or a surface.
    pub road: bool,
}

/// The id of texture `id`'s picture without its alpha (drawn whole).
fn opaque(id: u32) -> u32 {
    let path = skins().as_ref().and_then(|all| all.get(&id).map(|(p, _)| p.clone()));
    path.map_or(id, |p| skin_id(&p, false))
}

/// The model of the object `file` (named from `root`) as the generator reads a house's (the most
/// detailed level's meshes, the faces above its ground, the biggest [`FACES`]: the same faces as
/// the game's picture keeps), dressed: each face's texture coordinates and look.
pub fn dressed(root: &Path, file: &str) -> Option<Dressed> {
    let Worn { tris, looks, .. } = worn(root, file)?;
    // (a house's glass drawn whole: its texture without the alpha)
    let looks: Vec<Look> = looks.into_iter().map(|l| if l.blend { Look { skin: opaque(l.skin), blend: false, ..l } } else { l }).collect();
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (t, ..) in &tris {
        for p in t {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    if tris.is_empty() || !(lo.x.is_finite() && hi.x.is_finite()) {
        return None;
    }
    let base = lo.z.max(-1.0);
    let mut faces: Vec<(f32, Face)> = tris
        .iter()
        .filter_map(|(t, uv, look)| {
            let area = (t[1] - t[0]).cross(t[2] - t[0]).length() * 0.5;
            let zhi = t[0].z.max(t[1].z).max(t[2].z);
            (area >= 0.04 && zhi > base + 0.15).then(|| (area, (t.map(|p| [p.x, p.y, (p.z - base).max(0.0)]), uv.map(|q| [q.x, q.y]), *look)))
        })
        .collect();
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));
    faces.truncate(FACES);
    Some(Dressed { faces: faces.into_iter().map(|(_, f)| f).collect(), looks })
}

/// The model of the object `file` (named from `root`), every face of its most detailed level with
/// its texture coordinates and look ([`Worn`]).
pub fn worn(root: &Path, file: &str) -> Option<Worn> {
    let path = omsi_cfg::resolve_path(root, file);
    let sco = omsi_scenery::sco::SceneryObject::load(&path).ok()?;
    let sco_dir = path.parent()?.to_path_buf();
    let (model, model_dir) = match &sco.model_file {
        Some(m) => {
            let mp = omsi_cfg::resolve_path(&sco_dir, m);
            (omsi_model::Model::load(&mp).ok()?, mp.parent()?.to_path_buf())
        }
        None => (sco.model.clone(), sco_dir.clone()),
    };
    if model.lods.is_empty() {
        return None;
    }
    // (where the game looks for a scenery object's textures)
    let mut dirs: Vec<PathBuf> = vec![omsi_cfg::resolve_path(&sco_dir, "texture"), omsi_cfg::resolve_path(&model_dir, "texture"), model_dir.clone()];
    if let Some(p) = model_dir.parent() {
        dirs.push(omsi_cfg::resolve_path(p, "texture"));
    }
    dirs.push(omsi_cfg::resolve_path(root, "Texture"));
    let dir_refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
    let mut looks: Vec<Look> = Vec::new();
    let mut by_name: HashMap<(String, [u32; 4], i32), u16> = HashMap::new();
    let mut tris: Vec<([Vec3; 3], [Vec2; 3], u16)> = Vec::new();
    for md in model.lod_meshes(0) {
        if md.mouse_event.is_some() || md.is_shadow {
            continue;
        }
        let mp = omsi_cfg::resolve_path(&omsi_cfg::resolve_path(&model_dir, "model"), &md.file);
        let mp = if omsi_cfg::vfs::is_file(&mp) { mp } else { omsi_cfg::resolve_path(&model_dir, &md.file) };
        let Ok(m) = omsi_o3d::load_mesh(&mp) else { continue };
        // (a material that tests its texture's alpha, `[matl_alpha] 1`: a fence, leaves - the
        // clear parts holes; one that blends it, `2`: glass, a mark laid on the road)
        let alpha_of = |tex: &str| md.materials.iter().filter(|d| d.texture.eq_ignore_ascii_case(tex.trim())).map(|d| d.alpha).next_back().unwrap_or(0);
        // (as the generator reads it: its y the height)
        let swap = |v: Vec3| Vec3::new(v.x, v.z, v.y);
        for t in &m.triangles {
            let p = t.indices.map(|i| m.vertices.get(i as usize).map(|v| swap(v.position)).unwrap_or(Vec3::ZERO));
            let uv = t.indices.map(|i| m.vertices.get(i as usize).map(|v| v.uv).unwrap_or(Vec2::ZERO));
            let (tex, colour) = m.materials.get(t.material as usize).map(|mt| (mt.texture.as_str(), mt.diffuse)).unwrap_or(("", [0.8, 0.8, 0.8, 1.0]));
            let alpha = alpha_of(tex);
            let (cut, blend) = (alpha == 1, alpha >= 2);
            let name = (tex.to_ascii_lowercase(), colour.map(f32::to_bits), alpha);
            let look = *by_name.entry(name).or_insert_with(|| {
                let found = (!tex.trim().is_empty() && !super::bus3d::pack::is_null_texture(tex)).then(|| omsi_texture::find_texture(tex, &dir_refs)).flatten();
                let c = |k: usize| (colour[k].clamp(0.0, 1.0) * 255.0).round() as u8;
                looks.push(match found {
                    Some(p) => Look { skin: skin_id(&p, cut || blend), colour: [255; 4], cut, blend },
                    None => Look { skin: 0, colour: [c(0), c(1), c(2), 255], cut: false, blend: false },
                });
                (looks.len() - 1).min(u16::MAX as usize) as u16
            });
            tris.push((p, uv, look));
        }
    }
    Some(Worn { tris, looks, road: sco.surface || !sco.paths.is_empty() })
}

/// The houses of the map `global` dressed, read as they are asked for: the tiles once, each
/// object once however often it stands.
struct Wardrobe {
    global: PathBuf,
    tiles: MapTiles,
    models: HashMap<String, Option<Arc<Dressed>>>,
}

/// The house placed at `at` (map metres) facing `heading`, as the game's picture places it:
/// dressed, or None where its object is not found or has no model.
pub fn dressed_at(global: &Path, at: [f64; 2], heading: f32) -> Option<Arc<Dressed>> {
    static KEPT: Mutex<Option<Wardrobe>> = Mutex::new(None);
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if kept.as_ref().is_none_or(|w| w.global != global) {
        *kept = Some(Wardrobe { global: global.to_path_buf(), tiles: MapTiles::open(global, "")?, models: HashMap::new() });
    }
    let w = kept.as_mut()?;
    let rel = [(at[0] - w.tiles.origin.x) as f32, (at[1] - w.tiles.origin.y) as f32];
    let object = w.tiles.object_at(rel, heading)?;
    if let Some(had) = w.models.get(&object.key) {
        return had.clone();
    }
    let made = dressed(&w.tiles.root, &object.file).map(Arc::new);
    w.models.insert(object.key, made.clone());
    made
}

/// The picture `rgba` (`w` x `h`) made `nw` x `nh`, each pixel the mean of the ones it covers.
fn area_resize(rgba: &[u8], w: usize, h: usize, nw: usize, nh: usize) -> Vec<u8> {
    let mut out = vec![0u8; nw * nh * 4];
    for y in 0..nh {
        let (y0, y1) = (y * h / nh, ((y + 1) * h / nh).max(y * h / nh + 1).min(h));
        for x in 0..nw {
            let (x0, x1) = (x * w / nw, ((x + 1) * w / nw).max(x * w / nw + 1).min(w));
            let mut sum = [0u32; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let at = (sy * w + sx) * 4;
                    for (k, s) in sum.iter_mut().enumerate() {
                        *s += rgba[at + k] as u32;
                    }
                }
            }
            let n = ((y1 - y0) * (x1 - x0)).max(1) as u32;
            for (k, s) in sum.iter().enumerate() {
                out[(y * nw + x) * 4 + k] = ((s + n / 2) / n) as u8;
            }
        }
    }
    out
}

/// A house's texture by its id, as the phone gets it: [`SKIN`] pixels a side, a JPEG - a PNG
/// where its clear parts are holes. Made once.
pub fn skin(id: u32) -> Option<Skin> {
    if let Some(had) = MADE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).get(&id) {
        return Some(had.clone());
    }
    let (path, cut) = skins().as_ref()?.get(&id).cloned()?;
    let img = omsi_texture::decode_file(&path).ok()?;
    if img.width == 0 || img.height == 0 || img.rgba.len() < img.width as usize * img.height as usize * 4 {
        return None;
    }
    let side = SKIN as usize;
    let px = area_resize(&img.rgba, img.width as usize, img.height as usize, side, side);
    let holes = cut && img.has_alpha && px.as_chunks::<4>().0.iter().any(|p| p[3] < 250);
    let mut bytes = Vec::new();
    use image::ImageEncoder;
    let kind = if holes {
        image::codecs::png::PngEncoder::new(&mut bytes).write_image(&px, SKIN, SKIN, image::ExtendedColorType::Rgba8).ok()?;
        "image/png"
    } else {
        let rgb: Vec<u8> = px.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 82).write_image(&rgb, SKIN, SKIN, image::ExtendedColorType::Rgb8).ok()?;
        "image/jpeg"
    };
    let made = Arc::new((bytes, kind));
    MADE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new).insert(id, made.clone());
    Some(made)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hub_companion::ground::{cache_file, Ground, SQUARE};

    #[test]
    fn a_textures_id_is_its_own_whenever_it_is_asked_for() {
        let wall = Path::new("C:/OMSI 2/Sceneryobjects/Haus/texture/wall.bmp");
        // (the same file in another case or with the other slashes, any run of the add-on)
        assert_eq!(own_id(wall, false), own_id(Path::new("c:\\omsi 2\\sceneryobjects\\haus\\texture\\WALL.bmp"), false));
        assert_ne!(own_id(wall, false), own_id(wall, true));
        assert_ne!(own_id(wall, false), own_id(Path::new("C:/OMSI 2/Sceneryobjects/Haus/texture/roof.bmp"), false));
        assert!(own_id(wall, false) > 0 && own_id(wall, false) <= 0x7fff_ffff);
        // the ids given out, read back as they were written
        let line = format!("{}\t0\t{}\n{}\t1\t{}\nbroken\n", own_id(wall, false), wall.display(), 7, "C:/x/fence.tga");
        let read = read_skins(&line);
        assert_eq!(read.get(&own_id(wall, false)), Some(&(wall.to_path_buf(), false)));
        assert_eq!(read.get(&7), Some(&(PathBuf::from("C:/x/fence.tga"), true)));
        assert_eq!(read.len(), 2);
    }

    /// How the houses of the squares about a place come dressed, on a real map where a game left
    /// its picture: `HUB_MAP=<global.cfg> HUB_NEAR=<an entry point's name> cargo test -p hub-api
    /// --release -- --ignored --nocapture dressing_by_square` (Ahlheim's busiest square without).
    /// Each square: its houses, those in their own textures and why the others are not.
    #[test]
    #[ignore]
    fn dressing_by_square() {
        let map = std::env::var("HUB_MAP").unwrap_or_else(|_| "maps/Ahlheim 5/global.cfg".into());
        let Some(global) = hub_core::paths::resolve_content(&map).filter(|p| p.is_file()) else { return };
        let Some(g) = cache_file(&map).and_then(|f| Ground::open(&f)) else { return };
        let t0 = std::time::Instant::now();
        // (the square of the entry point named, else of the most houses: map metres)
        let near = std::env::var("HUB_NEAR").ok();
        let centre = near
            .as_deref()
            .and_then(|n| super::super::mapview::labels_near(&map, &global, "", [0.0, 0.0], 1e7).1.into_iter().find(|(name, _)| name.contains(n)).map(|(_, p)| p))
            .unwrap_or_else(|| {
                let mut count: std::collections::HashMap<(i64, i64), usize> = std::collections::HashMap::new();
                for h in &g.houses {
                    let p = h.outline[0];
                    *count.entry((((p[0] as f64 + g.origin[0]) / SQUARE).floor() as i64, ((p[1] as f64 + g.origin[1]) / SQUARE).floor() as i64)).or_default() += 1;
                }
                let (&(i, j), _) = count.iter().max_by_key(|(_, n)| **n).unwrap();
                [(i as f64 + 0.5) * SQUARE, (j as f64 + 0.5) * SQUARE]
            });
        let (ci, cj) = ((centre[0] / SQUARE).floor() as i64, (centre[1] / SQUARE).floor() as i64);
        println!("{map}: about {centre:?} (square {ci},{cj})");
        let dress = |it: &hub_companion::ground::Instance| dressed_at(&global, [it.at[0] as f64 + g.origin[0], it.at[1] as f64 + g.origin[1]], it.heading);
        let mut all: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        let mut sum = hub_companion::ground::Dressing::default();
        for dj in [1, 0, -1] {
            for di in [-1, 0, 1] {
                let (si, sj) = (ci + di, cj + dj);
                let corner = [si as f64 * SQUARE - g.origin[0], sj as f64 * SQUARE - g.origin[1]];
                let d = g.dressing(corner, &dress);
                let bad = d.skins.iter().filter(|&&id| skin(id).is_none()).count();
                println!(
                    "  square {si},{sj}: {} houses ({} faces), {} textured, {} not: {} raised from the outline, {} over the budget, {} not found, {} without a texture; {} textures ({bad} not made); {} with their biggest faces",
                    d.houses,
                    d.faces,
                    d.textured,
                    d.houses - d.textured,
                    d.plain,
                    d.over_budget,
                    d.not_found,
                    d.untextured,
                    d.skins.len(),
                    d.trimmed
                );
                all.extend(d.skins.iter().copied());
                sum.houses += d.houses;
                sum.textured += d.textured;
                sum.plain += d.plain;
                sum.over_budget += d.over_budget;
                sum.not_found += d.not_found;
                sum.untextured += d.untextured;
            }
        }
        println!(
            "nine squares: {} houses, {} textured ({:.0}%), {} raised, {} over the budget, {} not found, {} without a texture; {} textures in all; {:?}",
            sum.houses,
            sum.textured,
            100.0 * sum.textured as f64 / sum.houses.max(1) as f64,
            sum.plain,
            sum.over_budget,
            sum.not_found,
            sum.untextured,
            all.len(),
            t0.elapsed()
        );
        // the textures the phone's map wears at once: every square in full within 1.2 km of the
        // middle of this one (the whole map kept on the phone)
        let mut reach: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        let mut squares = 0;
        for dj in -3i64..=3 {
            for di in -3i64..=3 {
                let (si, sj) = (ci + di, cj + dj);
                if ((si as f64 + 0.5) * SQUARE - centre[0]).hypot((sj as f64 + 0.5) * SQUARE - centre[1]) > 1200.0 {
                    continue;
                }
                squares += 1;
                reach.extend(g.dressing([si as f64 * SQUARE - g.origin[0], sj as f64 * SQUARE - g.origin[1]], &dress).skins);
            }
        }
        println!("within 1.2 km: {squares} squares, {} textures worn at once", reach.len());
        // what the phone showed, looking at the place: a square in full (its houses in their
        // textures) when its middle was within 700 m - one far by its middle comes light, its
        // houses raised from their outlines, plain - and what it shows now, a square in full when
        // its nearest point is
        for (name, full) in [("before", 700.0), ("after", 700.0)] {
            let (mut houses, mut textured) = (0, 0);
            let mut line = Vec::new();
            for dj in [1i64, 0, -1] {
                for di in [-1i64, 0, 1] {
                    let (si, sj) = (ci + di, cj + dj);
                    let far = if name == "before" {
                        ((si as f64 + 0.5) * SQUARE - centre[0]).hypot((sj as f64 + 0.5) * SQUARE - centre[1])
                    } else {
                        let d = |lo: f64, v: f64| (lo - v).max(0.0).max(v - lo - SQUARE);
                        d(si as f64 * SQUARE, centre[0]).hypot(d(sj as f64 * SQUARE, centre[1]))
                    };
                    let d = g.dressing([si as f64 * SQUARE - g.origin[0], sj as f64 * SQUARE - g.origin[1]], &dress);
                    let shown = if far <= full { d.textured } else { 0 };
                    houses += d.houses;
                    textured += shown;
                    line.push(format!("{si},{sj}{}: {shown}/{}", if far <= full { "" } else { " light" }, d.houses));
                }
            }
            println!("  the phone {name}: {textured} of {houses} houses in their textures - {}", line.join(", "));
        }
        // (for the app's mock server: `HUB_GROUND_DUMP=<dir>` writes these squares dressed, their
        // roads in their textures, as <i>_<j>.bin, i from -3 to 2 and j from -3 to 3 about the
        // place, and the textures they wear as skins/<id>.jpg or .png)
        let Some(dir) = std::env::var_os("HUB_GROUND_DUMP").map(PathBuf::from) else { return };
        std::fs::create_dir_all(dir.join("skins")).unwrap();
        let mut worn: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for i in -3..=2 {
            for j in -3..=3 {
                let corner = [(ci + i) as f64 * SQUARE - g.origin[0], (cj + j) as f64 * SQUARE - g.origin[1]];
                let paved = super::super::roadtex::paved_in(&global, [corner[0] + g.origin[0], corner[1] + g.origin[1]], SQUARE);
                std::fs::write(dir.join(format!("{i}_{j}.bin")), g.square_dressed(corner, false, &[], Some(&dress), paved.as_deref()).unwrap()).unwrap();
                worn.extend(g.dressing(corner, &dress).skins);
                if let Some(p) = &paved {
                    worn.extend(p.looks.iter().chain(p.standing.iter().flat_map(|s| s.1.iter())).map(|l| l.skin).filter(|&s| s != 0));
                }
            }
        }
        for &id in &worn {
            let Some(s) = skin(id) else { continue };
            std::fs::write(dir.join("skins").join(format!("{id}.{}", if s.1 == "image/png" { "png" } else { "jpg" })), &s.0).unwrap();
        }
        println!("dumped about square {ci},{cj} to {dir:?}: {} textures", worn.len());
    }

    /// What dressing the houses would cost, on a real map where a game left its picture:
    /// `cargo test -p hub-api --release -- --ignored --nocapture houses_dressed`.
    #[test]
    #[ignore]
    fn houses_dressed() {
        let map = "maps/Ahlheim 5/global.cfg";
        let Some(global) = hub_core::paths::resolve_content(map).filter(|p| p.is_file()) else { return };
        let Some(g) = cache_file(map).and_then(|f| Ground::open(&f)) else { return };
        let t0 = std::time::Instant::now();
        let mut tiles = MapTiles::open(&global, "").expect("tiles");
        println!("origin: layers {:?}, tiles {:?}", g.origin, tiles.origin);
        // the square with the most houses, and those about it
        let mut count: std::collections::HashMap<(i64, i64), usize> = std::collections::HashMap::new();
        for h in &g.houses {
            let p = h.outline[0];
            *count.entry(((p[0] as f64 / SQUARE).floor() as i64, (p[1] as f64 / SQUARE).floor() as i64)).or_default() += 1;
        }
        let (&(ci, cj), _) = count.iter().max_by_key(|(_, n)| **n).unwrap();
        let mut all_tex: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        let mut kept: HashMap<u32, Option<Dressed>> = HashMap::new();
        for (di, dj) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1)] {
            let (si, sj) = (ci + di, cj + dj);
            let lo = [(si as f64 * SQUARE) as f32, (sj as f64 * SQUARE) as f32];
            let hi = [lo[0] + SQUARE as f32, lo[1] + SQUARE as f32];
            let mut wanted: Vec<u32> = g
                .houses
                .iter()
                .filter(|h| {
                    let n = h.outline.len() as f32;
                    let (sx, sy) = h.outline.iter().fold((0.0, 0.0), |(a, b), p| (a + p[0], b + p[1]));
                    let (x, y) = (sx / n, sy / n);
                    x >= lo[0] && x < hi[0] && y >= lo[1] && y < hi[1]
                })
                .filter_map(|h| h.model)
                .collect();
            wanted.sort();
            wanted.dedup();
            let (mut found, mut faces_cache, mut faces_ours, mut spent) = (0, 0usize, 0usize, 0usize);
            let mut texs: HashMap<u32, usize> = HashMap::new();
            let mut untextured = 0;
            let mut bounds_off = 0;
            for &i in &wanted {
                let it = g.instances[i as usize];
                let model = &g.models[it.model as usize];
                if spent + model.0.len() > hub_companion::ground::MODEL_BUDGET {
                    continue;
                }
                spent += model.0.len();
                faces_cache += model.0.len();
                let d = kept.entry(it.model).or_insert_with(|| tiles.object_at(it.at, it.heading).and_then(|p| dressed(&tiles.root, &p.file))).clone();
                let Some(d) = d else { continue };
                found += 1;
                faces_ours += d.faces.len();
                // (the same model as the layers': their boxes agree)
                let bx = |f: &mut dyn Iterator<Item = [f32; 3]>| {
                    f.fold(([f32::MAX; 3], [f32::MIN; 3]), |(l, h), p| ([l[0].min(p[0]), l[1].min(p[1]), l[2].min(p[2])], [h[0].max(p[0]), h[1].max(p[1]), h[2].max(p[2])]))
                };
                let (a, b) = (bx(&mut model.0.iter().flatten().copied()), bx(&mut d.faces.iter().flat_map(|f| f.0)));
                if (0..3).any(|k| (a.0[k] - b.0[k]).abs() > 0.5 || (a.1[k] - b.1[k]).abs() > 0.5) {
                    bounds_off += 1;
                    if bounds_off <= 3 {
                        println!("  box differs: layers {a:?} ours {b:?}");
                    }
                }
                for f in &d.faces {
                    match d.looks[f.2 as usize].skin {
                        0 => untextured += 1,
                        id => *texs.entry(id).or_default() += 1,
                    }
                }
            }
            all_tex.extend(texs.keys().copied());
            // what the phone gets: the square as before, and dressed
            let corner = [si as f64 * SQUARE, sj as f64 * SQUARE];
            let gz = |b: &[u8]| {
                let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
                std::io::Write::write_all(&mut z, b).unwrap();
                z.finish().unwrap().len()
            };
            let t = std::time::Instant::now();
            let dress = |it: &hub_companion::ground::Instance| dressed_at(&global, [it.at[0] as f64 + g.origin[0], it.at[1] as f64 + g.origin[1]], it.heading);
            let worn = g.square_dressed(corner, false, &[], Some(&dress), None).unwrap();
            let plain = g.square(corner, false).unwrap();
            println!(
                "square {si},{sj}: {} models wanted, {found} found; faces: layers {faces_cache}, ours {faces_ours} ({untextured} untextured); {} textures; boxes off {bounds_off}; bytes {} KB gz {} KB, dressed {} KB gz {} KB ({:?})",
                wanted.len(),
                texs.len(),
                plain.len() / 1024,
                gz(&plain) / 1024,
                worn.len() / 1024,
                gz(&worn) / 1024,
                t.elapsed()
            );
        }
        let t = std::time::Instant::now();
        let (mut jpeg, mut png, mut bytes) = (0, 0, 0);
        for &id in &all_tex {
            let Some(s) = skin(id) else { continue };
            bytes += s.0.len();
            if s.1 == "image/png" {
                png += 1;
            } else {
                jpeg += 1;
            }
        }
        println!("five squares: {} textures ({jpeg} JPEG, {png} PNG), {} KB for the phone, made in {:?}; all {:?}", all_tex.len(), bytes / 1024, t.elapsed(), t0.elapsed());
        // (for the app's mock server: `HUB_GROUND_DUMP=<dir>` writes the squares about the one with
        // the most houses dressed, as <i>_<j>.bin with i from -3 to 2 and j from -3 to 3 about it,
        // and the textures they wear as skins/<id>.jpg or .png)
        let Some(dir) = std::env::var_os("HUB_GROUND_DUMP").map(PathBuf::from) else { return };
        std::fs::create_dir_all(dir.join("skins")).unwrap();
        let dress = |it: &hub_companion::ground::Instance| dressed_at(&global, [it.at[0] as f64 + g.origin[0], it.at[1] as f64 + g.origin[1]], it.heading);
        for i in -3..=2 {
            for j in -3..=3 {
                let corner = [(ci + i) as f64 * SQUARE, (cj + j) as f64 * SQUARE];
                std::fs::write(dir.join(format!("{i}_{j}.bin")), g.square_dressed(corner, false, &[], Some(&dress), None).unwrap()).unwrap();
            }
        }
        let ids: Vec<u32> = skins().as_ref().map(|m| m.keys().copied().collect()).unwrap_or_default();
        for &id in &ids {
            let Some(s) = skin(id) else { continue };
            std::fs::write(dir.join("skins").join(format!("{id}.{}", if s.1 == "image/png" { "png" } else { "jpg" })), &s.0).unwrap();
        }
        println!("dumped about square {ci},{cj} to {dir:?}: {} textures", ids.len());
    }
}
