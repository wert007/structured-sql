use quote::quote;
use syn::Attribute;

pub enum StructuredAttributeArguments {
    Identifier(String),
}
impl StructuredAttributeArguments {
    fn new(argument: syn::Expr) -> Option<Self> {
        match argument {
            syn::Expr::Path(path) => Some(Self::Identifier(path.path.get_ident()?.to_string())),
            _ => None,
        }
    }
}

pub struct StructuredAttribute {
    path: String,
    arguments: StructuredAttributeArguments,
}
impl StructuredAttribute {
    fn new(attribute: &Attribute) -> Option<Self> {
        let path = attribute.path().get_ident()?.to_string();
        let arguments = StructuredAttributeArguments::new(attribute.parse_args().ok()?)?;
        Some(Self { path, arguments })
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
    pub fn parse(attrs: &[Attribute]) -> ToTableAttributesStruct {
        let mut this = Self::default();
        for attribute in attrs {
            let Some(attribute) = StructuredAttribute::new(attribute) else {
                panic!("Invalid attribute");
            };
            if attribute.path != "silo" {
                panic!("Invalid attribute");
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
            }
        }

        this.validate();
        this
    }

    fn validate(&self) {
        let on_conflict = [
            self.on_conflict_abort,
            self.on_conflict_fail,
            self.on_conflict_ignore,
            self.on_conflict_replace,
            self.on_conflict_rollback,
        ];
        if on_conflict.iter().fold(0, |acc, cur| acc + *cur as usize) > 1 {
            panic!("Only one on conflict attribute can be active at once.");
        }
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

#[derive(Debug, Default)]
pub struct AttributeFieldData {
    pub is_primary: bool,
    pub is_unique: bool,
    pub is_skip: bool,
}

impl AttributeFieldData {
    pub fn parse(attrs: &[Attribute]) -> AttributeFieldData {
        let mut this = Self::default();
        for attribute in attrs {
            let Some(attribute) = StructuredAttribute::new(attribute) else {
                panic!("Invalid attribute");
            };
            if attribute.path != "silo" {
                panic!("Invalid attribute");
            }
            match attribute.arguments {
                StructuredAttributeArguments::Identifier(name) => match name.as_str() {
                    "primary" => this.is_primary = true,
                    "unique" => this.is_unique = true,
                    "skip" => this.is_skip = true,
                    _ => {
                        panic!("Invalid attribute");
                    }
                },
            }
        }
        this
    }
}
