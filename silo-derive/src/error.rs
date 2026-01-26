use std::fmt::Display;

use quote::ToTokens;

pub struct Error {
    span: proc_macro2::Span,
    kind: ErrorKind,
}

pub enum ErrorKind {
    TooManyPrimaries,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::TooManyPrimaries => write!(
                f,
                "Found multiple elements marked with #[silo(primary)], at most one is allowed!"
            ),
        }
    }
}

impl ToTokens for Error {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(syn::Error::new(self.span, &self.kind).into_compile_error());
    }
}
