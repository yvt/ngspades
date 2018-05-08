//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cggeom::Box2;
use ngscom::{
    hresults, to_hresult, BString, BStringRef, ComPtr, HResult, IAny, IUnknown, UnownedComPtr,
};
use rgb::RGBA;
use std::sync::{Arc, Mutex};

use canvas::text;
use ngsbase::{
    self, INgsPFCharStyle, INgsPFCharStyleTrait, INgsPFFont, INgsPFFontConfig,
    INgsPFFontConfigTrait, INgsPFFontFace, INgsPFFontFaceTrait, INgsPFFontFactory,
    INgsPFFontFactoryTrait, INgsPFFontTrait, INgsPFParagraphStyle, INgsPFParagraphStyleTrait,
    INgsPFTextLayout, INgsPFTextLayoutTrait,
};

com_impl! {
    /// An implementation of `INgsPFFontFactory`.
    class ComFontFactory {
        ings_pf_font_factory: INgsPFFontFactory;
        @data: ();
    }
}

impl ComFontFactory {
    pub fn new() -> ComPtr<IUnknown> {
        Self::alloc(())
    }
}

impl INgsPFFontFactoryTrait for ComFontFactory {
    fn create_char_style(&self, retval: &mut ComPtr<INgsPFCharStyle>) -> HResult {
        // Suppress the "field is never used" warning
        let () = self.data;

        *retval = (&ComCharStyle::new(text::CharStyle::default())).into();
        hresults::E_OK
    }

    fn create_font(&self, bytes: usize, length: i32, retval: &mut ComPtr<INgsPFFont>) -> HResult {
        to_hresult(|| {
            use std::slice::from_raw_parts;

            if length <= 0 {
                return Err(hresults::E_INVALIDARG);
            }

            let font = text::Font::new(unsafe {
                from_raw_parts(bytes as *const u8, length as usize)
            }).map_err(|_| hresults::E_UNEXPECTED)?;

            *retval = (&ComFont::new(font)).into();

            Ok(())
        })
    }

    fn create_font_config(&self, retval: &mut ComPtr<INgsPFFontConfig>) -> HResult {
        *retval = (&ComFontConfig::new(text::FontConfig::new())).into();
        hresults::E_OK
    }

    fn create_paragraph_style(&self, retval: &mut ComPtr<INgsPFParagraphStyle>) -> HResult {
        *retval = (&ComParagraphStyle::new(text::ParagraphStyle::new())).into();
        hresults::E_OK
    }

    fn get_language_from_iso639(
        &self,
        iso639: Option<&BString>,
        retval: &mut ComPtr<IUnknown>,
    ) -> HResult {
        to_hresult(|| {
            let name = iso639.ok_or(hresults::E_INVALIDARG)?;
            let lang = text::Language::from_iso639(name.as_str()).ok_or(hresults::E_INVALIDARG)?;
            *retval = ComLanguage::new(lang);
            Ok(())
        })
    }

    fn get_script_from_iso15924(
        &self,
        iso15924: Option<&BString>,
        retval: &mut ComPtr<IUnknown>,
    ) -> HResult {
        to_hresult(|| {
            let name = iso15924.ok_or(hresults::E_INVALIDARG)?;
            let lang = text::Script::from_iso15924(name.as_str()).ok_or(hresults::E_INVALIDARG)?;
            *retval = ComScript::new(lang);
            Ok(())
        })
    }
}

com_impl! {
    /// A COM wrapper for `ngspf::canvas::text::Language`.
    class ComLanguage {
        iany: IAny;
        @data: text::Language;
    }
}

impl ComLanguage {
    pub fn new(x: text::Language) -> ComPtr<IUnknown> {
        Self::alloc(x)
    }

    pub fn get(&self) -> text::Language {
        self.data
    }
}

com_impl! {
    /// A COM wrapper for `ngspf::canvas::text::Script`.
    class ComScript {
        iany: IAny;
        @data: text::Script;
    }
}

impl ComScript {
    pub fn new(x: text::Script) -> ComPtr<IUnknown> {
        Self::alloc(x)
    }

