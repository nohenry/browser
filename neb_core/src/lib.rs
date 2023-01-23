#![feature(iter_intersperse)]

pub mod dom_parser;

pub mod tree_display;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub use neb_graphics as gfx;

pub mod node;

pub mod defaults;

mod ids;

pub mod styling;

mod rectr;

mod svg;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}

fn calculate_hash<T>(t: &T, state: &mut DefaultHasher) -> u64
where
    T: Hash,
{
    t.hash(state);
    state.finish()
}
