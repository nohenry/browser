use std::{fs::File, io::BufReader};

use neb_core::dom_parser::parse_from_stream;

fn main() {
    let file = File::open("text.html").unwrap();
    let file = BufReader::new(file);

    parse_from_stream(file);
}