    pub fn get(&self) -> text::Script {
        self.data
    }
}

com_impl! {
    /// A COM wrapper for `ngspf::canvas::text::CharStyle`.
    class ComCharStyle {
        ings_pf_char_style: INgsPFCharStyle;
        iany: IAny;
        @data: Box<Fn(&mut FnMut(&mut text::CharStyle)) + Send + Sync>;//Mutex<text::CharStyle>;
    }
}

impl ComCharStyle {
    pub fn new(x: text::CharStyle) -> ComPtr<IUnknown> {
        let cell = Mutex::new(x);
        Self::alloc(Box::new(move |cb| {
            let mut guard = cell.lock().unwrap();
            cb(&mut guard);
        }))
    }

    /// Construct a `ComCharStyle`.
    fn with_proxy<F>(proxy: F) -> ComPtr<IUnknown>
    where
        F: Fn(&mut FnMut(&mut text::CharStyle)) + Send + Sync + 'static,
    {
        Self::alloc(Box::new(proxy))
    }

    /// Acquire a lock and call the given closure with the contained `CharStyle`.
    fn with<F: FnOnce(&mut text::CharStyle) -> R, R>(&self, f: F) -> R {
        let mut ret_cell = None;
        let mut f_cell = Some(f);
        (self.data)(&mut |style| {
            let f = f_cell.take().expect("callback was called twice");
            ret_cell = Some(f(style));
        });
        ret_cell.expect("callback was not called")
    }
}

impl INgsPFCharStyleTrait for ComCharStyle {
    fn get_color(&self, retval: &mut RGBA<f32>) -> HResult {
        use std::f32::NAN;
        self.with(|style| {
            *retval = (style.color).unwrap_or(RGBA::new(NAN, NAN, NAN, NAN));
        });
        hresults::E_OK
    }

    fn set_color(&self, value: RGBA<f32>) -> HResult {
        self.with(|style| {
            style.color = if value.a.is_nan() { None } else { Some(value) };
        });
        hresults::E_OK
    }

    fn get_font_families(&self, retval: &mut BStringRef) -> HResult {
        self.with(|style| {
            *retval = BStringRef::new(&style.font_family.join(", "));
        });
        hresults::E_OK
    }

    fn set_font_families(&self, value: Option<&BString>) -> HResult {
        self.with(|style| {
            style.font_family = if let Some(joined) = value {
                let joined = joined.as_str().trim();
                if joined.len() == 0 {
                    Vec::new()
                } else {
                    joined.split(',').map(|x| x.trim().to_owned()).collect()
                }
            } else {
                Vec::new()
            };
        });
        hresults::E_OK
    }

    fn get_font_size(&self, retval: &mut f64) -> HResult {
        self.with(|style| {
            *retval = (style.font_size).unwrap_or(::std::f64::NAN);
        });
        hresults::E_OK
    }

    fn set_font_size(&self, value: f64) -> HResult {
        self.with(|style| {
            style.font_size = if value.is_nan() { None } else { Some(value) };
        });
        hresults::E_OK
    }

    fn get_font_style(&self, retval: &mut ngsbase::FontStyle) -> HResult {
        self.with(|style| {
            *retval = match style.font_style {
                Some(text::FontStyle::Normal) => ngsbase::FontStyleItem::Normal,
                Some(text::FontStyle::Italic) => ngsbase::FontStyleItem::Italic,
                Some(text::FontStyle::Oblique) => ngsbase::FontStyleItem::Oblique,
                None => ngsbase::FontStyleItem::Inherited,
            }.into();
        });
        hresults::E_OK
    }

    fn set_font_style(&self, value: ngsbase::FontStyle) -> HResult {
        to_hresult(|| {
            let value = value.get().ok_or(hresults::E_INVALIDARG)?;
            self.with(|style| {
                style.font_style = match value {
                    ngsbase::FontStyleItem::Normal => Some(text::FontStyle::Normal),
                    ngsbase::FontStyleItem::Italic => Some(text::FontStyle::Italic),
                    ngsbase::FontStyleItem::Oblique => Some(text::FontStyle::Oblique),
                    ngsbase::FontStyleItem::Inherited => None,
                };
            });
            Ok(())
        })
    }

