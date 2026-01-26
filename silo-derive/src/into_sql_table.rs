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
            // const COLUMNS: &'static [silo::SqlColumn] = silo::concat_sql_columns!(&[#(#columns,)*]);

            fn fill_columns(columns: &mut Vec<silo::SqlColumn>) {
                #(columns.extend(#columns);)*
            }

            fn insert_foreign_references(self, connection: &silo::rusqlite::Connection) -> Result<(), silo::rusqlite::Error> {
                use silo::AsForeignReference;
                #(
                    self.#field_names.insert_as_foreign_reference(connection)?;
                )*
                Ok(())
            }
        }
    }
}
