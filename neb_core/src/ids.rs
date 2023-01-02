use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use neb_graphics::vello::kurbo::Rect;

lazy_static::lazy_static! {
    pub(crate) static ref ID_MANAGER: Mutex<IDManager> = {
        Mutex::new(IDManager { id_mappings: HashMap::new(), next_id: rand::random() })
    };
}

pub(crate) fn get_id_mgr() -> MutexGuard<'static, IDManager> {
    ID_MANAGER.lock().unwrap()
}

pub type ID = u64;

#[derive(Debug)]
pub struct Layout {
    pub content_rect: Rect,
    pub border_rect: Rect,
}

pub const LAYOUT_ZERO: Layout = Layout {
    content_rect: Rect::ZERO,
    border_rect: Rect::ZERO,
};

impl Default for Layout {
    fn default() -> Self {
        Self {
            content_rect: Rect::ZERO,
            border_rect: Rect::ZERO,
        }
    }
}

#[derive(Debug)]
pub(crate) struct IDManager {
    pub(crate) id_mappings: HashMap<ID, Layout>,
    next_id: ID,
}

impl IDManager {
    pub fn gen_id(&mut self) -> ID {
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn gen_insert_zero(&mut self) -> ID {
        let id = self.gen_id();
        self.id_mappings.insert(id, Default::default());
        id
    }

    pub fn set_layout_content(&mut self, id: ID, layout: Rect) -> Option<Layout> {
        if let Some(full) = self.id_mappings.get_mut(&id) {
            full.content_rect = layout;
            None
        } else {
            self.id_mappings.insert(
                id,
                Layout {
                    content_rect: layout,
                    border_rect: layout,
                },
            )
        }
    }

    pub fn set_layout_border(&mut self, id: ID, layout: Rect) -> Option<Layout> {
        if let Some(full) = self.id_mappings.get_mut(&id) {
            full.border_rect = layout;
            None
        } else {
            self.id_mappings.insert(
                id,
                Layout {
                    content_rect: layout,
                    border_rect: layout,
                },
            )
        }
    }

    pub fn get_layout(&mut self, id: ID) -> &Layout {
        self.id_mappings.get(&id).unwrap_or(&LAYOUT_ZERO)
    }
}

// pub fn fd() {
//     let options = IdGeneratorOptions::new().worker_id(1).worker_id_bit_len(6);
//     // Initialize the id generator instance with the option.
//     // Other options not set will be given the default value.
//     let _ = IdInstance::init(options).unwrap();
//     // Call `next_id` to generate a new unique id.
//     let id = IdInstance::next_id();
//     println!("id is {}", id);
// }
