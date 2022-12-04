use cssparser::{ParserInput, Parser};


pub fn parse_from_str(input: &str) {
    let mut input = ParserInput::new(input);
    let parser = Parser::new(&mut input);

    // parser.parse_entirely(|f| {

    // });
}