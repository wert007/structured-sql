use quote::quote;

pub(crate) fn create_into_sql_table(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let name = &base_struct.name;
    let table_name = base_struct.table_name();
    let columns = base_struct.columns();

    let field_names: Vec<_> = base_struct.fields().iter().map(|f| f.name).collect();

    quote! {
        impl<'a> silo::ToTable<'a> for #name {
            type Table = #table_name<'a>;
            const NAME: &'static str = stringify!(#name);
        }
    }
}
