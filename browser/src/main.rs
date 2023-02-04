use std::{fs::File, io::BufReader};

use neb_core::dom_parser::parse_from_stream;

fn main() {
    env_logger::init();
    let file = File::open("test_files/new.smf").unwrap();
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
        document.layout(builder.size.width, builder.size.height);

        document.draw(builder)
    }))
    .unwrap();
}
