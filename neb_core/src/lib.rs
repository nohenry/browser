#![feature(iter_intersperse)]

pub mod dom_parser;

pub mod tree_display;

mod refs;
pub use refs::*;

pub use neb_graphics as gfx;

pub mod node;

pub mod defaults;

mod ids;

pub mod styling;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
    }
}
