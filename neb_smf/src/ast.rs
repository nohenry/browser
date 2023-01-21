use crate::{
    format::{NodeDisplay, TreeDisplay},
    token::{Range, SpannedToken, Token},
};

pub trait AstNode {
    fn get_range(&self) -> Range;
}

pub struct PunctuationList<T: AstNode> {
    tokens: Vec<(T, Option<SpannedToken>)>,
}

impl<T: AstNode> PunctuationList<T> {
    pub fn new() -> PunctuationList<T> {
        PunctuationList { tokens: Vec::new() }
    }

    pub fn push(&mut self, val: T, separator: Option<SpannedToken>) {
        self.tokens.push((val, separator))
    }

    pub fn push_sep(&mut self, val: T, separator: SpannedToken) {
        self.tokens.push((val, Some(separator)))
    }

    pub fn push_term(&mut self, val: T) {
        self.tokens.push((val, None))
    }

    pub fn iter_items(&self) -> impl Iterator<Item = &T> + '_ {
        self.tokens.iter().map(|(v, _)| v)
    }
}

impl<T> NodeDisplay for PunctuationList<T>
where
    T: NodeDisplay + AstNode,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Punctuation List")?;
        write!(f, " {}", self.tokens.len())
    }
}

impl<T> TreeDisplay for PunctuationList<T>
where
    T: TreeDisplay + AstNode,
{
    fn num_children(&self) -> usize {
        if let Some((_, Some(_))) = self.tokens.last() {
            self.tokens.len() * 2
        } else {
            self.tokens.len() * 2 - 1
        }
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        let p = &self.tokens[index / 2];
        if index % 2 == 0 {
            Some(&p.0)
        } else {
            Some(p.1.as_ref().unwrap())
        }
    }
}

pub struct ElementArgs {
    pub range: Range,
    pub items: PunctuationList<Arg>,
}

impl AstNode for ElementArgs {
    fn get_range(&self) -> Range {
        self.range 
    }
}

impl ElementArgs {
    pub fn iter_items(&self) -> impl Iterator<Item = &Arg> + '_ {
        self.items.iter_items()
    }
}

impl NodeDisplay for ElementArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Element Parameters")
    }
}

impl TreeDisplay for ElementArgs {
    fn num_children(&self) -> usize {
        2
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        match index {
            0 => Some(&self.range),
            1 => Some(&self.items),
            _ => panic!(),
        }
    }
}

pub struct Arg {
    pub name: SpannedToken,
    pub colon: SpannedToken,
    pub value: Expression,
}

impl AstNode for Arg {
    fn get_range(&self) -> Range {
        Range::from((&self.name, &self.value.get_range())) 
    }
}

impl Arg {
    pub fn name(&self) -> &String {
        match &self.name.1 {
            Token::Ident(s) => s,
            _ => panic!(),
        }
    }
}

impl NodeDisplay for Arg {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Parameter")
    }
}

impl TreeDisplay for Arg {
    fn num_children(&self) -> usize {
        3
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        match index {
            0 => Some(&self.name),
            1 => Some(&self.colon),
            2 => Some(&self.value),
            _ => panic!(),
        }
    }
}

pub enum Expression {
    Ident(SpannedToken),
}

impl AstNode for Expression {
    fn get_range(&self) -> Range {
        match self {
            Self::Ident(i) => i.0.into(),
        }
    }
}

impl NodeDisplay for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Ident(s) => write!(f, "{}", s.format()),
        }
    }
}

impl TreeDisplay for Expression {
    fn num_children(&self) -> usize {
        match self {
            Self::Ident(_) => 0,
        }
    }

    fn child_at(&self, _index: usize) -> Option<&dyn TreeDisplay> {
        match self {
            Self::Ident(_) => None,
        }
    }
}

pub enum Statement {
    Expression(Expression),
    Element {
        name: String,
        arguments: Option<ElementArgs>,
        body: Vec<Statement>,
        body_range: Range,
        token: SpannedToken,
    },
}

impl AstNode for Statement {
    fn get_range(&self) -> Range {
        match self {
            Self::Expression(e) => e.get_range(),
            Self::Element {
                body_range, token, ..
            } => Range::from((token, body_range)),
        }
    }
}

impl NodeDisplay for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("Statement")
        // match self {
        //     Self::Element { .. } => f.write_str("Element"),
        //     Self::Expression { .. } => f.write_str("Expression"),
        // }
    }
}

impl TreeDisplay for Statement {
    fn num_children(&self) -> usize {
        match self {
            Self::Element { arguments, .. } => 4 + if arguments.is_some() { 1 } else { 0 },
            Self::Expression(_) => 1,
        }
    }

    fn child_at(&self, index: usize) -> Option<&dyn TreeDisplay> {
        match self {
            Self::Element {
                body,
                body_range,
                token,
                arguments,
                ..
            } => match index {
                0 => None,
                1 => Some(body),
                2 => Some(body_range),
                3 => Some(token),
                4 => Some(arguments.as_ref().unwrap()),
                _ => panic!(),
            },
            Self::Expression(e) => Some(e),
        }
    }

    fn child_at_bx<'b>(&'b self, index: usize) -> Box<dyn TreeDisplay + 'b> {
        match self {
            Self::Element { name, .. } => match index {
                0 => Box::new(format!("Name: `{}`", name)),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }
}
