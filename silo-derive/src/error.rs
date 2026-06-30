use std::fmt::Display;

use quote::ToTokens;

pub struct Error {
    span: proc_macro2::Span,
    kind: ErrorKind,
}
impl Error {
    pub(crate) fn new(span: proc_macro2::Span, kind: ErrorKind) -> Self {
        Self { span, kind }
    }
}

pub enum ErrorKind {
    TooManyPrimaries,
    MultipleConflictAttributes,
    InvalidAttribute(String),
    NoColumns,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::TooManyPrimaries => write!(
                f,
                "Found multiple elements marked with #[silo(primary)], at most one is allowed!"
            ),
            ErrorKind::MultipleConflictAttributes => write!(
                f,
                "Found multiple on clonflict attributes. At most one is allowed."
            ),
            ErrorKind::InvalidAttribute(attribute) => {
                write!(f, "No attribute named {attribute} was expected here.")
            }
            ErrorKind::NoColumns => {
                write!(f, "No columns on this struct, nothing to put into a table.")
            }
        }
    }
}

impl ToTokens for Error {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(syn::Error::new(self.span, &self.kind).into_compile_error());
    }
}
