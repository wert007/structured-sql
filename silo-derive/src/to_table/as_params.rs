use itertools::Itertools;
use quote::{format_ident, quote};
use syn::{LitStr, ext::IdentExt};

pub(crate) fn create_as_params(
    base_struct: &super::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
    _for_table: bool,
) {
    let name = &base_struct.name;
    let columns = base_struct.columns();
    let is_primary = columns
        .iter()
        .map(|c| syn::LitBool::new(c.is_primary, c.span));
    let is_unique = columns
        .iter()
        .map(|c| syn::LitBool::new(c.is_unique, c.span));
    let column_types = columns.iter().map(|c| &c.type_).collect_vec();
    let names = columns
        .iter()
        .map(|c| format_ident!("{}", &c.name, span = c.span))
        .collect_vec();
    let names_str_lit = names.iter().map(|i| {
        let n = i.unraw();
        LitStr::new(&n.to_string(), n.span())
    });
    let as_params = quote! {
            impl silo::AsColumns for #name {
                const COLUMN_COUNT: usize = 0 #(+ <#column_types as silo::AsColumns>::COLUMN_COUNT)*;
            }

            impl silo::AsColumnsDynamicallySized for #name {
                fn columns(parent: Option<&str>, is_unique: bool, is_primary: bool) -> Vec<silo::SqlColumn> {
                    assert!(!is_unique);
                    assert!(!is_primary);
                    let parent = parent.map(|p| format!("{p}_")).unwrap_or_default();
                    let mut result = Vec::with_capacity(<Self as silo::AsColumns>::COLUMN_COUNT);
                    #(
                        result.append(&mut <#column_types as silo::AsColumnsDynamicallySized>::columns(Some(&format!("{parent}{}", #names_str_lit)), #is_unique, #is_primary));
                    )*
                    result
                }
            }

            impl silo::AsParams for #name {
                fn as_params<'a>(&'a self) -> Vec<silo::ToSqlDyn<'a>> {
                    use silo::{AsParams};
                    let mut result = Vec::with_capacity(<Self as silo::AsColumns>::COLUMN_COUNT);
                    #(
                        result.extend(AsParams::as_params(&self.#names));
                    )*
                    result
                }
            }
    };
    tokens.extend(as_params);
}
