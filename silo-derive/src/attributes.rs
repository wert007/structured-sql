use convert_case::Case;
use itertools::Itertools;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::{Attribute, spanned::Spanned};

use crate::error::Error;

pub enum StructuredAttributeArguments {
    Identifier(String),
    IdentifierExpression(String, syn::Expr),
}

impl StructuredAttributeArguments {
    fn new(meta: syn::Meta) -> Vec<Self> {
        match meta {
            syn::Meta::Path(path) => {
                let Some(ident) = path.get_ident() else {
                    return Vec::new();
                };
                vec![Self::Identifier(ident.to_string())]
            }
            syn::Meta::List(meta_list) => {
                let tokens = meta_list.tokens.into_iter().collect_vec();
                let arguments = tokens
                    .split(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ','))
                    .map(|t| {
                        let mut ts = TokenStream::new();
                        ts.extend(t.into_iter().cloned());
                        ts
                    })
                    .collect_vec();
                let mut result = Vec::new();
                for argument in arguments {
                    let Ok(expr) = syn::parse2::<syn::Meta>(argument) else {
                        continue;
                    };
                    result.append(&mut Self::new(expr));
                }
                result
            }
            syn::Meta::NameValue(meta_name_value) => {
                let Some(name) = meta_name_value.path.get_ident() else {
                    return Vec::new();
                };
                let name = name.to_string();
                let expr = meta_name_value.value;
                vec![Self::IdentifierExpression(name, expr)]
            }
        }
    }
}

pub struct StructuredAttribute {
    span: Span,
    path: String,
    arguments: Vec<StructuredAttributeArguments>,
}
impl StructuredAttribute {
    fn new(attribute: &Attribute) -> Option<Self> {
        let path = attribute.path().get_ident()?.to_string();
        let arguments = StructuredAttributeArguments::new(attribute.meta.clone());
        let span = attribute.span();
        Some(Self {
            path,
            arguments,
            span,
        })
    }
}

#[derive(Debug, Default)]
pub struct ToColumnsAttributesEnum {
    pub rename: Option<Case<'static>>,
}

impl ToColumnsAttributesEnum {
    pub fn parse(attrs: &[Attribute]) -> Result<ToColumnsAttributesEnum, Error> {
        let mut this: ToColumnsAttributesEnum = Self::default();
        for attribute in attrs {
            let span = attribute.span();
            let Some(attribute) = StructuredAttribute::new(attribute) else {
                // This is not intended for us.
                continue;
            };
            if attribute.path != "silo" {
                // This is not intended for us.
                continue;
            }
            for attribute in attribute.arguments {
                match attribute {
                    StructuredAttributeArguments::Identifier(ident) => {
                        return Err(Error::new(
                            span,
                            crate::error::ErrorKind::InvalidAttribute(ident),
                        ));
                    }
                    StructuredAttributeArguments::IdentifierExpression(name, expr) => {
                        match name.as_str() {
                            "rename" => {
                                this.rename = Some(convert_expr_to_case_naming(expr)?);
                            }
                            _ => {
                                return Err(Error::new(
                                    span,
                                    crate::error::ErrorKind::InvalidAttribute(name),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(this)
    }
}

fn convert_expr_to_case_naming(expr: syn::Expr) -> Result<Case<'static>, Error> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => match lit.value().as_str() {
            "kebab-case" => Ok(Case::Kebab),
            "PascalCase" => Ok(Case::Pascal),
            "camelCase" => Ok(Case::Camel),
            "snake_case" => Ok(Case::Snake),
            "UPPER-KEBAB-CASE" => Ok(Case::UpperKebab),
            "UPPER_SNAKE_CASE" => Ok(Case::UpperSnake),
            _ => Err(Error::new(
                lit.span(),
                crate::error::ErrorKind::InvalidArgumentToAttributeForRename,
            )),
        },
        _ => Err(Error::new(
            expr.span(),
            crate::error::ErrorKind::ArgumentToAttributeMustBeStringLiteral,
        )),
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
                // This is not intended for us.
                continue;
            };
            if attribute.path != "silo" {
                // This is not intended for us.
                continue;
            }
            for attribute in attribute.arguments {
                match attribute {
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
                // This is not intended for us.
                continue;
            };
            if attribute.path != "silo" {
                // This is not intended for us.
                continue;
            }
            for attribute in attribute.arguments {
                match attribute {
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
        }
        this
    }
}
