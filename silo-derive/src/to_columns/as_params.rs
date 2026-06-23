use crate::to_table;

pub(crate) fn impl_as_params(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    to_table::as_params::create_as_params(base_struct, tokens, false);
}
