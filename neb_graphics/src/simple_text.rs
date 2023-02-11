use vello::glyph::{pinot, pinot::TableProvider, GlyphContext};
use vello::kurbo::{Affine, Rect};
use vello::{peniko::Brush, SceneBuilder};

pub use pinot::FontRef;

// This is very much a hack to get things working.
// On Windows, can set this to "c:\\Windows\\Fonts\\seguiemj.ttf" to get color emoji
const FONT_DATA: &[u8] =
    include_bytes!("../../resources/Roboto_Mono/static/RobotoMono-Regular.ttf");

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

    pub fn layout(&mut self, font: Option<&FontRef>, size: f32, text: &str, bounds: &Rect) -> Rect {
        let font = font.unwrap_or(&FontRef {
            data: FONT_DATA,
            offset: 0,
        });

        if let Some(cmap) = font.cmap() {
            if let Some(hmtx) = font.hmtx() {
                let upem = font.head().map(|head| head.units_per_em()).unwrap_or(1000) as f64;
                let scale = size as f64 / upem;
                let hmetrics = hmtx.hmetrics();

                let height = if let Some(h) = font.hhea() {
                    h.ascender() as f64 * scale - h.descender() as f64 * scale
                        + h.line_gap() as f64 * scale
                } else {
                    size as f64
                }
                .ceil();

                let default_hadvance = hmetrics
                    .get(hmetrics.len().saturating_sub(1))
                    .map(|h| h.advance_width)
                    .unwrap_or(0);

                let mut words: Vec<_> = text
                    .split(' ')
                    .map(|f| {
                        f.chars().chain([' '].into_iter()).fold(0.0, |acc, b| {
                            acc + hmetrics
                                .get(cmap.map(b as u32).unwrap_or(0) as usize)
                                .map(|h| h.advance_width)
                                .unwrap_or(default_hadvance)
                                as f64
                                * scale
                        })
                    })
                    .chain([0.0].into_iter())
                    .collect();

                let mut pen_x = 0f64;
                let mut max_x = 0f64;
                let mut pen_y = 0f64;
                let mut word_index = 0;
                let mut overflow = false;

                for (ch, nxt) in text.chars().zip(text.chars()) {
                    let gid = cmap.map(ch as u32).unwrap_or(0);
                    let advance = hmetrics
                        .get(gid as usize)
                        .map(|h| h.advance_width)
                        .unwrap_or(default_hadvance) as f64
                        * scale;

                    // If overflow, go to next line
                    if pen_x + words[word_index + 1] > bounds.width() && ch == ' ' {
                        // if pen_x + advance > bounds.width() {
                        pen_x = 0.0;
                        pen_y += height;
                        overflow = true;
                    }

                    if ch == ' ' {
                        word_index += 1;
                    }

                    // If newline starts with space, don't add it
                    if ch == ' ' && pen_y > 0.0 && pen_x < 0.1 {
                        continue;
                    }

                    pen_x += advance.ceil();

                    if pen_x > max_x {
                        max_x = pen_x
                    }
                }

                if max_x > bounds.width() || overflow {
                    max_x = bounds.width();
                }
                return Rect::new(0.0, 0.0, max_x, pen_y + height);
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
        brush: Option<&Brush>,
        transform: Affine,
        text: &str,
        bounds: &Rect,
    ) {
        let font = _font.unwrap_or(&FontRef {
            data: FONT_DATA,
            offset: 0,
        });

        if let Some(cmap) = font.cmap() {
            if let Some(hmtx) = font.hmtx() {
                let upem = font.head().map(|head| head.units_per_em()).unwrap_or(1000) as f64;
                let scale = size as f64 / upem;

                let vars: [(pinot::types::Tag, f32); 0] = [];
                let mut provider = self.gcx.new_provider(font, None, size, false, vars);
                let hmetrics = hmtx.hmetrics();
                let default_advance = hmetrics
                    .get(hmetrics.len().saturating_sub(1))
                    .map(|h| h.advance_width)
                    .unwrap_or(0);

                let mut pen_x = 0.0f64;
                let mut pen_y = 0f64;

                let mut word_index = 0;
                // for text in words {
                //     println!("{}", text);
                // }

                let mut words: Vec<_> = text
                    .split(' ')
                    .map(|f| {
                        f.chars().chain([' '].into_iter()).fold(0.0, |acc, b| {
                            acc + hmetrics
                                .get(cmap.map(b as u32).unwrap_or(0) as usize)
                                .map(|h| h.advance_width)
                                .unwrap_or(default_advance) as f64
                                * scale
                        })
                    })
                    .chain([0.0].into_iter())
                    .collect();

                for ch in text.chars() {
                    let gid = cmap.map(ch as u32).unwrap_or(0);
                    let advance = hmetrics
                        .get(gid as usize)
                        .map(|h| h.advance_width)
                        .unwrap_or(default_advance) as f64
                        * scale;

                    if let Some(glyph) = provider.get(gid, brush) {
                        if pen_x + words[word_index + 1] > bounds.width() && ch == ' ' {
                            if let Some(vmtx) = font.hhea() {
                                let height = (vmtx.ascender() as f64 * scale
                                    - vmtx.descender() as f64 * scale
                                    + vmtx.line_gap() as f64);

                                pen_x = 0.0;
                                pen_y += height;
                            }
                        }

                        if ch == ' ' {
                            word_index += 1;
                        }
                        // Skip space on start of newline
                        if ch == ' ' && pen_y > 0.0 && pen_x < 0.1 {
                            continue;
                        }

                        let xform = transform
                            * Affine::translate((
                                pen_x,
                                (font.hhea().unwrap().ascender() as f64 * scale + pen_y).ceil(),
                            ))
                            * Affine::scale_non_uniform(1.0, -1.0);
                        builder.append(&glyph, Some(xform));
                    }

                    pen_x += advance.ceil();
                }
            }
        }
    }
}

pub fn xy_from_align(
    ascent: f64,
    descent: f64,
    size: Rect,
    vertical_align: TextAlign,
    horizontal_algin: TextAlign,
) -> (f64, f64) {
    let (mut x, mut y) = (0.0, 0.0);

    match vertical_align {
        // TextAlign::Top => y += size.max_y(),
        TextAlign::Center => y += (ascent + descent) / 2.0,
        _ => (),
    }

    match horizontal_algin {
        TextAlign::Right => x += -size.width(),
        TextAlign::Center => x += -size.width() / 2.0,
        _ => (),
    }

    (x, y)
}

pub fn transform_from_align(
    ascent: f64,
    descent: f64,
    size: Rect,
    vertical_align: TextAlign,
    horizontal_algin: TextAlign,
) -> Affine {
    // let mut transform = Affine::IDENTITY;
    let (x, y) = xy_from_align(ascent, descent, size, vertical_align, horizontal_algin);

    Affine::translate((x, y))
}
