use neb_graphics::vello::peniko::Color;

use crate::styling::Direction;

pub const SCALE: f32 = 2.0;

pub const TEXT_SIZE: f32 = 24.0;
pub const FOREGROUND_COLOR: Color = Color::BLACK;
pub const DOCUMENT_PADDING: f32 = 8.0;
pub const GAP: f64 = 4.0;
pub const DIRECTION: Direction = Direction::Vertical;

#[macro_export]
macro_rules! psize {
    ($e:expr) => {{
        $e * $crate::defaults::SCALE
    }};
}
