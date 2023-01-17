use std::{fs::File, io::BufReader};

use neb_core::{
    defaults,
    dom_parser::parse_from_stream,
    gfx::vello::{kurbo::Rect, peniko::Color},
    psize,
};

fn main() {
    env_logger::init();
    let file = File::open("text.smf").unwrap();
    let file = BufReader::new(file);

    let document = parse_from_stream(file);

    let errors = document.get_errors();
    if errors.len() > 0 {
        for e in errors {
            println!("{}", e)
        }
        return;
    };

    pollster::block_on(neb_core::gfx::start_graphics_thread(move |builder| {
        {
            let body = document.get_body();
            let body = body.borrow();

            body.get_element()
                .layout(&body, Rect::from_origin_size((0.0, 0.0), builder.size), 0, &document);
        }

        document.draw(builder)
    }))
    .unwrap();
}
