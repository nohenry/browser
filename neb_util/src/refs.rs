use std::{
    borrow::Borrow,
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex, MutexGuard},
};

pub struct Rf<T: ?Sized>(pub Arc<Mutex<T>>);

impl<T: ?Sized> Clone for Rf<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Rf<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Rf").field(&self.0.lock().unwrap()).finish()
    }
}

impl<T> Deref for Rf<T> {
    type Target = Mutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Rf<T> {
    pub fn new(t: T) -> Rf<T> {
        Rf(Arc::new(Mutex::new(t)))
    }

    pub fn borrow(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap()
    }

}

impl<T> From<T> for Rf<T> {
    fn from(t: T) -> Self {
        Rf::new(t)
    }
}
