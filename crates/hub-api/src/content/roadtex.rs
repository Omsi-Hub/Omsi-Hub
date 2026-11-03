//! The roads of the companion's 3D navigator as the game lays them: every spline's surface in its
//! own textures (the carriageways with the markings painted in them, the pavements, the kerbs, the
//! marking splines laid over the road) and the flat faces of the road objects (the crossings, the
//! bus bays), read from the map's tiles as the game builds them (`omsi_geometry::build_spline_mesh`).
//!
//! The map is drawn flat: a spline is laid as though its curve ran level (its profile's own heights
//! kept), and what lies within [`FLAT`] of the ground is the paved ground, laid in the order the game
//! lays it over itself - the opaque surfaces, then the marks and edges seen through. What stands
//! higher (a wall, a railing) stands with the houses.

use super::housetex::{skin_id, worn, MapTiles, Worn};
use glam::{DVec2, DVec3, Vec3};
use hashbrown::HashMap;
use hub_companion::dress::{Face, Flat, Look, Paved, Placed};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// What lies this close to the ground (m) is laid flat as the paved ground; higher, it stands.
pub const FLAT: f32 = 0.3;
/// The faces of a square's paved ground at most (the opaque surfaces first).
pub const PAVED_FACES: usize = 120_000;
/// An object other than a piece of road lays its flat faces with the roads where they cover this
/// much ground (m²): a car park, a square, a mark laid on the road.
const BIG_FLAT: f32 = 30.0;
/// The squares' paved ground kept at most.
const KEPT: usize = 96;

/// Where the game looks for a spline's textures: beside its `.sli`, in the `texture` folders there
/// and above, then the game's own.
fn texture_dirs(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![omsi_cfg::resolve_path(dir, "texture"), dir.to_path_buf()];
    if let Some(p) = dir.parent() {
        dirs.push(omsi_cfg::resolve_path(p, "texture"));
    }
    dirs.push(omsi_cfg::resolve_path(root, "Texture"));
    dirs
}

/// A spline type as read, and the folder its textures are looked for from.
type Sli = Option<Arc<(omsi_scenery::sli::Spline, PathBuf)>>;

/// The roads of the map `global`, read as squares ask for them: the tiles, the spline types and
/// the objects once each.
struct Roads {
    global: PathBuf,
    tiles: MapTiles,
    slis: HashMap<String, Sli>,
    objects: HashMap<String, Option<Arc<Worn>>>,
    squares: HashMap<(i64, i64), Option<Arc<Paved>>>,
}

/// The paved ground of the square of `side` metres with its lower corner at `lo` (map metres):
/// [`Paved`], x, y from that corner. None where the map has no tiles there.
pub fn paved_in(global: &Path, lo: [f64; 2], side: f64) -> Option<Arc<Paved>> {
    static ROADS: Mutex<Option<Roads>> = Mutex::new(None);
    let mut kept = ROADS.lock().unwrap_or_else(|e| e.into_inner());
    if kept.as_ref().is_none_or(|r| r.global != global) {
        *kept = Some(Roads { global: global.to_path_buf(), tiles: MapTiles::open(global, "")?, slis: HashMap::new(), objects: HashMap::new(), squares: HashMap::new() });
    }
    let r = kept.as_mut()?;
    let key = ((lo[0] / side).round() as i64, (lo[1] / side).round() as i64);
    if let Some(had) = r.squares.get(&key) {
        return had.clone();
    }
    let made = r.pave(DVec2::new(lo[0], lo[1]), side).map(Arc::new);
    if r.squares.len() >= KEPT {
        r.squares.clear();
    }
    r.squares.insert(key, made.clone());
    made
}

/// A square's faces as they are gathered: its looks, the faces of each layer in the order laid.
#[derive(Default)]
struct Laying {
    looks: Vec<Look>,
    index: HashMap<Look, u16>,
    /// The opaque surfaces of the splines, then the objects'; then the seen-through ones of each.
    layers: [Vec<Flat>; 4],
    standing: Vec<Placed>,
}

impl Laying {
    fn look(&mut self, l: Look) -> u16 {
        *self.index.entry(l).or_insert_with(|| {
            self.looks.push(l);
            (self.looks.len() - 1).min(u16::MAX as usize) as u16
        })
    }

