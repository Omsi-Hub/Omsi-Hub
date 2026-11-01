//! The houses as the game dresses them, for the companion's 3D navigator: a house's own model
//! with its textures' coordinates and which of its looks each face wears (`Source::dressed`, read
//! from the map's objects by the app), sent with a square (`?dress=1`, see the protocol's
//! `GET /api/ground`) instead of the untextured model of the game's picture; the textures
//! themselves are fetched apart, once each, small ([`SKIN`] pixels a side: `GET /api/ground/skin`).

/// The side of a house's texture as the phone gets it (px): every one the same, a layer of the
/// phone's texture array.
pub const SKIN: u32 = 128;
/// A texture coordinate on the wire: so many to a repeat of the texture (i16, a face's
/// coordinates moved by whole repeats to start near 0: some 16 repeats either way).
pub const UV_UNITS: f32 = 2048.0;

/// A face of a dressed model: its corners (x, y, z up from the model's ground, as the game's
/// picture keeps its models), their texture coordinates, which of the model's looks it wears.
pub type Face = ([[f32; 3]; 3], [[f32; 2]; 3], u16);

/// A dressed model as a square places it: its faces, its looks, the middle its walls face away from.
pub type Placed = (Vec<Face>, Vec<Look>, [f32; 3]);

/// What dresses a square's houses: a placed model's own, dressed (`Source::dressed`), or None.
pub type Dresser<'a> = &'a dyn Fn(&crate::ground::Instance) -> Option<std::sync::Arc<Dressed>>;

/// A face's shade as a square gives it (`ground`): the face placed, its house's middle.
pub type Shade<'a> = &'a dyn Fn(&[[f32; 3]; 3], [f32; 3]) -> Option<u8>;

/// A house's model, dressed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dressed {
    pub faces: Vec<Face>,
    pub looks: Vec<Look>,
}

/// What a face wears: a texture (its id for `GET /api/ground/skin`, 0: none - the colour only)
/// and the material's colour; `cut`: the texture's clear parts are holes (a fence, leaves).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Look {
    pub skin: u32,
    pub colour: [u8; 4],
    pub cut: bool,
}

/// The dressed models' section of a square: `SKN1`, the looks the square's faces wear, then the
/// faces (placed, at `units` a metre from `corner`) with their texture coordinates, their look
/// and their shade (`shade_of`).
pub fn write_section(out: &mut Vec<u8>, placed: &[Placed], corner: [f64; 2], units: f64, shade_of: Shade) {
    out.extend_from_slice(b"SKN1");
    // (the square's looks, each once)
    let mut looks: Vec<Look> = Vec::new();
    let mut index = std::collections::HashMap::new();
    let mapped: Vec<Vec<u16>> = placed
        .iter()
        .map(|(_, ls, _)| {
            ls.iter()
                .map(|l| {
                    *index.entry(*l).or_insert_with(|| {
                        looks.push(*l);
                        (looks.len() - 1).min(u16::MAX as usize) as u16
                    })
                })
                .collect()
        })
        .collect();
    out.extend_from_slice(&(looks.len().min(u16::MAX as usize) as u16).to_le_bytes());
    for l in looks.iter().take(u16::MAX as usize) {
        out.extend_from_slice(&l.skin.to_le_bytes());
        out.extend_from_slice(&l.colour);
        out.push(l.cut as u8);
    }
    let count_at = out.len();
    out.extend_from_slice(&0u32.to_le_bytes());
    let q = |v: f32, c: f64| ((v as f64 - c) * units).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16;
    let mut n = 0u32;
    for ((faces, _, centre), map) in placed.iter().zip(&mapped) {
        for (p, uv, look) in faces {
            let Some(shade) = shade_of(p, *centre) else { continue };
            for v in p {
                out.extend_from_slice(&q(v[0], corner[0]).to_le_bytes());
                out.extend_from_slice(&q(v[1], corner[1]).to_le_bytes());
                out.extend_from_slice(&((v[2] as f64 * units).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16).to_le_bytes());
            }
            // (whole repeats off: the texture repeats, the numbers stay small)
            let (du, dv) = (uv.iter().map(|t| t[0]).fold(f32::MAX, f32::min).floor(), uv.iter().map(|t| t[1]).fold(f32::MAX, f32::min).floor());
            for t in uv {
                for w in [t[0] - du, t[1] - dv] {
                    out.extend_from_slice(&((w * UV_UNITS).round().clamp(i16::MIN as f32, i16::MAX as f32) as i16).to_le_bytes());
                }
            }
            out.extend_from_slice(&map.get(*look as usize).copied().unwrap_or(0).to_le_bytes());
            out.push(shade);
            n += 1;
        }
    }
    out[count_at..count_at + 4].copy_from_slice(&n.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dressed_face_goes_with_its_look_and_where_on_its_texture() {
        let look = Look { skin: 7, colour: [255; 4], cut: false };
        let plain = Look { skin: 0, colour: [200, 10, 10, 255], cut: false };
        // a wall 3 m wide and 2 m high at 10 m east of the corner, its texture repeated, and a face
        // of no size (left out)
        let faces = vec![([[10.0, 5.0, 0.0], [13.0, 5.0, 0.0], [13.0, 5.0, 2.0]], [[2.0, 3.5], [3.5, 3.5], [3.5, 2.25]], 1), ([[0.0; 3]; 3], [[0.0; 2]; 3], 0)];
        let mut out = Vec::new();
        write_section(&mut out, &[(faces, vec![plain, look], [11.5, 9.0, 0.0])], [0.0, 0.0], 16.0, &|p: &[[f32; 3]; 3], _| (p[0] != p[1]).then_some(42));
        assert_eq!(&out[..4], b"SKN1");
        assert_eq!(u16::from_le_bytes([out[4], out[5]]), 2);
        // the looks as the faces met them: the plain one, then texture 7
        assert_eq!(u32::from_le_bytes(out[6..10].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(out[15..19].try_into().unwrap()), 7);
        let at = 6 + 2 * 9;
        assert_eq!(u32::from_le_bytes(out[at..at + 4].try_into().unwrap()), 1);
        let f = at + 4;
        let i16_at = |k: usize| i16::from_le_bytes([out[f + k], out[f + k + 1]]);
        assert_eq!((i16_at(0), i16_at(2), i16_at(4)), (160, 80, 0));
        // (whole repeats off: 2 and 2, at 2048 a repeat)
        assert_eq!((i16_at(18), i16_at(20), i16_at(28)), (0, (1.5 * UV_UNITS) as i16, (0.25 * UV_UNITS) as i16));
        assert_eq!(u16::from_le_bytes([out[f + 30], out[f + 31]]), 1);
        assert_eq!(out[f + 32], 42);
        assert_eq!(out.len(), f + 33);
    }
}
