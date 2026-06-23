use crate::{to_columns::extract_from_row, to_table};

pub(crate) fn impl_to_partial(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    // Seems to be the same for now.
    to_table::partial::create_partial_for(base_struct, tokens);
    extract_from_row::impl_extract_from_row(tokens, &base_struct.to_partial());
}
