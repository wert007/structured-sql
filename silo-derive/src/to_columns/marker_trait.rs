use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn impl_marker_trait(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let a = &base_struct.generics;
    let b = base_struct.generics_names_only(TokenStream::new());
    let c = &base_struct.where_clause;
    let name = &base_struct.name;
    tokens.extend(quote! {
        impl #a silo::SiloMarkerTrait for #name #b #c {}
    });
}
