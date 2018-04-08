//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{ftutils, hbutils};
use attrtext::FontStyle;
use harfbuzz;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Maintains a set of fonts and their associated font style values which are
/// used to determine the optimal font for rendering given characters.
///
/// `FontConfig` is not `Sync` because FreeType's `FT_Face` is not thread-safe.
///
/// Font selection is done as following:
///
///  1. If the input character is a control character, then just return the last
///     result.
///
///  2. The preferred font family list specified in the character style is
///     iterated through.
///     For each font family, the score for each font face is calculated from
///     the difference of the font face's style properties and ones specified
///     by the character style, and a font face with the lowest score is
///     selected.
///     If the font face contains a glyph for the input character, the algorithm
///     stops there and returns the font face.
///
///  3. The previous step is repeated, but this time, every font family
///     registered to the `FontConfig` participates in the selection process.
///     Font families are iterated in the reverse order of that in which they
///     are added to the `FontConfig` for the first time.
///
/// This algorithm has some quirks:
///
///  - All font faces belonging to one particular font family are assumed to
///    have the identical set of supported characters.
///
#[derive(Debug)]
pub struct FontConfig {
    /// A table used to look up a font family.
    familiy_names: HashMap<String, FontFamilyId>,
    /// A list of font families, indexed by `FontFamilyId`.
    families: Vec<FontFamily>,
    /// A list of font faces, indexed by `FontFaceId`.
    faces: Vec<FontFace>,
    /// Caches the result of the last step of the font selection algorithm.
    /// Wrapped by `RefCell` as it's updated dynamically.
    fallback_cache: RefCell<HashMap<char, Option<FontFamilyId>>>,
}

pub(crate) type FontFaceId = usize;

type FontFamilyId = usize;

#[derive(Debug)]
struct FontFamily {
    faces: Vec<FontFaceId>,
}

#[derive(Debug)]
pub(crate) struct FontFace {
    props: FontFaceProps,
    ft_face: ftutils::MemoryFace,
    hb_font: hbutils::Font,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FontFaceProps {
    weight: u16,
    style: FontStyle,
}

impl FontConfig {
    /// Construct an empty `FontConfig`.
    pub fn new() -> Self {
        Self {
            familiy_names: HashMap::new(),
            families: Vec::new(),
            faces: Vec::new(),
            fallback_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Insert a font.
    pub fn insert(
        &mut self,
        font: &Font,
        face_index: usize,
        family: &str,
        style: FontStyle,
        weight: u16,
    ) {
        // Create a FreeType face object
        assert!(face_index < font.num_faces());
        let ft_face = ftutils::Library::global()
            .new_memory_face(font.data.clone(), face_index as i32)
            .unwrap();

        // Create a HarfBuzz font object
        // `font.data.as_slice()` is guaranteed to be pinned because it's pinned
        // by `ft_face.1`
        let hb_blob = unsafe { hbutils::Blob::new_pinned(font.data.as_slice()) };
        let hb_face = hbutils::Face::new(&hb_blob, face_index as u32);
        let hb_font = hbutils::Font::new(&hb_face);

        // Allocate a font family ID (if it's new)
        let family_id = {
            let ref mut families = self.families;
            *self.familiy_names
                .entry(family.to_owned())
                .or_insert_with(|| {
                    let id = families.len();
                    families.push(FontFamily { faces: Vec::new() });
                    id
                })
        };

        let face_id = self.faces.len();

        self.faces.push(FontFace {
            props: FontFaceProps { weight, style },
            ft_face,
            hb_font,
        });

        self.families[family_id].faces.push(face_id);

        // Clear the fallback cache
        self.fallback_cache.get_mut().clear();
    }

    /// Construct a `FontSelector`.
    pub(crate) fn selector(&self) -> FontSelector {
        FontSelector {
            config: self,
            last_face_id: None,
            families: Vec::with_capacity(8),
        }
    }

    pub(crate) fn font_face(&self, id: FontFaceId) -> &FontFace {
        &self.faces[id]
    }
}

/// Stateful font selector used to determine the optimal font for characters.
#[derive(Debug)]
pub(crate) struct FontSelector<'a> {
    config: &'a FontConfig,
    last_face_id: Option<FontFaceId>,
    families: Vec<FontFamilyId>,
}

impl<'a> FontSelector<'a> {
    /// Set the list of the preferred font families used during the second step of the
    /// font selection algorithm.
    pub fn set_font_families(&mut self, family_names: &[String]) {
        let ref global_family_names = self.config.familiy_names;

        self.families.clear();
        self.families.extend(
            family_names
                .iter()
                .filter_map(|name| global_family_names.get(name).cloned()),
        );
    }

    pub fn optimal_font_face(&mut self, x: char, props: &FontFaceProps) -> Option<FontFaceId> {
        if x.is_control() {
            return self.last_face_id;
        }

        macro_rules! try_family {
            ($family:expr, $on_success:expr) => {{
                if let Some((face_id, face)) = $family.choose_face(props, &self.config.faces) {
                    if face.contains(x) {
                        self.last_face_id = Some(face_id);
                        $on_success;
                        return Some(face_id);
                    }
                }
            }};
        }

        // Iterate through the preferred font family list
        for &family_id in self.families.iter() {
            try_family!(self.config.families[family_id], {});
        }

        // Check the cache before doing the last step
        let mut fallback_cache = self.config.fallback_cache.borrow_mut();
        if let Some(&maybe_family_id) = fallback_cache.get(&x) {
            if let Some(family_id) = maybe_family_id {
                try_family!(self.config.families[family_id], {});
            } else {
                self.last_face_id = None;
                return None;
            }
        }

        // Iterate through the global font family list
        for (family_id, family) in self.config.families.iter().enumerate().rev() {
            try_family!(family, {
                fallback_cache.insert(x, Some(family_id));
            });
        }

        // We failed!
        fallback_cache.insert(x, None);
        self.last_face_id = None;
        None
    }
}

impl FontFamily {
    fn choose_face<'a>(
        &self,
        props: &FontFaceProps,
        faces: &'a [FontFace],
    ) -> Option<(FontFaceId, &'a FontFace)> {
        self.faces
            .iter()
            .map(|&face_id| (face_id, &faces[face_id]))
            .min_by_key(|&(_, face)| face.props.diff_score(props))
    }
}

impl FontFaceProps {
    fn diff_score(&self, other: &Self) -> u32 {
        let mut score = (self.weight as i32 - other.weight as i32).abs() as u32;
        if self.style != other.style {
            score += 10000;
        }
        score
    }
}

impl FontFace {
    fn contains(&self, x: char) -> bool {
        let mut glyph: harfbuzz::hb_codepoint_t = 0;
        unsafe {
            harfbuzz::RUST_hb_font_get_glyph(self.hb_font.get(), x as u32, 0, &mut glyph) != 0
        }
    }
}

#[derive(Debug)]
pub struct Font {
    data: Arc<Vec<u8>>,
    num_faces: usize,
}

impl Font {
    /// Construct a `Font` from a slice containing TrueType/OpenType data.
    pub fn new(data: &[u8]) -> Result<Self, String> {
        let num_faces;
        let data = Arc::new(Vec::from(data));
        {
            let face = ftutils::Library::global()
                .new_memory_face(data.clone(), -1)
                .map_err(|x| format!("FreeType error {:?}", x))?;
            num_faces = unsafe { *face.raw() }.num_faces as usize;
        }
        Ok(Font { data, num_faces })
    }

    pub fn num_faces(&self) -> usize {
        self.num_faces
    }
}
