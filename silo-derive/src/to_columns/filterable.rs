use crate::to_table;

pub(crate) fn impl_filterable(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    tokens.extend(to_table::filter::create_filter_for(base_struct));
}