    /// A face (x, y from the square's corner, z up from the ground) of a spline or an object,
    /// wearing `look`: laid flat, or standing.
    fn put(&mut self, p: [Vec3; 3], uv: [[f32; 2]; 3], look: Look, object: bool) {
        if p.iter().all(|v| v.z.abs() <= FLAT) {
            let l = self.look(look);
            let layer = object as usize + if look.cut || look.blend { 2 } else { 0 };
            self.layers[layer].push((p.map(|v| [v.x, v.y]), uv, l));
            return;
        }
        // (a standing face lit by the way it faces: its middle a metre behind it)
        let n = (p[1] - p[0]).cross(p[2] - p[0]);
        let flat = n.truncate().normalize_or_zero();
        let c = (p[0] + p[1] + p[2]) / 3.0 - flat.extend(0.0);
        let face: Face = (p.map(|v| [v.x, v.y, v.z.max(0.0)]), uv, 0);
        self.standing.push((vec![face], vec![look], [c.x, c.y, 0.0]));
    }
}

impl Roads {
    /// The spline type `file`, read once.
    fn sli(&mut self, file: &str) -> Sli {
        let root = &self.tiles.root;
        self.slis
            .entry(file.trim().to_ascii_lowercase())
            .or_insert_with(|| {
                let path = omsi_cfg::resolve_path(root, file);
                let def = omsi_scenery::sli::Spline::load(&path).ok().filter(|d| !d.only_editor)?;
                Some(Arc::new((def, path.parent().map(Path::to_path_buf).unwrap_or_default())))
            })
            .clone()
    }

    fn pave(&mut self, lo: DVec2, side: f64) -> Option<Paved> {
        let ts = self.tiles.tile_size();
        let inside = |c: Vec3| c.x >= 0.0 && c.y >= 0.0 && (c.x as f64) < side && (c.y as f64) < side;
        let mut lay = Laying::default();
        // (a tile's splines and objects reach past it: the tiles about the square too)
        let (t0, t1) = (((lo / ts).floor().as_ivec2()) - 1, ((lo + side) / ts).floor().as_ivec2() + 1);
        let mut any = false;
        for tx in t0.x..=t1.x {
            for ty in t0.y..=t1.y {
                let Some(tile) = self.tiles.tile((tx, ty)) else { continue };
                any = true;
                let o2 = DVec2::new(tx as f64 * ts, ty as f64 * ts);
                for sp in tile.splines.iter().filter(|s| !s.deleted && !s.file.trim().is_empty()) {
                    let Some(sli) = self.sli(&sp.file) else { continue };
                    let (def, dir) = (&sli.0, &sli.1);
                    // (laid level: the profile's own heights only)
                    let mut curve = omsi_geometry::SplineCurve::from_map(sp, o2);
                    curve.start.z = 0.0;
                    (curve.grad_start, curve.grad_end, curve.delta_h, curve.cant_start, curve.cant_end) = (0.0, 0.0, None, 0.0, 0.0);
                    let mesh = omsi_geometry::build_spline_mesh(def, &curve, sp.mirror, DVec3::new(lo.x, lo.y, 0.0));
                    let dirs = texture_dirs(&self.tiles.root, dir);
                    let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
                    for &(first, count, slot) in &mesh.ranges {
                        let Some(tex) = def.textures.get(slot as usize) else { continue };
                        let (cut, blend) = (tex.alpha == 1, tex.alpha >= 2);
                        let look = match omsi_texture::find_texture(&tex.file, &refs) {
                            Some(p) => Look { skin: skin_id(&p, cut || blend), colour: [255; 4], cut, blend },
                            None => continue,
                        };
                        for t in mesh.indices[first as usize..(first + count) as usize].as_chunks::<3>().0 {
                            let p = [0, 1, 2].map(|k| mesh.positions[t[k] as usize]);
                            if inside((p[0] + p[1] + p[2]) / 3.0) {
                                lay.put(p, [0, 1, 2].map(|k| mesh.uvs[t[k] as usize].to_array()), look, false);
                            }
                        }
                    }
                }
            }
        }
        if !any {
            return None;
        }
        // the objects' faces flat on the ground: the pieces of road, and what covers much ground
        let o = (self.tiles.origin - lo).as_vec2();
        for tx in t0.x..=t1.x {
            for ty in t0.y..=t1.y {
                for p in self.tiles.objects((tx, ty)).to_vec() {
                    let root = self.tiles.root.clone();
                    let Some(w) = self.objects.entry(p.key.clone()).or_insert_with(|| worn(&root, &p.file).map(Arc::new)).clone() else { continue };
                    let ground: Vec<&([Vec3; 3], [glam::Vec2; 3], u16)> = w
                        .tris
                        .iter()
                        .filter(|(t, ..)| {
                            let n = (t[1] - t[0]).cross(t[2] - t[0]);
                            n.length_squared() > 1e-8 && n.z.abs() >= n.length() * 0.7 && t.iter().all(|v| v.z.abs() <= FLAT)
                        })
                        .collect();
                    if !w.road && ground.iter().map(|(t, ..)| (t[1] - t[0]).cross(t[2] - t[0]).z.abs() * 0.5).sum::<f32>() < BIG_FLAT {
                        continue;
                    }
                    let a = -(p.heading as f64).to_radians();
                    let (sn, cs) = (a.sin() as f32, a.cos() as f32);
                    let place = |v: Vec3| Vec3::new(v.x * cs - v.y * sn + p.at[0] + o.x, v.x * sn + v.y * cs + p.at[1] + o.y, v.z);
                    for (t, uv, look) in ground {
                        let q = t.map(place);
                        let Some(&l) = w.looks.get(*look as usize) else { continue };
                        if inside((q[0] + q[1] + q[2]) / 3.0) {
                            lay.put(q, uv.map(|v| v.to_array()), l, true);
                        }
                    }
                }
            }
        }
        let mut faces = Vec::new();
        for layer in lay.layers {
            faces.extend(layer.into_iter().take(PAVED_FACES - faces.len()));
        }
        Some(Paved { faces, looks: lay.looks, standing: lay.standing })
    }
}

