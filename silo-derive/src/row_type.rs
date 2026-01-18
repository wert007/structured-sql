use quote::quote;

pub(crate) fn create_row_type(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let name = &base_struct.name;
    quote! {
        impl silo::RowType for #name {}
    }
}
