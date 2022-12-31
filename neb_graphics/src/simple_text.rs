use vello::glyph::{pinot, pinot::TableProvider, GlyphContext};
use vello::kurbo::{Affine, Rect, Size};
use vello::{peniko::Brush, SceneBuilder};

pub use pinot::FontRef;

// This is very much a hack to get things working.
// On Windows, can set this to "c:\\Windows\\Fonts\\seguiemj.ttf" to get color emoji
const FONT_DATA: &[u8] = include_bytes!("../../resources/Roboto/Roboto-Regular.ttf");

pub struct SimpleText {
    gcx: GlyphContext,
}

#[derive(Clone, Copy)]
pub enum TextAlign {
    Left,
    Right,

    Top,
    Bottom,

    Center,
}

impl SimpleText {
    pub fn new() -> Self {
        Self {
            gcx: GlyphContext::new(),
        }
    }

    pub fn layout(&mut self, font: Option<&FontRef>, size: f32, text: &str) -> Rect {
        let font = font.unwrap_or(&FontRef {
            data: FONT_DATA,
            offset: 0,
        });

        if let Some(cmap) = font.cmap() {
            if let Some(hmtx) = font.hmtx() {
                let upem = font.head().map(|head| head.units_per_em()).unwrap_or(1000) as f64;
                let scale = size as f64 / upem;
                let hmetrics = hmtx.hmetrics();

                let height = if let Some(h) = font.os2() {
                    h.typographic_ascender() as f64 * scale
                        + -h.typographic_descender() as f64 * scale
                        + h.typographic_line_gap() as f64 * scale
                } else {
                    size as f64
                };

                let default_hadvance = hmetrics
                    .get(hmetrics.len().saturating_sub(1))
                    .map(|h| h.advance_width)
                    .unwrap_or(0);

                let mut pen_x = 0f64;
                for ch in text.chars() {
                    let gid = cmap.map(ch as u32).unwrap_or(0);
                    let advance = hmetrics
                        .get(gid as usize)
                        .map(|h| h.advance_width)
                        .unwrap_or(default_hadvance) as f64
                        * scale;

                    pen_x += advance;
                }

                return Rect::new(0.0, 0.0, pen_x, height);
            }
        }
        Rect::ZERO
    }

    pub fn get_adg(&mut self, font: Option<&FontRef>, size: f32) -> (f64, f64, f64) {
        let font = font.unwrap_or(&FontRef {
            data: FONT_DATA,
            offset: 0,
        });

        let upem = font.head().map(|head| head.units_per_em()).unwrap_or(1000) as f64;
        let scale = size as f64 / upem;

        if let Some(h) = font.os2() {
            (
                h.typographic_ascender() as f64 * scale,
                -h.typographic_descender() as f64 * scale,
                h.typographic_line_gap() as f64 * scale,
            )
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    pub fn add(
        &mut self,
        builder: &mut SceneBuilder,
        _font: Option<&FontRef>,
        size: f32,
        vertical_align: Option<TextAlign>,
        horizontal_align: Option<TextAlign>,
        brush: Option<&Brush>,
        transform: Affine,
        text: &str,
    ) {
        let font = _font.unwrap_or(&FontRef {
            data: FONT_DATA,
            offset: 0,
        });
        if let Some(cmap) = font.cmap() {
            if let Some(hmtx) = font.hmtx() {
                let layout = self.layout(_font, size, text);
                let (ascent, descent, _gap) = self.get_adg(_font, size);

                let valign = vertical_align.unwrap_or(TextAlign::Top);
                let halign = horizontal_align.unwrap_or(TextAlign::Top);

                let upem = font.head().map(|head| head.units_per_em()).unwrap_or(1000) as f64;
                let scale = size as f64 / upem;
                let vars: [(pinot::types::Tag, f32); 0] = [];
                let mut provider = self.gcx.new_provider(font, None, size, false, vars);
                let hmetrics = hmtx.hmetrics();
                let default_advance = hmetrics
                    .get(hmetrics.len().saturating_sub(1))
                    .map(|h| h.advance_width)
                    .unwrap_or(0);
                let mut pen_x = 0f64;
                for ch in text.chars() {
                    let gid = cmap.map(ch as u32).unwrap_or(0);
                    let advance = hmetrics
                        .get(gid as usize)
                        .map(|h| h.advance_width)
                        .unwrap_or(default_advance) as f64
                        * scale;
                    if let Some(glyph) = provider.get(gid, brush) {
                        let xform = transform
                            * Affine::translate((pen_x, 0.0))
                            * transform_from_align(ascent, descent, layout.size(), valign, halign)
                            * Affine::scale_non_uniform(1.0, -1.0);
                        builder.append(&glyph, Some(xform));
                    }
                    pen_x += advance;
                }
            }
        }
    }
}

pub fn xy_from_align(
    ascent: f64,
    descent: f64,
    size: Size,
    vertical_align: TextAlign,
    horizontal_algin: TextAlign,
) -> (f64, f64) {
    let (mut x, mut y) = (0.0, 0.0);

    match vertical_align {
        TextAlign::Top => y += ascent,
        TextAlign::Center => y += (ascent + descent) / 2.0,
        _ => (),
    }

    match horizontal_algin {
        TextAlign::Right => x += -size.width,
        TextAlign::Center => x += -size.width / 2.0,
        _ => (),
    }

    (x, y)
}

pub fn transform_from_align(
    ascent: f64,
    descent: f64,
    size: Size,
    vertical_align: TextAlign,
    horizontal_algin: TextAlign,
) -> Affine {
    // let mut transform = Affine::IDENTITY;
    let (x, y) = xy_from_align(ascent, descent, size, vertical_align, horizontal_algin);

    Affine::translate((x, y))
}
