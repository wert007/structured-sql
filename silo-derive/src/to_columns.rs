use quote::quote;

pub(crate) fn create_to_columns_for_pk(
    base_struct: &crate::base_struct::StructData,
    pk: crate::base_struct::Field<'_>,
    tokens: &mut proc_macro2::TokenStream,
) {
    let name = &base_struct.name;
    let pk = pk.name;
    tokens.extend(quote! {
        impl silo::ToColumns for #name {
            fn fill_columns(columns: &mut Vec<silo::SqlColumn>) {
            columns.push(silo::SqlColumn {
                name: stringify!(#pk).into(),
                // TODO: Support all possible pk types.
                r#type: silo::SqlColumnType::Integer,
                is_primary: false,
                // Depends on how many to how many relation ship!
                is_unique: false,
            });
        }
        }
    });
}
