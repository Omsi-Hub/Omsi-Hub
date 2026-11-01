//! The game's own picture of the map, for the companion's 3D navigator. openOMSI reads a map's
//! tiles once into layers (`omsi_scene::mapgen`) and keeps them in its cache,
//! `~/.openomsi/cache/mapgen/<map>.bin` (`OMML`, openOMSI's `omsi-map/src/layers.rs`): the ground
//! by what it is painted with, every paved surface, the paint on the roads, rail and tram tracks
//! and rivers, single trees, and the houses as outlines with their roofs and heights. This reads
//! that file - not the houses' models: the phone raises a house from its outline - and hands it
//! out by squares of [`SQUARE`] metres, packed small ([`Ground::square`]).
//!
//! The houses' own models (untextured, as openOMSI's 3D map draws the nearest ones) go with
//! them, each triangle with the shade the game gives it, up to [`MODEL_BUDGET`] a square; a house
//! whose model did not fit is raised from its outline.
//!
//! The file is read once through: the houses, trees, lines and models are kept, and of the triangles
//! (hundreds of megabytes on a big map) only where each run of [`RUN`] of them lies. The
//! generator writes them tile by tile, so a run covers a small place, and a square reads only
//! the runs that touch it.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The side of a square the phone asks for (m).
pub const SQUARE: f64 = 500.0;
/// The layers' format this reads (openOMSI's `layers::VERSION`): another is not read.
pub const VERSION: u32 = 19;
const MAGIC: &[u8; 4] = b"OMML";
/// Land (grass, forest, field, paved, sand, water), paving (road, foot, area, green, inner) and
/// paint (white, amber), in that order: the classes a square's triangles come by.
pub const CLASSES: [usize; 3] = [6, 5, 2];
const ALL: usize = 13;
/// Triangles to a run.
const RUN: usize = 2048;
/// The houses' model triangles a square carries at most (openOMSI's small map allows 30 000 about
/// the bus; a square is a part of what the phone shows).
pub const MODEL_BUDGET: usize = 40_000;
/// Which way the shadows fall, per metre of height (openOMSI's `SHADOW_DIR`): the light comes from
/// the other way.
const SHADOW_DIR: [f32; 2] = [0.34, -0.28];

/// A square's positions: sixteenths of a metre from its corner, as `i16` (some 2 km either way:
/// a triangle reaching far out of its square still fits).
pub const UNITS: f64 = 16.0;

#[derive(Debug, Clone, Copy)]
struct Run {
    /// Where its first point is in the file, and how many triangles.
    at: u64,
    n: u32,
    lo: [f32; 2],
    hi: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct House {
    pub outline: Vec<[f32; 2]>,
    pub roof: Vec<[u16; 3]>,
    pub height: f32,
    /// A roof on posts (a canopy, a bridge): no walls.
    pub open: bool,
    /// The placed model it is a part of (`Ground::instances`), drawn in 3D where it fits.
    pub model: Option<u32>,
}

/// A placed model: which of `Ground::models`, where (relative to the origin), which way (compass
/// degrees).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Instance {
    pub model: u32,
    pub at: [f32; 2],
    pub heading: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// Rail, tram, river, road edge, area edge (openOMSI's `LINE_*`).
    pub class: u8,
    pub width: f32,
    pub points: Vec<[f32; 2]>,
}

/// A house's model: its faces (x, y, z up from its ground) and the middle of its ground plan.
pub type Model = (Vec<[[f32; 3]; 3]>, [f32; 2]);

/// A map's layers as the game cached them.
pub struct Ground {
    file: PathBuf,
    /// The file as it was read (its length and time): read again when the game writes it anew.
    stamp: (u64, Option<SystemTime>),
    /// The layers' positions are relative to this (the map's metres).
    pub origin: [f64; 2],
    runs: Vec<Vec<Run>>,
    pub houses: Vec<House>,
    pub trees: Vec<[f32; 3]>,
    pub lines: Vec<Line>,
    /// The houses' models: their faces (x, y, z up from the model's ground) and the middle of
    /// their ground plan (the walls face away from it).
    pub models: Vec<Model>,
    pub instances: Vec<Instance>,
}