    fn get_font_weight(&self, retval: &mut i32) -> HResult {
        self.with(|style| {
            *retval = (style.font_weight).map(|x| x as i32).unwrap_or(0);
        });
        hresults::E_OK
    }

    fn set_font_weight(&self, value: i32) -> HResult {
        self.with(|style| {
            style.font_weight = if value == 0 { None } else { Some(value as u16) };
        });
        hresults::E_OK
    }

    fn get_language(&self, retval: &mut ComPtr<IUnknown>) -> HResult {
        self.with(|style| {
            *retval = if let Some(x) = style.language {
                ComLanguage::new(x)
            } else {
                ComPtr::null()
            };
        });
        hresults::E_OK
    }

    fn set_language(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            let new_value;
            if value.is_null() {
                new_value = None;
            } else {
                let value: ComPtr<IAny> = (&*value).into();
                let value = value.non_null().ok_or(hresults::E_UNEXPECTED)?;
                let value: &ComLanguage = value.downcast_ref().ok_or(hresults::E_UNEXPECTED)?;
                new_value = Some(value.get());
            }
            self.with(|style| {
                style.language = new_value;
            });
            Ok(())
        })
    }

    fn get_script(&self, retval: &mut ComPtr<IUnknown>) -> HResult {
        self.with(|style| {
            *retval = if let Some(x) = style.script {
                ComScript::new(x)
            } else {
                ComPtr::null()
            };
        });
        hresults::E_OK
    }

    fn set_script(&self, value: UnownedComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            let new_value;
            if value.is_null() {
                new_value = None;
            } else {
                let value: ComPtr<IAny> = (&*value).into();
                let value = value.non_null().ok_or(hresults::E_UNEXPECTED)?;
                let value: &ComScript = value.downcast_ref().ok_or(hresults::E_UNEXPECTED)?;
                new_value = Some(value.get());
            }
            self.with(|style| {
                style.script = new_value;
            });
            Ok(())
        })
    }

    fn get_text_decoration(&self, retval: &mut ngsbase::TextDecoration) -> HResult {
        *retval = if let Some(styles) = self.with(|style| style.text_decoration) {
            styles
                .iter()
                .fold(ngsbase::TextDecoration::empty(), |r, x| {
                    r | match x {
                        text::TextDecoration::Underline => ngsbase::TextDecorationItem::Underline,
                        text::TextDecoration::Overline => ngsbase::TextDecorationItem::Overline,
                        text::TextDecoration::Strikethrough => {
                            ngsbase::TextDecorationItem::Strikethrough
                        }
                    }
                })
        } else {
            ngsbase::TextDecorationItem::Inherited.into()
        };
        hresults::E_OK
    }

    fn set_text_decoration(&self, value: ngsbase::TextDecoration) -> HResult {
        let translated;

        if value.contains(ngsbase::TextDecorationItem::Inherited) {
            translated = None;
        } else {
            translated = Some(
                value
                    .iter()
                    .fold(text::TextDecorationFlags::empty(), |r, x| {
                        r | match x {
                            ngsbase::TextDecorationItem::Underline => {
                                text::TextDecoration::Underline
                            }
                            ngsbase::TextDecorationItem::Overline => text::TextDecoration::Overline,
                            ngsbase::TextDecorationItem::Strikethrough => {
                                text::TextDecoration::Strikethrough
                            }
                            ngsbase::TextDecorationItem::Inherited => unreachable!(),
                        }
                    }),
            );
        }

        self.with(|style| {
            style.text_decoration = translated;
        });

        hresults::E_OK
    }
}

com_impl! {
    /// A COM wrapper for `ngspf::canvas::text::ParagraphStyle`.
    class ComParagraphStyle {
        ings_pf_paragraph_style: INgsPFParagraphStyle;
        iany: IAny;
        @data: Arc<Mutex<text::ParagraphStyle>>;
    }
}

