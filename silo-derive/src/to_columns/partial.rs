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
    }
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
