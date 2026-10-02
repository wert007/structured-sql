use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Lifetime, LifetimeParam, LitStr, ext::IdentExt};

use crate::attributes::ToTableAttributesStruct;

pub(crate) fn create_into_sql_table(
    base_struct: &super::base_struct::StructData,
    attr: &ToTableAttributesStruct,
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
    let table_name_impl = create_table_name_impl(base_struct, attr);

    quote! {
        impl #generics sqilo::ToTable<'__silo__a> for #name #generics_names_only #where_clause {
            type Table = #table_name #a;
            fn table_name() -> std::borrow::Cow<'static, str> {
                #table_name_impl
            }
        }
    }
}

fn create_table_name_impl(
    base_struct: &crate::base_struct::StructData,
    attr: &ToTableAttributesStruct,
) -> TokenStream {
    if let Some(name) = &attr.custom_table_name {
        let mut t = base_struct.generics.type_params().peekable();
        if t.peek().is_none() {
            quote! { #name.into() }
        } else {
            let idents = t.map(|t| &t.ident);
            quote! {format!(#name, #(#idents = <#idents as sqilo::NameableType>::type_name(),)*).into()}
        }
    } else {
        let name = &base_struct.name;
        let name_str_lit = LitStr::new(&name.unraw().to_string(), name.span());
        let mut t = base_struct.generics.type_params().peekable();
        if t.peek().is_none() {
            quote! { #name_str_lit.into() }
        } else {
            let idents = t.map(|t| &t.ident);
            quote! {
                [
                    #name_str_lit,
                    #(&<#idents as sqilo::NameableType>::type_name(),)*
                ].join("").into()
            }
        }
    }
}
