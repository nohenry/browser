use std::{cell::{Ref, RefCell}, fmt, sync::{Mutex, MutexGuard, RwLockReadGuard}};

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

pub trait NodeDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result;
}

pub trait TreeDisplay: NodeDisplay {
    fn num_children(&self) -> usize;
    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay>;
    fn child_at_bx<'a>(&'a self, _index: usize) -> Box<dyn TreeDisplay + 'a> {
        panic!("This type doesn't used box values!")
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

        self.fmt(f)?;
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

    fn format(&self) -> String {
        format!("{}", Fmt(|f| self.write(f, 0, &String::from(""), false)))
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

impl<T> TreeDisplay for RwLockReadGuard<'_, T>
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


