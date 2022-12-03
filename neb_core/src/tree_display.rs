use std::{fmt::{self, Display}, rc::Rc};

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

pub trait TreeDisplay: Display {
    fn num_children(&self) -> usize;
    fn child_at(&self, index: usize) -> Option<Rf<dyn TreeDisplay>>;
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
        write!(f, "{}\n", self)?;

        let n = self.num_children();
        for i in 0..n {
            let child = self.child_at(i);
            if let Some(child) = child {
                child.0.borrow().as_ref().write(
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

impl Display for Grouper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TreeDisplay for Grouper {
    fn num_children(&self) -> usize {
        0
    }

    fn child_at(&self, _index: usize) -> Option<Rf<dyn TreeDisplay>> {
        panic!()
    }
}

pub struct CreateParent(pub String, pub Vec<Rf<dyn TreeDisplay>>);

impl Display for CreateParent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TreeDisplay for CreateParent {
    fn num_children(&self) -> usize {
        self.1.len()
    }

    fn child_at(&self, index: usize) -> Option<Rf<dyn TreeDisplay>> {
        Some(Rf(self.1[index].0.clone()))
    }
}
