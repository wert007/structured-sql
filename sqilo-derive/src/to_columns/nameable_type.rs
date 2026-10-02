use proc_macro2::TokenStream;
use quote::quote;
use syn::{LitStr, ext::IdentExt};

pub(crate) fn impl_nameable_type(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let name = &base_struct.name;
    let name_str_lit = LitStr::new(&name.unraw().to_string(), name.span());
    let a = &base_struct.generics;
    let b = &base_struct.generics_names_only(TokenStream::new());
    let c = &base_struct.where_clause;
    let mut t = a.type_params().peekable();
    let type_name_impl = if t.peek().is_none() {
        quote! { #name_str_lit.into() }
    } else {
        let idents = t.map(|t| &t.ident);
        quote! {
            [
                #name_str_lit,
                #(&<#idents as sqilo::NameableType>::type_name(),)*
            ].join("").into()
        }
    };

    tokens.extend(quote! {
        impl #a sqilo::NameableType for #name #b #c {
            fn type_name() -> std::borrow::Cow<'static, str> {
                #type_name_impl
            }
        }
    });
}