impl ComParagraphStyle {
    pub fn new(x: text::ParagraphStyle) -> ComPtr<IUnknown> {
        Self::alloc(Arc::new(Mutex::new(x)))
    }

    fn with<F: FnOnce(&mut text::ParagraphStyle) -> R, R>(&self, f: F) -> R {
        let mut guard = self.data.lock().unwrap();
        f(&mut guard)
    }
}

impl INgsPFParagraphStyleTrait for ComParagraphStyle {
    fn get_char_style(&self, retval: &mut ComPtr<INgsPFCharStyle>) -> HResult {
        let data = Arc::clone(&self.data);
        *retval = (&ComCharStyle::with_proxy(move |cb| {
            let mut lock = data.lock().unwrap();
            cb(&mut lock.char_style);
        })).into();
        hresults::E_OK
    }

    fn get_line_height_factor(&self, retval: &mut f32) -> HResult {
        self.with(|style| {
            *retval = style.line_height.factor as f32;
        });
        hresults::E_OK
    }

    fn set_line_height_factor(&self, value: f32) -> HResult {
        self.with(|style| {
            style.line_height.factor = value as f64;
        });
        hresults::E_OK
    }

    fn get_minimum_line_height(&self, retval: &mut f32) -> HResult {
        self.with(|style| {
            *retval = style.line_height.minimum as f32;
        });
        hresults::E_OK
    }

    fn set_minimum_line_height(&self, value: f32) -> HResult {
        self.with(|style| {
            style.line_height.minimum = value as f64;
        });
        hresults::E_OK
    }

    fn get_text_align(&self, retval: &mut ngsbase::TextAlign) -> HResult {
        self.with(|style| {
            *retval = match style.text_align {
                text::TextAlign::Start => ngsbase::TextAlignItem::Start,
                text::TextAlign::Center => ngsbase::TextAlignItem::Center,
                text::TextAlign::End => ngsbase::TextAlignItem::End,
                text::TextAlign::Justify => ngsbase::TextAlignItem::Justify,
                text::TextAlign::JustifyAll => ngsbase::TextAlignItem::JustifyAll,
            }.into();
        });
        hresults::E_OK
    }

    fn set_text_align(&self, value: ngsbase::TextAlign) -> HResult {
        to_hresult(|| {
            let value = value.get().ok_or(hresults::E_INVALIDARG)?;
            self.with(|style| {
                style.text_align = match value {
                    ngsbase::TextAlignItem::Start => text::TextAlign::Start,
                    ngsbase::TextAlignItem::Center => text::TextAlign::Center,
                    ngsbase::TextAlignItem::End => text::TextAlign::End,
                    ngsbase::TextAlignItem::Justify => text::TextAlign::Justify,
                    ngsbase::TextAlignItem::JustifyAll => text::TextAlign::JustifyAll,
                };
            });
            Ok(())
        })
    }

    fn get_text_direction(&self, retval: &mut ngsbase::TextDirection) -> HResult {
        self.with(|style| {
            *retval = match style.direction {
                text::Direction::LeftToRight => ngsbase::TextDirectionItem::LeftToRight,
                text::Direction::RightToLeft => ngsbase::TextDirectionItem::RightToLeft,
                text::Direction::TopToBottom => ngsbase::TextDirectionItem::TopToBottom,
                text::Direction::BottomToTop => ngsbase::TextDirectionItem::BottomToTop,
            }.into();
        });
        hresults::E_OK
    }

    fn set_text_direction(&self, value: ngsbase::TextDirection) -> HResult {
        to_hresult(|| {
            let value = value.get().ok_or(hresults::E_INVALIDARG)?;
            self.with(|style| {
                style.direction = match value {
                    ngsbase::TextDirectionItem::LeftToRight => text::Direction::LeftToRight,
                    ngsbase::TextDirectionItem::RightToLeft => text::Direction::RightToLeft,
                    ngsbase::TextDirectionItem::TopToBottom => text::Direction::TopToBottom,
                    ngsbase::TextDirectionItem::BottomToTop => text::Direction::BottomToTop,
                };
            });
            Ok(())
        })
    }