/// Where openOMSI keeps a map's layers: its folder's name (the global.cfg's), made safe as the
/// game makes it, under `~/.openomsi/cache/mapgen`.
pub fn cache_file(map: &str) -> Option<PathBuf> {
    let m = map.replace('\\', "/");
    let mut parts: Vec<&str> = m.split('/').filter(|p| !p.is_empty()).collect();
    if parts.last().is_some_and(|p| p.to_lowercase().ends_with(".cfg")) {
        parts.pop();
    }
    let name = parts.last()?;
    let safe: String = name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let base = PathBuf::from(home);
    base.is_absolute().then(|| base.join(".openomsi").join("cache").join("mapgen").join(format!("{safe}.bin")))
}

fn stamp_of(path: &Path) -> Option<(u64, Option<SystemTime>)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.len(), m.modified().ok()))
}

struct Reader {
    r: BufReader<File>,
    at: u64,
    len: u64,
}

impl Reader {
    fn bytes<const N: usize>(&mut self) -> Option<[u8; N]> {
        let mut b = [0u8; N];
        self.r.read_exact(&mut b).ok()?;
        self.at += N as u64;
        Some(b)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.bytes::<1>()?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.bytes()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.bytes()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.bytes()?)).filter(|v| v.is_finite())
    }
    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.bytes()?)).filter(|v| v.is_finite())
    }
    /// A count of items at least `min` bytes each: never more than the bytes left allow.
    fn count(&mut self, min: u64) -> Option<usize> {
        let n = self.u32()? as u64;
        (n.checked_mul(min)? <= self.len.saturating_sub(self.at)).then_some(n as usize)
    }
    fn skip(&mut self, n: u64) -> Option<()> {
        self.r.seek_relative(i64::try_from(n).ok()?).ok()?;
        self.at += n;
        Some(())
    }
    fn points(&mut self) -> Option<Vec<[f32; 2]>> {
        let n = self.count(8)?;
        (0..n).map(|_| Some([self.f32()?, self.f32()?])).collect()
    }
}

