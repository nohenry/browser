use std::{
    cell::{Ref, RefCell},
    fmt,
    sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard},
};

use crate::Rf;

pub struct Fmt<F>(pub F)
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result;

impl<F> fmt::Display for Fmt<F>
where
    F: Fn(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.0)(f)
    }
}

pub struct FmtMut<F>(pub Box<RwLock<F>>)
where
    F: FnMut(&mut fmt::Formatter) -> fmt::Result;

impl<F> FmtMut<F>
where
    F: FnMut(&mut fmt::Formatter) -> fmt::Result,
{
    pub fn new(f: F) -> Self {
        FmtMut(Box::new(RwLock::new(f)))
    }
}

impl<F> fmt::Display for FmtMut<F>
where
    F: FnMut(&mut fmt::Formatter) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut p = self.0.write().unwrap();
        p(f)
    }
}

pub trait NodeDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

pub trait AsTrait<U> {
    fn as_trait(&self) -> &dyn TreeDisplay<U>;
}

impl<T: TreeDisplay<U> + Sized, U> AsTrait<U> for T {
    fn as_trait(&self) -> &dyn TreeDisplay<U> {
        self
    }
}

pub trait TreeDisplay<U = ()>: NodeDisplay + AsTrait<U> {
    fn num_children(&self) -> usize;
    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay<U>>;
    fn child_at_bx<'a>(&'a self, _index: usize) -> Box<dyn TreeDisplay<U> + 'a> {
        panic!("This type doesn't used box values!")
    }

    fn get_user_data(&self) -> Option<U> {
        None
    }

    fn write(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        index: u32,
        indent: &String,
        last: bool,
    ) -> std::fmt::Result {
        write!(f, "{}", indent)?;
        if index != 0 {
            write!(f, "{}", if last { "└──" } else { "├──" })?;
        }
        let nindent = format!(
            "{}{}",
            indent,
            if index == 0 {
                ""
            } else if last {
                "    "
            } else {
                "│   "
            }
        );

        let st = self.fmt(f)?;
        write!(f, "\n")?;

        // write!(f, "{}\n", self)?;

        let n = self.num_children();
        for i in 0..n {
            let child = self.child_at(i);
            if let Some(child) = child {
                child.write(
                    f,
                    (i + 1).try_into().unwrap(),
                    &nindent,
                    if i == n - 1 { true } else { false },
                )?;
            } else {
                let child = self.child_at_bx(i);
                child.write(
                    f,
                    (i + 1).try_into().unwrap(),
                    &nindent,
                    if i == n - 1 { true } else { false },
                )?;
            }
        }

        write!(f, "")
    }

    fn write_unformatted(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        index: u32,
        indent: &String,
        last: bool,
        founc: &mut Box<dyn FnMut(&dyn TreeDisplay<U>, &str) -> Option<String>>,
    ) -> std::fmt::Result {
        write!(f, "{}", indent)?;
        if index != 0 {
            write!(f, "{}", if last { "└──" } else { "├──" })?;
        }
        let nindent = format!(
            "{}{}",
            indent,
            if index == 0 {
                ""
            } else if last {
                "    "
            } else {
                "│   "
            }
        );

        let val = format!("{}", Fmt(|f| self.fmt(f)));
        let valo = founc(self.as_trait(), &val);
        if let Some(val) = valo {
            write!(f, "{}\n", val)?;
        } else {
            write!(f, "{}\n", val)?;
        }
        // self.fmt(f)?;
        // write!(f, "\n")?;

        let n = self.num_children();
        for i in 0..n {
            let child = self.child_at(i);
            if let Some(child) = child {
                child.write_unformatted(
                    f,
                    (i + 1).try_into().unwrap(),
                    &nindent,
                    if i == n - 1 { true } else { false },
                    founc,
                )?;
            } else {
                let child = self.child_at_bx(i);
                child.write_unformatted(
                    f,
                    (i + 1).try_into().unwrap(),
                    &nindent,
                    if i == n - 1 { true } else { false },
                    founc,
                )?;
            }
        }

        write!(f, "")
    }

    fn format(&self) -> String {
        format!("{}", Fmt(|f| self.write(f, 0, &String::from(""), false)))
    }

    fn format_unformat(
        &self,
        mut founc: Box<dyn FnMut(&dyn TreeDisplay<U>, &str) -> Option<String>>,
    ) -> String {
        format!(
            "{}",
            FmtMut::new(|f| self.write_unformatted(f, 0, &String::from(""), false, &mut founc))
        )
    }
}

