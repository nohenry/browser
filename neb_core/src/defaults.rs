
pub const SCALE: f32 = 2.0;

pub const TEXT_SIZE: f32 = 24.0;
pub const DOCUMENT_PADDING: f32 = 8.0;

#[macro_export]
macro_rules! psize {
    ($e:expr) => {{
        $e  * $crate::defaults::SCALE
    }};
}