impl Ground {
    /// The layers in `file`, if it is openOMSI's cache of this [`VERSION`]; None for anything else.
    pub fn open(file: &Path) -> Option<Ground> {
        let stamp = stamp_of(file)?;
        let mut r = Reader { r: BufReader::with_capacity(1 << 20, File::open(file).ok()?), at: 0, len: stamp.0 };
        if &r.bytes::<4>()? != MAGIC || r.u32()? != VERSION {
            return None;
        }
        r.skip(8)?; // (the key the game checks its tiles by)
        let origin = [r.f64()?, r.f64()?];
        let tiles = r.count(16)?;
        r.skip(tiles as u64 * 16)?;
        let mut runs = Vec::with_capacity(ALL);
        for want in CLASSES {
            let classes = r.count(4)?;
            for class in 0..classes {
                let n = r.count(8)? / 3;
                let mut of = Vec::new();
                let mut left = n;
                while left > 0 {
                    let take = left.min(RUN);
                    let mut run = Run { at: r.at, n: take as u32, lo: [f32::MAX; 2], hi: [f32::MIN; 2] };
                    for _ in 0..take * 3 {
                        let p = [r.f32()?, r.f32()?];
                        for (k, v) in p.into_iter().enumerate() {
                            run.lo[k] = run.lo[k].min(v);
                            run.hi[k] = run.hi[k].max(v);
                        }
                    }
                    of.push(run);
                    left -= take;
                }
                // (a class this reader does not know of is read past)
                if class < want {
                    runs.push(of);
                }
            }
            for _ in classes..want {
                runs.push(Vec::new());
            }
        }
        let mut houses = Vec::new();
        for _ in 0..r.count(17)? {
            let outline = r.points()?;
            let mut roof = Vec::new();
            for _ in 0..r.count(6)? {
                let t = [r.u16()?, r.u16()?, r.u16()?];
                if t.iter().any(|&i| i as usize >= outline.len()) {
                    return None;
                }
                roof.push(t);
            }
            let height = r.f32()?;
            let flags = r.u8()?;
            let inst = r.u32()?;
            houses.push(House { outline, roof, height, open: flags & 1 != 0, model: (flags & 2 != 0 && inst != u32::MAX).then_some(inst) });
        }
        let mut models = Vec::new();
        for _ in 0..r.count(4)? {
            let n = r.count(36)?;
            let mut faces = Vec::with_capacity(n);
            let (mut sx, mut sy) = (0.0f32, 0.0f32);
            for _ in 0..n {
                let mut t = [[0.0f32; 3]; 3];
                for v in t.iter_mut() {
                    *v = [r.f32()?, r.f32()?, r.f32()?];
                    sx += v[0];
                    sy += v[1];
                }
                faces.push(t);
            }
            let k = (n * 3).max(1) as f32;
            models.push((faces, [sx / k, sy / k]));
        }
        let mut instances = Vec::new();
        for _ in 0..r.count(16)? {
            let model = r.u32()?;
            if model as usize >= models.len() {
                return None;
            }
            instances.push(Instance { model, at: [r.f32()?, r.f32()?], heading: r.f32()? });
        }
        let mut trees = Vec::new();
        for _ in 0..r.count(12)? {
            trees.push([r.f32()?, r.f32()?, r.f32()?]);
        }
        let mut lines = Vec::new();
        for _ in 0..r.count(9)? {
            let class = r.u8()?;
            let width = r.f32()?;
            lines.push(Line { class, width, points: r.points()? });
        }
        Some(Ground { file: file.to_path_buf(), stamp, origin, runs, houses, trees, lines, models, instances })
    }

    /// Whether the game wrote the file anew since it was read.
    pub fn stale(&self) -> bool {
        stamp_of(&self.file) != Some(self.stamp)
    }

    /// The square with its lower corner at `corner` (the layers' metres, relative to
    /// [`Ground::origin`]), packed for the phone (see the protocol's `GET /api/ground`): every
    /// triangle, house and tree whose middle is in it, and every line that passes it, at
    /// sixteenths of a metre from the corner.
    /// `low`: a square far off - without the paint, the trees and the houses' models.
    pub fn square(&self, corner: [f64; 2], low: bool) -> Option<Vec<u8>> {
        self.square_with(corner, low, &[])
    }

