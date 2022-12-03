use std::fmt::Display;

use colored::Colorize;

pub enum ErrorKind {
    Info,
    Warning,
    Error,
}

pub enum DocumentErrorType {
    ExpectedTag(String),
}

impl DocumentErrorType {
    pub fn get_message(&self) -> String {
        match self {
            DocumentErrorType::ExpectedTag(tag) => format!("Expected Tag `{}`", tag),
        }
    }
}

pub struct DocumentError {
    error_kind: ErrorKind,
    error_type: DocumentErrorType,
}

impl DocumentError {
    pub fn new(ty: DocumentErrorType, kind: ErrorKind) -> DocumentError {
        DocumentError {
            error_kind: kind,
            error_type: ty,
        }
    }

    pub fn get_message(&self) -> String {
        self.error_type.get_message()
    }
}

impl Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match &self.error_kind {
            ErrorKind::Info => format!("{}{}", "Info: ".blue().bold(), self.get_message().bold()),
            ErrorKind::Warning => format!(
                "{}{}",
                "Warning: ".yellow().bold(),
                self.get_message().bold()
            ),
            ErrorKind::Error => {
                format!("{}{}", "Info: ".red().bold(), self.get_message().bold())
            }
        };
        write!(f, "{}", msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
