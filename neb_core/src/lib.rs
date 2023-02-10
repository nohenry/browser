#![feature(iter_intersperse)]

pub mod document;

pub use neb_graphics as gfx;

pub mod node;

pub mod defaults;

pub mod ids;

pub mod styling;

mod rectr;

// mod svg;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}