    /// [`Ground::square`] with what stands at the stops in it (the game's picture of the map
    /// has no shelters, signs or benches): each object's middle and its triangles, in this
    /// file's coordinates (relative to [`Ground::origin`]; z up from the ground), sent with the
    /// houses' models (not in a square far off).
    pub fn square_with(&self, corner: [f64; 2], low: bool, props: &[([f32; 2], Vec<[[f32; 3]; 3]>)]) -> Option<Vec<u8>> {
        let lo = [corner[0] as f32, corner[1] as f32];
        let hi = [(corner[0] + SQUARE) as f32, (corner[1] + SQUARE) as f32];
        let inside = |x: f32, y: f32| x >= lo[0] && x < hi[0] && y >= lo[1] && y < hi[1];
        let q = |v: f32, c: f64| ((v as f64 - c) * UNITS).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16;
        let mut out = Vec::with_capacity(1 << 16);
        out.extend_from_slice(b"OHG1");
        out.extend_from_slice(&(UNITS as u16).to_le_bytes());
        out.extend_from_slice(&(ALL as u16).to_le_bytes());
        let mut file = File::open(&self.file).ok()?;
        let mut buf = Vec::new();
        for (class, runs) in self.runs.iter().enumerate() {
            let count_at = out.len();
            out.extend_from_slice(&0u32.to_le_bytes());
            let mut n = 0u32;
            // (far off, no paint: the classes after the paving)
            let skip = low && class >= CLASSES[0] + CLASSES[1];
            for run in runs.iter().filter(|r| !skip && r.hi[0] >= lo[0] && r.lo[0] < hi[0] && r.hi[1] >= lo[1] && r.lo[1] < hi[1]) {
                buf.resize(run.n as usize * 24, 0);
                file.seek(SeekFrom::Start(run.at)).ok()?;
                file.read_exact(&mut buf).ok()?;
                for t in buf.as_chunks::<24>().0 {
                    let f = |i: usize| f32::from_le_bytes(t[i * 4..i * 4 + 4].try_into().unwrap_or_default());
                    let (cx, cy) = ((f(0) + f(2) + f(4)) / 3.0, (f(1) + f(3) + f(5)) / 3.0);
                    if !inside(cx, cy) {
                        continue;
                    }
                    for k in 0..3 {
                        out.extend_from_slice(&q(f(k * 2), corner[0]).to_le_bytes());
                        out.extend_from_slice(&q(f(k * 2 + 1), corner[1]).to_le_bytes());
                    }
                    n += 1;
                }
            }
            out[count_at..count_at + 4].copy_from_slice(&n.to_le_bytes());
        }
        let houses: Vec<&House> = self
            .houses
            .iter()
            .filter(|h| {
                let n = h.outline.len().max(1) as f32;
                let (sx, sy) = h.outline.iter().fold((0.0, 0.0), |(a, b), p| (a + p[0], b + p[1]));
                h.outline.len() >= 3 && h.outline.len() <= u16::MAX as usize && inside(sx / n, sy / n)
            })
            .collect();
        // the houses' models that fit the budget, nearest the square's middle first
        let mid = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
        let mut wanted: Vec<u32> = houses.iter().filter_map(|h| h.model).filter(|&i| (i as usize) < self.instances.len()).collect();
        wanted.sort_by(|a, b| {
            let d = |i: &u32| {
                let at = self.instances[*i as usize].at;
                (at[0] - mid[0]).hypot(at[1] - mid[1])
            };
            d(a).total_cmp(&d(b))
        });
        wanted.dedup();
        let mut drawn = std::collections::HashSet::new();
        let mut spent = 0;
        for i in wanted {
            let faces = self.models[self.instances[i as usize].model as usize].0.len();
            if low || spent + faces > MODEL_BUDGET || drawn.contains(&i) {
                continue;
            }
            spent += faces;
            drawn.insert(i);
        }
        out.extend_from_slice(&(houses.len() as u32).to_le_bytes());
        for h in houses {
            out.extend_from_slice(&(h.outline.len() as u16).to_le_bytes());
            for p in &h.outline {
                out.extend_from_slice(&q(p[0], corner[0]).to_le_bytes());
                out.extend_from_slice(&q(p[1], corner[1]).to_le_bytes());
            }
            let roof = &h.roof[..h.roof.len().min(u16::MAX as usize)];
            out.extend_from_slice(&(roof.len() as u16).to_le_bytes());
            for t in roof {
                t.iter().for_each(|i| out.extend_from_slice(&i.to_le_bytes()));
            }
            out.extend_from_slice(&((h.height * 10.0).round().clamp(0.0, u16::MAX as f32) as u16).to_le_bytes());
            let modelled = h.model.is_some_and(|i| drawn.contains(&i));
            out.push(h.open as u8 | (modelled as u8) << 1);
        }
        let trees: Vec<&[f32; 3]> = self.trees.iter().filter(|t| !low && inside(t[0], t[1])).collect();
        out.extend_from_slice(&(trees.len() as u32).to_le_bytes());
        for t in trees {
            out.extend_from_slice(&q(t[0], corner[0]).to_le_bytes());
            out.extend_from_slice(&q(t[1], corner[1]).to_le_bytes());
            out.extend_from_slice(&((t[2] as f64 * UNITS).round().clamp(0.0, i16::MAX as f64) as i16).to_le_bytes());
        }
        // (a line by the stretch of it within reach of the square: a river is long)
        let near = |p: &[f32; 2]| p[0] >= lo[0] - 50.0 && p[0] < hi[0] + 50.0 && p[1] >= lo[1] - 50.0 && p[1] < hi[1] + 50.0;
        let mut stretches: Vec<(&Line, &[[f32; 2]])> = Vec::new();
        for l in &self.lines {
            let mut i = 0;
            while i < l.points.len() {
                if !near(&l.points[i]) {
                    i += 1;
                    continue;
                }
                let start = i.saturating_sub(1);
                while i < l.points.len() && near(&l.points[i]) {
                    i += 1;
                }
                let end = (i + 1).min(l.points.len());
                if end - start >= 2 {
                    for part in l.points[start..end].chunks(u16::MAX as usize) {
                        stretches.push((l, part));
                    }
                }
            }
        }
        out.extend_from_slice(&(stretches.len() as u32).to_le_bytes());
        for (l, pts) in stretches {
            out.push(l.class);
            out.extend_from_slice(&((l.width as f64 * UNITS).round().clamp(0.0, u16::MAX as f64) as u16).to_le_bytes());
            out.extend_from_slice(&(pts.len() as u16).to_le_bytes());
            for p in pts {
                out.extend_from_slice(&q(p[0], corner[0]).to_le_bytes());
                out.extend_from_slice(&q(p[1], corner[1]).to_le_bytes());
            }
        }
        // the houses' models: each triangle placed, with its shade - a wall's light (0..127) as
        // it faces away from the house's middle, a roof's (128 and up) darker as it turns away
        let model_at = out.len();
        out.extend_from_slice(&0u32.to_le_bytes());
        let mut n = 0u32;
        let ll = SHADOW_DIR[0].hypot(SHADOW_DIR[1]);
        let light = [-SHADOW_DIR[0] / ll, -SHADOW_DIR[1] / ll];
        let mut ids: Vec<&u32> = drawn.iter().collect();
        ids.sort();
        // (each model's triangles placed, with the middle its walls face away from)
        let mut placed: Vec<(Vec<[[f32; 3]; 3]>, [f32; 3])> = Vec::new();
        for &i in ids {
            let it = self.instances[i as usize];
            let (faces, centre) = &self.models[it.model as usize];
            let a = -(it.heading as f64).to_radians();
            let (sn, cs) = (a.sin() as f32, a.cos() as f32);
            let place = |v: [f32; 3]| [v[0] * cs - v[1] * sn + it.at[0], v[0] * sn + v[1] * cs + it.at[1], v[2]];
            placed.push((faces.iter().map(|t| t.map(place)).collect(), place([centre[0], centre[1], 0.0])));
        }
        if !low {
            placed.extend(props.iter().filter(|(c, _)| inside(c[0], c[1])).map(|(c, faces)| (faces.clone(), [c[0], c[1], 0.0])));
        }
        for (faces, c) in &placed {
            for &p in faces {
                let (u, v) = ([p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]], [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]]);
                let mut nrm = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
                let len = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
                if len < 1e-9 {
                    continue;
                }
                nrm = nrm.map(|x| x / len);
                let flat = nrm[0].hypot(nrm[1]).max(1e-6);
                let facing = (nrm[0] * light[0] + nrm[1] * light[1]) / flat;
                let shade = if nrm[2].abs() < 0.35 {
                    let mid = [(p[0][0] + p[1][0] + p[2][0]) / 3.0 - c[0], (p[0][1] + p[1][1] + p[2][1]) / 3.0 - c[1]];
                    let out_ward = if nrm[0] * mid[0] + nrm[1] * mid[1] < 0.0 { -facing } else { facing };
                    ((0.5 + 0.5 * out_ward).clamp(0.0, 1.0) * 127.0).round() as u8
                } else {
                    let (nz, f) = if nrm[2] < 0.0 { (-nrm[2], -facing) } else { (nrm[2], facing) };
                    let away = (1.0 - nz) * (1.0 - (0.5 + 0.5 * f).clamp(0.0, 1.0));
                    128 | (((away * 0.9).clamp(0.0, 0.45) / 0.45 * 127.0).round() as u8)
                };
                for v in p {
                    out.extend_from_slice(&q(v[0], corner[0]).to_le_bytes());
                    out.extend_from_slice(&q(v[1], corner[1]).to_le_bytes());
                    out.extend_from_slice(&((v[2] as f64 * UNITS).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16).to_le_bytes());
                }
                out.push(shade);
                n += 1;
            }
        }
        out[model_at..model_at + 4].copy_from_slice(&n.to_le_bytes());
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `OMML` file as openOMSI's `Layers::to_bytes` writes it.
    fn omml(origin: [f64; 2], land_water: &[[f32; 2]], house: &[[f32; 2]], tree: [f32; 3]) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(MAGIC);
        w.extend_from_slice(&VERSION.to_le_bytes());
        w.extend_from_slice(&7u64.to_le_bytes());
        origin.iter().for_each(|v| w.extend_from_slice(&v.to_le_bytes()));
        let u32_ = |w: &mut Vec<u8>, v: usize| w.extend_from_slice(&(v as u32).to_le_bytes());
        let pts = |w: &mut Vec<u8>, p: &[[f32; 2]]| {
            u32_(w, p.len());
            p.iter().for_each(|q| q.iter().for_each(|v| w.extend_from_slice(&v.to_le_bytes())));
        };
        u32_(&mut w, 1);
        [0.0f32, 0.0, 300.0, 300.0].iter().for_each(|v| w.extend_from_slice(&v.to_le_bytes()));
        for classes in CLASSES {
            u32_(&mut w, classes);
            for class in 0..classes {
                pts(&mut w, if classes == 6 && class == 5 { land_water } else { &[] });
            }
        }
        u32_(&mut w, 1);
        pts(&mut w, house);
        u32_(&mut w, 2);
        [0u16, 1, 2, 0, 2, 3].iter().for_each(|i| w.extend_from_slice(&i.to_le_bytes()));
        w.extend_from_slice(&12.5f32.to_le_bytes());
        w.push(0);
        u32_(&mut w, u32::MAX as usize);
        // a model of one face and one instance of it: read past
        u32_(&mut w, 1);
        u32_(&mut w, 1);
        w.extend(std::iter::repeat_n(0u8, 36));
        u32_(&mut w, 1);
        w.extend(std::iter::repeat_n(0u8, 16));
        u32_(&mut w, 1);
        tree.iter().for_each(|v| w.extend_from_slice(&v.to_le_bytes()));
        u32_(&mut w, 1);
        w.push(2);
        w.extend_from_slice(&3.0f32.to_le_bytes());
        pts(&mut w, &[[-100.0, 10.0], [100.0, 10.0], [700.0, 10.0], [1200.0, 10.0]]);
        w
    }

    fn i16_at(b: &[u8], at: usize) -> i16 {
        i16::from_le_bytes([b[at], b[at + 1]])
    }
    fn u32_at(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn a_square_holds_what_lies_in_it() {
        let dir = std::env::temp_dir().join(format!("hub-ground-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("map.bin");
        // two triangles of water: one in the first square, one in the next
        let water = [[10.0, 10.0], [40.0, 10.0], [10.0, 40.0], [610.0, 10.0], [640.0, 10.0], [610.0, 40.0]];
        let house = [[100.0, 100.0], [120.0, 100.0], [120.0, 110.0], [100.0, 110.0]];
        std::fs::write(&path, omml([1000.0, -500.0], &water, &house, [200.0, 250.0, 3.0])).unwrap();
        let g = Ground::open(&path).expect("read");
        assert_eq!(g.origin, [1000.0, -500.0]);
        assert_eq!(g.houses.len(), 1);
        assert_eq!(g.houses[0].height, 12.5);
        assert_eq!(g.trees, vec![[200.0, 250.0, 3.0]]);
        assert!(!g.stale());

        let b = g.square([0.0, 0.0], false).unwrap();
        assert_eq!(&b[..4], b"OHG1");
        let mut at = 8;
        let mut tris = Vec::new();
        for _ in 0..ALL {
            let n = u32_at(&b, at) as usize;
            tris.push(n);
            at += 4 + n * 12;
        }
        assert_eq!(tris, [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]);
        // the water's first point, at sixteenths of a metre
        let water_at = 8 + 5 * 4 + 4;
        assert_eq!((i16_at(&b, water_at), i16_at(&b, water_at + 2)), (160, 160));
        assert_eq!(u32_at(&b, at), 1);
        assert_eq!(u16::from_le_bytes([b[at + 4], b[at + 5]]), 4);
        // the next square has the other triangle and neither house nor tree
        let next = g.square([500.0, 0.0], false).unwrap();
        let land_water = 8 + 5 * 4;
        assert_eq!(u32_at(&next, land_water), 1);
        assert_eq!(i16_at(&next, land_water + 4), 110 * 16);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn another_version_or_a_short_file_is_not_read() {
        let dir = std::env::temp_dir().join(format!("hub-ground-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("map.bin");
        let mut b = omml([0.0, 0.0], &[], &[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], [0.0, 0.0, 1.0]);
        let good = b.clone();
        b[4] = 18;
        std::fs::write(&path, &b).unwrap();
        assert!(Ground::open(&path).is_none());
        std::fs::write(&path, &good[..good.len() - 5]).unwrap();
        assert!(Ground::open(&path).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_cache_is_found_by_the_maps_folder() {
        let f = cache_file("maps\\Ahlheim 5\\global.cfg").unwrap();
        assert!(f.ends_with(Path::new(".openomsi/cache/mapgen/Ahlheim_5.bin")), "{f:?}");
        assert_eq!(cache_file("maps/Grundorf").unwrap().file_name().unwrap(), "Grundorf.bin");
    }

    /// The real thing, where a game left it: `cargo test -p hub-companion -- --ignored ground`.
    #[test]
    #[ignore]
    fn ground_of_a_real_map() {
        let Some(file) = cache_file("maps/Ahlheim_5/global.cfg").filter(|f| f.is_file()) else { return };
        let t = std::time::Instant::now();
        let g = Ground::open(&file).expect("the game's cache");
        println!("read in {:?}: {} houses, {} trees, {} lines", t.elapsed(), g.houses.len(), g.trees.len(), g.lines.len());
        let mid = g.houses[g.houses.len() / 2].outline[0];
        let corner = [(mid[0] as f64 / SQUARE).floor() * SQUARE, (mid[1] as f64 / SQUARE).floor() * SQUARE];
        let t = std::time::Instant::now();
        let b = g.square(corner, false).unwrap();
        println!("a square in {:?}: {} KB ({} KB far off)", t.elapsed(), b.len() / 1024, g.square(corner, true).unwrap().len() / 1024);
        // (for the app's mock server: `HUB_GROUND_DUMP=<dir>` writes the squares around the one
        // with the most houses as <i>_<j>.bin, i from -3 to 2 and j from -3 to 3 about it)
        let Some(dir) = std::env::var_os("HUB_GROUND_DUMP").map(PathBuf::from) else { return };
        let mut count: std::collections::HashMap<(i64, i64), usize> = std::collections::HashMap::new();
        for h in &g.houses {
            let p = h.outline[0];
            *count.entry(((p[0] as f64 / SQUARE).floor() as i64, (p[1] as f64 / SQUARE).floor() as i64)).or_default() += 1;
        }
        let (&(ci, cj), _) = count.iter().max_by_key(|(_, n)| **n).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        for i in -3..=2 {
            for j in -3..=3 {
                let corner = [(ci + i) as f64 * SQUARE, (cj + j) as f64 * SQUARE];
                std::fs::write(dir.join(format!("{i}_{j}.bin")), g.square(corner, false).unwrap()).unwrap();
            }
        }
        println!("dumped about square {ci},{cj} to {dir:?}");
    }
}
