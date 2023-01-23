use std::{
    cell::{RefCell},
    ops::Deref,
    rc::Rc,
};

pub struct Rf<T: ?Sized>(pub Rc<RefCell<T>>);

impl<T: ?Sized> Clone for Rf<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Rf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Rf")
            .field(&self.0.borrow())
            .finish()
    }
}

impl<T> Deref for Rf<T> {
    type Target = RefCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Rf<T> {
    pub fn new(t: T) -> Rf<T> {
        Rf(Rc::new(RefCell::new(t)))
    }
}

impl<T> From<T> for Rf<T> {
    fn from(t: T) -> Self {
        Rf::new(t)
    }
}