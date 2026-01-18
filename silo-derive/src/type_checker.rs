use quote::format_ident;
use syn::{Ident, Type};

pub trait StripOption {
    fn strip_option(&self) -> &Self;
}

pub trait ToName {
    fn to_name(&self) -> Option<String>;
}

impl StripOption for syn::Type {
    fn strip_option(&self) -> &Self {
        match self {
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.iter().last() else {
                    return self;
                };
                if segment.ident.to_string() != "Option" {
                    return self;
                }
                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        let Some(syn::GenericArgument::Type(arg)) =
                            angle_bracketed_generic_arguments.args.iter().next()
                        else {
                            return self;
                        };
                        arg
                    }
                    _ => self,
                }
            }
            _ => self,
        }
    }
}

impl ToName for Type {
    fn to_name(&self) -> Option<String> {
        match self {
            Type::Group(type_group) => type_group.elem.to_name(),
            Type::Paren(type_paren) => type_paren.elem.to_name(),
            Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .into_iter()
                .filter(|s| s.arguments.is_empty())
                .map(|s| s.ident.to_string())
                .next(),
            // Should be unreachable!
            // Type::Ptr(type_ptr) => todo!(),
            // Type::Reference(type_reference) => todo!(),
            _ => None,
        }
    }
}
