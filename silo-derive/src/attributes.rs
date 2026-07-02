use proc_macro2::Span;
use quote::quote;
use syn::{Attribute, spanned::Spanned};

use crate::error::{Error, ErrorKind};

pub enum StructuredAttributeArguments {
    Identifier(String),
    IdentifierExpression(String, syn::Expr),
}
impl StructuredAttributeArguments {
    fn new(argument: syn::Expr) -> Option<Self> {
        match argument {
            syn::Expr::Path(path) => Some(Self::Identifier(path.path.get_ident()?.to_string())),
            syn::Expr::Assign(assign) => {
                let syn::Expr::Path(name) = *assign.left else {
                    return None;
                };
                let name = name.path.get_ident()?.to_string();
                let expr = *assign.right;
                Some(Self::IdentifierExpression(name, expr))
            }
            _ => None,
        }
    }
}

pub struct StructuredAttribute {
    span: Span,
    path: String,
    arguments: StructuredAttributeArguments,
}
impl StructuredAttribute {
    fn new(attribute: &Attribute) -> Option<Self> {
        let path = attribute.path().get_ident()?.to_string();
        let arguments = StructuredAttributeArguments::new(attribute.parse_args().ok()?)?;
        let span = attribute.span();
        Some(Self {
            path,
            arguments,
            span,
        })
    }
}

#[derive(Debug, Default)]
pub struct ToTableAttributesStruct {
    pub on_conflict_rollback: bool,
    pub on_conflict_abort: bool,
    pub on_conflict_fail: bool,
    pub on_conflict_ignore: bool,
    pub on_conflict_replace: bool,
    pub has_custom_migration_handler: bool,
}

impl ToTableAttributesStruct {
    pub fn parse(attrs: &[Attribute]) -> Result<ToTableAttributesStruct, Error> {
        let mut this = Self::default();
        for attribute in attrs {
            let Some(attribute) = StructuredAttribute::new(attribute) else {
                panic!("Invalid attribute");
            };
            if attribute.path != "silo" {
                return Err(Error::new(
                    attribute.span,
                    ErrorKind::InvalidAttribute(attribute.path),
                ));
            }
            match attribute.arguments {
                StructuredAttributeArguments::Identifier(name) => match name.as_str() {
                    "rollback" => this.on_conflict_rollback = true,
                    "abort" => this.on_conflict_abort = true,
                    "fail" => this.on_conflict_fail = true,
                    "ignore" => this.on_conflict_ignore = true,
                    "replace" => this.on_conflict_replace = true,
                    "migrate" => this.has_custom_migration_handler = true,
                    _ => {
                        panic!("Invalid attribute");
                    }
                },
                StructuredAttributeArguments::IdentifierExpression(..) => {
                    panic!("No = in attributes allowed here!");
                }
            }
        }

        Ok(this)
    }

    pub fn on_conflict(&self) -> proc_macro2::TokenStream {
        match [
            self.on_conflict_abort,
            self.on_conflict_fail,
            self.on_conflict_ignore,
            self.on_conflict_replace,
            self.on_conflict_rollback,
        ] {
            [false, false, false, false, false] | [true, ..] => {
                quote! {silo::SqlFailureBehavior::Abort}
            }
            [_, true, ..] => quote! {silo::SqlFailureBehavior::Fail},
            [_, _, true, ..] => quote! {silo::SqlFailureBehavior::Ignore},
            [_, _, _, true, ..] => quote! {silo::SqlFailureBehavior::Replace},
            [.., true] => quote! {silo::SqlFailureBehavior::Rollback},
        }
    }
}

#[derive(Default)]
pub struct AttributeFieldData {
    pub is_primary: bool,
    pub is_unique: bool,
    pub is_skip: bool,
    pub default: Option<syn::Expr>,
}

impl std::fmt::Debug for AttributeFieldData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AttributeFieldData")
            .field("is_primary", &self.is_primary)
            .field("is_unique", &self.is_unique)
            .field("is_skip", &self.is_skip)
            .field("default", &self.default.is_some())
            .finish()
    }
}

impl AttributeFieldData {
    pub fn parse(attrs: &[Attribute]) -> AttributeFieldData {
        let mut this = Self::default();
        for attribute in attrs {
            let Some(attribute) = StructuredAttribute::new(attribute) else {
                panic!("Invalid attribute is not formatted right (sadly)");
            };
            if attribute.path != "silo" {
                panic!("Invalid attribute");
            }
            match attribute.arguments {
                StructuredAttributeArguments::Identifier(name) => match name.as_str() {
                    "primary" => this.is_primary = true,
                    "unique" => this.is_unique = true,
                    "skip" => this.is_skip = true,
                    name => {
                        panic!("Invalid attribute: {name}");
                    }
                },
                StructuredAttributeArguments::IdentifierExpression(name, expr) => {
                    match name.as_str() {
                        "default" => {
                            this.default = Some(expr);
                        }
                        name => {
                            panic!("Invalid assignment attribute: {name}");
                        }
                    }
                }
            }
        }
        this
    }
}
