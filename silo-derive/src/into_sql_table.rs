use ident_case_conversions::CaseConversions;
use quote::{format_ident, quote};

use crate::type_checker::{StripOption, ToName};

pub(crate) fn create_into_sql_table(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let name = &base_struct.name;
    let table_name = base_struct.table_name();
    let columns = base_struct.columns();

    quote! {
        impl<'a> silo::IntoSqlTable<'a> for #name {
            type Table = #table_name<'a>;
            const NAME: &'static str = stringify!(#name);
            // const COLUMNS: &'static [silo::SqlColumn] = silo::concat_sql_columns!(&[#(#columns,)*]);

            fn fill_columns(columns: &mut Vec<silo::SqlColumn>) {
                #(columns.extend(#columns);)*
            }
        }
    }
}
