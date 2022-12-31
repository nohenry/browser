use vello::{SceneBuilder, kurbo::Size};

use crate::simple_text::SimpleText;


pub struct DrawingContext<'a> {
    pub builder: SceneBuilder<'a>,
    pub text: SimpleText,
    pub size: Size
}