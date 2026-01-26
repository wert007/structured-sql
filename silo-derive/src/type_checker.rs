use quote::format_ident;
use syn::{Ident, Type};

pub trait TypeIsLike {
    fn is_string_like(&self) -> bool;
    fn is_number_like(&self) -> bool;
    fn is_bool_like(&self) -> bool;
}

impl TypeIsLike for Type {
    fn is_string_like(&self) -> bool {
        match self {
            Type::Group(type_group) => type_group.elem.is_string_like(),
            Type::Paren(type_paren) => type_paren.elem.is_string_like(),
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.last() else {
                    return false;
                };
                match segment.ident.to_string().as_str() {
                    "String" | "str" | "CStr" | "CString" | "OsString" | "OsStr" | "Path"
                    | "PathBuf" => true,
                    "Box" | "Arc" | "Rc" | "Option" => match &segment.arguments {
                        syn::PathArguments::None => false,
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let Some(arg) = angle_bracketed_generic_arguments.args.first() else {
                                return false;
                            };
                            match arg {
                                syn::GenericArgument::Type(t) => t.is_string_like(),
                                _ => false,
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => false,
                    },
                    "Cow" => match &segment.arguments {
                        syn::PathArguments::None => false,
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let Some(arg) = angle_bracketed_generic_arguments.args.get(1) else {
                                return false;
                            };
                            match arg {
                                syn::GenericArgument::Type(t) => t.is_string_like(),
                                _ => false,
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => false,
                    },
                    _ => false,
                }
            }
            Type::Reference(type_reference) => type_reference.elem.is_string_like(),
            _ => false,
        }
    }

    fn is_number_like(&self) -> bool {
        match self {
            Type::Group(type_group) => type_group.elem.is_number_like(),
            Type::Paren(type_paren) => type_paren.elem.is_number_like(),
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.last() else {
                    return false;
                };
                match segment.ident.to_string().trim_start_matches("r#") {
                    "u8" | "u16" | "u32" | "u64" | "usize" | "i8" | "i16" | "i32" | "i64"
                    | "isize" | "f32" | "f64" => true,
                    "Box" | "Arc" | "Rc" | "Option" => match &segment.arguments {
                        syn::PathArguments::None => false,
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let Some(arg) = angle_bracketed_generic_arguments.args.first() else {
                                return false;
                            };
                            match arg {
                                syn::GenericArgument::Type(t) => t.is_string_like(),
                                _ => false,
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => false,
                    },
                    "Cow" => match &segment.arguments {
                        syn::PathArguments::None => false,
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let Some(arg) = angle_bracketed_generic_arguments.args.get(1) else {
                                return false;
                            };
                            match arg {
                                syn::GenericArgument::Type(t) => t.is_string_like(),
                                _ => false,
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => false,
                    },
                    _ => false,
                }
            }
            Type::Reference(type_reference) => type_reference.elem.is_number_like(),
            _ => false,
        }
    }

    fn is_bool_like(&self) -> bool {
        match self {
            Type::Group(type_group) => type_group.elem.is_bool_like(),
            Type::Paren(type_paren) => type_paren.elem.is_bool_like(),
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.last() else {
                    return false;
                };
                match segment.ident.to_string().as_str() {
                    "bool" => true,
                    "Box" | "Arc" | "Rc" | "Option" => match &segment.arguments {
                        syn::PathArguments::None => false,
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let Some(arg) = angle_bracketed_generic_arguments.args.first() else {
                                return false;
                            };
                            match arg {
                                syn::GenericArgument::Type(t) => t.is_string_like(),
                                _ => false,
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => false,
                    },
                    "Cow" => match &segment.arguments {
                        syn::PathArguments::None => false,
                        syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                            let Some(arg) = angle_bracketed_generic_arguments.args.get(1) else {
                                return false;
                            };
                            match arg {
                                syn::GenericArgument::Type(t) => t.is_string_like(),
                                _ => false,
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => false,
                    },
                    _ => false,
                }
            }
            Type::Reference(type_reference) => type_reference.elem.is_bool_like(),
            _ => false,
        }
    }
}

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