    fn get_word_wrap_mode(&self, retval: &mut ngsbase::WordWrapMode) -> HResult {
        self.with(|style| {
            *retval = match style.word_wrap_mode {
                text::WordWrapMode::MinNumLines => ngsbase::WordWrapModeItem::MinNumLines,
                text::WordWrapMode::MinRaggedness => ngsbase::WordWrapModeItem::MinRaggedness,
            }.into();
        });
        hresults::E_OK
    }

    fn set_word_wrap_mode(&self, value: ngsbase::WordWrapMode) -> HResult {
        to_hresult(|| {
            let value = value.get().ok_or(hresults::E_INVALIDARG)?;
            self.with(|style| {
                style.word_wrap_mode = match value {
                    ngsbase::WordWrapModeItem::MinNumLines => text::WordWrapMode::MinNumLines,
                    ngsbase::WordWrapModeItem::MinRaggedness => text::WordWrapMode::MinRaggedness,
                };
            });
            Ok(())
        })
    }
}

com_impl! {
    /// A COM wrapper for `ngspf::canvas::text::Font`. Retains a set of
    /// `ComFontFace`s each of which refers to a font face within a font object.
    class ComFont {
        ings_pf_font: INgsPFFont;
        iany: IAny;
        @data: Vec<ComPtr<INgsPFFontFace>>;
    }
}

impl ComFont {
    pub fn new(font: text::Font) -> ComPtr<IUnknown> {
        let font = Arc::new(font);
        let faces = (0..font.num_faces())
            .map(|i| (&ComFontFace::new(font.clone(), i)).into())
            .collect();
        Self::alloc(faces)
    }
}

impl INgsPFFontTrait for ComFont {
    fn get_num_font_faces(&self, retval: &mut i32) -> HResult {
        *retval = self.data.len() as i32;
        hresults::E_OK
    }

    fn get_font_face(&self, index: i32, retval: &mut ComPtr<INgsPFFontFace>) -> HResult {
        to_hresult(|| {
            let index = index as usize;
            *retval = self.data.get(index).ok_or(hresults::E_INVALIDARG)?.clone();
            Ok(())
        })
    }
}

com_impl! {
    /// A COM class that holds a reference to `ngspf::canvas::text::Font` and
    /// a font face index into it.
    class ComFontFace {
        ings_pf_font_face: INgsPFFontFace;
        iany: IAny;
        @data: (usize, Arc<text::Font>);
    }
}

impl ComFontFace {
    fn new(font: Arc<text::Font>, face_index: usize) -> ComPtr<IUnknown> {
        Self::alloc((face_index, font))
    }

    /// Get the `Font` referenced by a `ComFontFace`.
    pub fn font(&self) -> &text::Font {
        &self.data.1
    }

    /// Get a face index within `font()`.
    pub fn face_index(&self) -> usize {
        self.data.0
    }
}

impl INgsPFFontFaceTrait for ComFontFace {}

com_impl! {
    /// A COM wrapper for `ngspf::canvas::text::FontConfig`.
    class ComFontConfig {
        ings_pf_font_config: INgsPFFontConfig;
        iany: IAny;
        @data: Arc<Mutex<text::FontConfig>>;
    }
}

impl ComFontConfig {
    pub fn new(x: text::FontConfig) -> ComPtr<IUnknown> {
        Self::alloc(Arc::new(Mutex::new(x)))
    }

    fn with<F: FnOnce(&mut text::FontConfig) -> R, R>(&self, f: F) -> R {
        let mut guard = self.data.lock().unwrap();
        f(&mut guard)
    }
}