pub struct Grouper(pub String);

impl NodeDisplay for Grouper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TreeDisplay for Grouper {
    fn num_children(&self) -> usize {
        0
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        panic!()
    }
}

impl<'a, T: NodeDisplay + 'a> NodeDisplay for Vec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("")
    }
}

impl<'a, T: TreeDisplay + 'a> TreeDisplay for Vec<T> {
    fn num_children(&self) -> usize {
        self.len()
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        Some(&self[index])
    }
}

impl NodeDisplay for String {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self)
    }
}

impl TreeDisplay for String {
    fn num_children(&self) -> usize {
        0
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        None
    }
}

impl<T> NodeDisplay for Option<T>
where
    T: NodeDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Some(v) => v.fmt(f),
            _ => f.write_str(""),
        }
    }
}

impl<T> NodeDisplay for Rf<T>
where
    T: NodeDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.borrow().fmt(f)
    }
}

impl<T> TreeDisplay for Rf<T>
where
    T: NodeDisplay + TreeDisplay,
{
    fn num_children(&self) -> usize {
        1
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        None
    }

    fn child_at_bx<'a>(&'a self, _index: usize) -> Box<dyn TreeDisplay + 'a> {
        Box::new(self.borrow())
    }
}

impl<T> NodeDisplay for Ref<'_, T>
where
    T: NodeDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <T as NodeDisplay>::fmt(&self, f)
    }
}

impl<T> TreeDisplay for Ref<'_, T>
where
    T: NodeDisplay + TreeDisplay,
{
    fn num_children(&self) -> usize {
        <T as TreeDisplay>::num_children(&self)
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        <T as TreeDisplay>::child_at(&self, index)
    }
}

impl<T> NodeDisplay for MutexGuard<'_, T>
where
    T: NodeDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <T as NodeDisplay>::fmt(&self, f)
    }
}

impl<T> TreeDisplay for MutexGuard<'_, T>
where
    T: NodeDisplay + TreeDisplay,
{
    fn num_children(&self) -> usize {
        <T as TreeDisplay>::num_children(&self)
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        <T as TreeDisplay>::child_at(&self, index)
    }
}

impl<T> NodeDisplay for RwLockReadGuard<'_, T>
where
    T: NodeDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <T as NodeDisplay>::fmt(&self, f)
    }
}

impl<T, U> TreeDisplay<U> for RwLockReadGuard<'_, T>
where
    T: NodeDisplay + TreeDisplay<U>,
{
    fn num_children(&self) -> usize {
        <T as TreeDisplay<U>>::num_children(&self)
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay<U>> {
        <T as TreeDisplay<U>>::child_at(&self, index)
    }

    fn child_at_bx<'a>(&'a self, index: usize) -> Box<dyn TreeDisplay<U> + 'a> {
        <T as TreeDisplay<U>>::child_at_bx(&self, index)
    }

    fn get_user_data(&self) -> Option<U> {
        <T as TreeDisplay<U>>::get_user_data(&self)
    }
}

impl<T> NodeDisplay for RefCell<T>
where
    T: NodeDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <T as NodeDisplay>::fmt(&self.borrow(), f)
    }
}

impl<T> NodeDisplay for Mutex<T>
where
    T: NodeDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <T as NodeDisplay>::fmt(&self.lock().unwrap(), f)
    }
}
