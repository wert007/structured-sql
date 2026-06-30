use quote::quote;
use syn::{LitStr, ext::IdentExt};

pub(crate) fn create_into_sql_table(
    base_struct: &super::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let name = &base_struct.name;
    let table_name = base_struct.table_name();
    let name_str_lit = LitStr::new(&name.unraw().to_string(), name.span());

    quote! {
        impl<'a> silo::ToTable<'a> for #name {
            type Table = #table_name<'a>;
            const NAME: &'static str = #name_str_lit;
        }
    }
}
