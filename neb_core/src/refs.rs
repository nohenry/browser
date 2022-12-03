use std::{
    cell::{Ref, RefCell},
    fmt::Display,
    ops::Deref,
    rc::Rc,
};

use crate::tree_display::TreeDisplay;

pub struct Rf<T: ?Sized>(pub RefCell<Rc<T>>);

impl<T: ?Sized> Clone for Rf<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

// impl<T: Display> Display for Rf<T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.0.borrow().as_ref().fmt(f)
//     }
// }

// impl <T: TreeDisplay> TreeDisplay for Rf<T> {
//     fn num_children(&self) -> usize {
//         self.0.borrow().as_ref().num_children()
//     }

//     fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
//         self.0.borrow().as_ref().child_at(index)
//     }

//     fn child_at_bx<'a>(&'a self, index: usize) -> Box<dyn TreeDisplay + 'a> {
//         self.0.borrow().as_ref().child_at_bx(index)
//     }
// }

impl<T: std::fmt::Debug> std::fmt::Debug for Rf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Rf")
            .field(&self.0.borrow().as_ref())
            .finish()
    }
}

impl<T> Deref for Rf<T> {
    type Target = RefCell<Rc<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: std::fmt::Debug> Rf<T> {
    pub fn new(t: T) -> Rf<T> {
        Rf(RefCell::new(Rc::new(t)))
    }
}

impl<T: std::fmt::Debug> From<T> for Rf<T> {
    fn from(t: T) -> Self {
        Rf::new(t)
    }
}

// impl<T> !From<Rf<T>> for Rf<T> {}