impl INgsPFFontConfigTrait for ComFontConfig {
    fn add_font_face(
        &self,
        font_face: UnownedComPtr<INgsPFFontFace>,
        font_family: Option<&BString>,
        font_style: ngsbase::FontStyle,
        weight: i32,
    ) -> HResult {
        to_hresult(|| {
            let face: ComPtr<IAny> = (&*font_face).into();
            let face = face.non_null().ok_or(hresults::E_UNEXPECTED)?;
            let face: &ComFontFace = face.downcast_ref().ok_or(hresults::E_UNEXPECTED)?;

            let family = font_family.ok_or(hresults::E_POINTER)?.as_str();

            let style = font_style.get().ok_or(hresults::E_INVALIDARG)?;
            let style = match style {
                ngsbase::FontStyleItem::Normal => text::FontStyle::Normal,
                ngsbase::FontStyleItem::Italic => text::FontStyle::Italic,
                ngsbase::FontStyleItem::Oblique => text::FontStyle::Oblique,
                ngsbase::FontStyleItem::Inherited => return Err(hresults::E_INVALIDARG),
            };

            self.with(|config| {
                config.insert(face.font(), face.face_index(), family, style, weight as u16);
            });

            Ok(())
        })
    }

    fn layout_box_area_string(
        &self,
        text: Option<&BString>,
        paragraph_style: UnownedComPtr<INgsPFParagraphStyle>,
        width: f32,
        retval: &mut ComPtr<INgsPFTextLayout>,
    ) -> HResult {
        to_hresult(|| {
            let style: ComPtr<IAny> = (&*paragraph_style).into();
            let style = style.non_null().ok_or(hresults::E_UNEXPECTED)?;
            let style: &ComParagraphStyle = style.downcast_ref().ok_or(hresults::E_UNEXPECTED)?;

            let text = text.ok_or(hresults::E_POINTER)?.as_str();

            let boundary = text::BoxBoundary::new(0.0..width as f64);

            let layout;
            {
                let config = self.data.lock().unwrap();
                layout = style.with(|style| {
                    config.layout_area_text([(text, ())][..].into(), style, &boundary)
                });
            }

            *retval = (&ComTextLayout::new(layout, Arc::clone(&self.data))).into();

            Ok(())
        })
    }

    fn layout_point_string(
        &self,
        text: Option<&BString>,
        paragraph_style: UnownedComPtr<INgsPFParagraphStyle>,
        retval: &mut ComPtr<INgsPFTextLayout>,
    ) -> HResult {
        to_hresult(|| {
            let style: ComPtr<IAny> = (&*paragraph_style).into();
            let style = style.non_null().ok_or(hresults::E_UNEXPECTED)?;
            let style: &ComParagraphStyle = style.downcast_ref().ok_or(hresults::E_UNEXPECTED)?;

            let text = text.ok_or(hresults::E_POINTER)?.as_str();

            let layout;
            {
                let config = self.data.lock().unwrap();
                layout =
                    style.with(|style| config.layout_point_text([(text, ())][..].into(), style));
            }

            *retval = (&ComTextLayout::new(layout, Arc::clone(&self.data))).into();

            Ok(())
        })
    }
}

com_impl! {
    class ComTextLayout {
        ings_pf_text_layout: INgsPFTextLayout;
        iany: IAny;
        @data: (Mutex<text::TextLayout>, Arc<Mutex<text::FontConfig>>);
    }
}

impl ComTextLayout {
    fn new(layout: text::TextLayout, config: Arc<Mutex<text::FontConfig>>) -> ComPtr<IUnknown> {
        Self::alloc((Mutex::new(layout), config))
    }

    pub(crate) fn with_layout_config<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&text::TextLayout, &text::FontConfig) -> R,
    {
        let layout = self.data.0.lock().unwrap();
        let config = self.data.1.lock().unwrap();
        f(&layout, &config)
    }
}

impl INgsPFTextLayoutTrait for ComTextLayout {
    fn get_visual_bounds(&self, retval: &mut Box2<f32>) -> HResult {
        let layout = self.data.0.lock().unwrap();
        let bounds = layout.visual_bounds();
        retval.min = bounds[0].cast();
        retval.max = bounds[1].cast();
        hresults::E_OK
    }
}
