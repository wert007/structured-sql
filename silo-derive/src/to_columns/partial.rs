use proc_macro2::TokenStream;
use quote::quote;

use crate::{to_columns::extract_from_row, to_table};

pub(crate) fn impl_to_partial(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    if base_struct.is_simple_enum() {
        impl_to_partial_for_simple_enum(tokens, base_struct);
    } else {
        // Seems to be the same for now.
        to_table::partial::create_partial_for(base_struct, tokens);
        extract_from_row::impl_extract_from_row(tokens, &base_struct.to_partial());
        impl_marker_trait(tokens, base_struct);
    }
}

fn impl_marker_trait(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let a = &base_struct.generics;
    let b = base_struct.generics_names_only(TokenStream::new());
    let c = &base_struct.where_clause;
    let name = &base_struct.name;
    let partial_name = base_struct.partial_name();
    tokens.extend(quote! {
        impl #a silo::SiloPartialMarkerTrait<#name #b> for #partial_name #b #c {}

    });
}

fn impl_to_partial_for_simple_enum(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let name = &base_struct.name;
    tokens.extend(quote! {
        impl silo::partial::HasPartial for #name {
            type Partial = Option<#name>;
        }
    });
}