#[cfg(test)]
mod tests {
    use super::super::housetex::MapTiles;
    use glam::{DVec2, DVec3};
    use hashbrown::HashMap;
    use std::path::{Path, PathBuf};

    /// What the roads of a square cost in their own textures, on a real map:
    /// `HUB_MAP=<global.cfg> HUB_AT=<x>,<y> cargo test -p hub-api --release -- --ignored --nocapture roads_measured`.
    #[test]
    #[ignore]
    fn roads_measured() {
        let map = std::env::var("HUB_MAP").unwrap_or_else(|_| "maps/Bad_Huegelsdorf_2020/global.cfg".into());
        let at: Vec<f64> = std::env::var("HUB_AT").unwrap_or_else(|_| "-3063,1713".into()).split(',').filter_map(|v| v.trim().parse().ok()).collect();
        let Some(global) = hub_core::paths::resolve_content(&map).filter(|p| p.is_file()) else { return };
        let tiles = MapTiles::open(&global, "").expect("tiles");
        let side = 500.0;
        let (si, sj) = ((at[0] / side).floor(), (at[1] / side).floor());
        let (lo, hi) = (DVec2::new(si * side, sj * side), DVec2::new((si + 1.0) * side, (sj + 1.0) * side));
        let ts = tiles.tile_size();
        let t0 = std::time::Instant::now();
        let mut slis: HashMap<String, Option<(omsi_scenery::sli::Spline, PathBuf)>> = HashMap::new();
        let (mut splines, mut tris, mut alpha_tris) = (0usize, 0usize, 0usize);
        let mut textures: HashMap<PathBuf, (usize, i32)> = HashMap::new();
        let mut missing: HashMap<String, usize> = HashMap::new();
        let mut kinds: HashMap<String, usize> = HashMap::new();
        for tx in (lo.x / ts).floor() as i32 - 1..=(hi.x / ts).floor() as i32 {
            for ty in (lo.y / ts).floor() as i32 - 1..=(hi.y / ts).floor() as i32 {
                let Some(tile) = tiles.tile((tx, ty)) else { continue };
                let o2 = DVec2::new(tx as f64 * ts, ty as f64 * ts);
                for sp in tile.splines.iter().filter(|s| !s.deleted && !s.file.trim().is_empty()) {
                    let def = slis.entry(sp.file.to_ascii_lowercase()).or_insert_with(|| {
                        let path = omsi_cfg::resolve_path(&tiles.root, &sp.file);
                        omsi_scenery::sli::Spline::load(&path).ok().map(|d| (d, path.parent().map(Path::to_path_buf).unwrap_or_default()))
                    });
                    let Some((def, dir)) = def.as_ref() else { continue };
                    if def.only_editor {
                        continue;
                    }
                    let curve = omsi_geometry::SplineCurve::from_map(sp, o2);
                    let mesh = omsi_geometry::build_spline_mesh(def, &curve, sp.mirror, DVec3::new(lo.x, lo.y, 0.0));
                    let mut here = 0;
                    for &(first, count, slot) in &mesh.ranges {
                        for t in mesh.indices[first as usize..(first + count) as usize].as_chunks::<3>().0 {
                            let c = (mesh.positions[t[0] as usize] + mesh.positions[t[1] as usize] + mesh.positions[t[2] as usize]) / 3.0;
                            if c.x < 0.0 || c.y < 0.0 || c.x >= side as f32 || c.y >= side as f32 {
                                continue;
                            }
                            here += 1;
                            let Some(tex) = def.textures.get(slot as usize) else { continue };
                            if tex.alpha > 0 {
                                alpha_tris += 1;
                            }
                            let dirs = [dir.join("texture"), dir.clone(), dir.parent().map(|p| p.join("texture")).unwrap_or_default(), tiles.root.join("Texture")];
                            let refs: Vec<&Path> = dirs.iter().map(PathBuf::as_path).collect();
                            match omsi_texture::find_texture(&tex.file, &refs) {
                                Some(p) => textures.entry(p).or_insert((0, tex.alpha)).0 += 1,
                                None => *missing.entry(tex.file.clone()).or_default() += 1,
                            }
                        }
                    }
                    if here > 0 {
                        splines += 1;
                        tris += here;
                        *kinds.entry(sp.file.replace('\\', "/").rsplit('/').next().unwrap_or("").to_string()).or_default() += 1;
                    }
                }
            }
        }
        // the objects' faces flat on the ground (the crossings, the bus bays, the marks laid on the road)
        let mut tiles = tiles;
        let (mut objects, mut flat, mut skins) = (0usize, 0usize, std::collections::BTreeSet::new());
        let mut worn: HashMap<String, Option<super::super::housetex::Worn>> = HashMap::new();
        let mut cats: [(usize, usize, std::collections::BTreeSet<u32>); 3] = Default::default();
        for tx in (lo.x / ts).floor() as i32 - 1..=(hi.x / ts).floor() as i32 + 1 {
            for ty in (lo.y / ts).floor() as i32 - 1..=(hi.y / ts).floor() as i32 + 1 {
                let placed = tiles.objects((tx, ty)).to_vec();
                for p in placed {
                    let w = worn.entry(p.key.clone()).or_insert_with(|| super::super::housetex::worn(&tiles.root, &p.file));
                    let Some(w) = w.as_ref() else { continue };
                    let a = -(p.heading as f64).to_radians();
                    let (sn, cs) = (a.sin() as f32, a.cos() as f32);
                    let o = (tiles.origin - lo).as_vec2();
                    let mut here = 0;
                    for (t, _, look) in &w.tris {
                        let n = (t[1] - t[0]).cross(t[2] - t[0]);
                        if n.length_squared() < 1e-8 || n.z.abs() < n.length() * 0.7 || t.iter().any(|v| v.z.abs() > 0.35) {
                            continue;
                        }
                        let c = (t[0] + t[1] + t[2]) / 3.0;
                        let (x, y) = (c.x * cs - c.y * sn + p.at[0] + o.x, c.x * sn + c.y * cs + p.at[1] + o.y);
                        if x < 0.0 || y < 0.0 || x >= side as f32 || y >= side as f32 {
                            continue;
                        }
                        here += 1;
                        if let Some(l) = w.looks.get(*look as usize).filter(|l| l.skin != 0) {
                            skins.insert(l.skin);
                        }
                    }
                    if here > 0 {
                        let area: f32 = w.tris.iter().filter(|(t, ..)| t.iter().all(|v| v.z.abs() <= 0.35)).map(|(t, ..)| (t[1] - t[0]).cross(t[2] - t[0]).z.abs() * 0.5).sum();
                        let k = if w.road {
                            0
                        } else if area >= 30.0 {
                            1
                        } else {
                            2
                        };
                        cats[k].0 += 1;
                        cats[k].1 += here;
                        for (t, _, look) in &w.tris {
                            if t.iter().all(|v| v.z.abs() <= 0.35) {
                                if let Some(l) = w.looks.get(*look as usize).filter(|l| l.skin != 0) {
                                    cats[k].2.insert(l.skin);
                                }
                            }
                        }
                        objects += 1;
                        flat += here;
                    }
                }
            }
        }
        println!("objects flat on the ground: {objects} objects, {flat} triangles, {} textures", skins.len());
        for (name, c) in ["road objects", "big flat (>= 30 m2)", "the rest"].iter().zip(&cats) {
            println!("  {name}: {} objects, {} triangles, {} textures", c.0, c.1, c.2.len());
        }
        let bytes: u64 = textures.keys().map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)).sum();
        println!("square {si},{sj}: {splines} splines, {tris} triangles ({alpha_tris} with alpha), {} textures ({:.1} MB of files), {:?}", textures.len(), bytes as f64 / 1e6, t0.elapsed());
        let mut by: Vec<_> = textures.iter().collect();
        by.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
        for (p, (n, a)) in by.iter().take(15) {
            let (w, h) = crate::content::bus3d::tex::probe(p).map(|(i, _)| (i.w, i.h)).unwrap_or((0, 0));
            println!("  {n:6} tris  alpha {a}  {w}x{h}  {}", p.display());
        }
        println!("  not found: {missing:?}");
        let mut k: Vec<_> = kinds.into_iter().collect();
        k.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        println!("  spline kinds: {:?}", &k[..k.len().min(20)]);
    }
}
