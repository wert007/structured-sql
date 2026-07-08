use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Lifetime, LifetimeParam, LitStr, ext::IdentExt};

pub(crate) fn create_into_sql_table(
    base_struct: &super::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let name = &base_struct.name;
    let where_clause = &base_struct.where_clause;
    let mut generics = base_struct.generics.clone();
    generics.params.insert(
        0,
        syn::GenericParam::Lifetime(LifetimeParam::new(Lifetime::new(
            "'__silo__a",
            Span::call_site(),
        ))),
    );
    let generics_names_only = base_struct.generics_names_only(TokenStream::new());
    let a = base_struct.generics_names_only(quote! {'__silo__a, });
    let table_name = base_struct.table_name();
    let name_str_lit = LitStr::new(&name.unraw().to_string(), name.span());

    quote! {
        impl #generics silo::ToTable<'__silo__a> for #name #generics_names_only #where_clause {
            type Table = #table_name #a;
            const NAME: &'static str = #name_str_lit;
        }
    }
}
