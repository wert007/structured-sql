use proc_macro::TokenStream;
use quote::ToTokens;

mod to_table;
use to_table::ToTableStruct;
mod to_columns;
use to_columns::ToColumnsStruct;

mod attributes;
mod base_struct;
mod error;

#[proc_macro_derive(ToTable, attributes(silo))]
/// This allows you to use your struct as a table definition.
///
/// If a struct does not have any fields, or they are skipped, then there is
/// nothing to put into a table.
///
/// ```compile_fail
///# use silo_derive::ToTable;
/// #[derive(Debug, Clone, ToTable)]
/// struct EmptyTable {}
///```
/// ```compile_fail
/// # use silo_derive::ToTable;
/// #[derive(Debug, Clone, ToTable)]
/// struct AllFieldsSkippedInEmptyTable {
///     #[silo(skip)]
///     field: usize,
/// }
///```
/// ```compile_fail
/// # use silo_derive::ToTable;
/// #[derive(Debug, Clone, ToColumns)]
/// struct EmptyColumns {}
/// ```
///
/// # Attributes
///
/// ## Struct Attributes
///
/// ## Field Attributes
///
/// **#[[silo(primary)]]**
///
/// You can designate one field as primary field. If you have multiple fields
/// marked as primary, compilation will fail.
///
/// ```compile_fail
/// # use silo_derive::ToTable;
/// #[derive(Debug, Clone, ToTable)]
/// struct Person {
///     #[silo(primary)]
///     id: usize,
///     #[silo(primary)]
///     last_name: String
/// }
/// ```
///
/// **#[[silo(skip)]]**
///
/// Any field, which can not be represented in a database, or which you do not want to put into the database you can mark with skip.
///
/// ```ignore
/// #[derive(ToTable)]
/// struct Person {
///     age: usize,
///     name: String,
///     #[silo(skip)]
///     is_senior: bool,
///     #[silo(skip)]
///     employment_history: JsonValue,
/// }
/// ```

pub fn derive_to_table(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input)
        .expect("This is a derive macro and should be used with structs or enums.");

    let base = match input.data {
        syn::Data::Struct(data_struct) => {
            ToTableStruct::from_struct(input.attrs, input.ident, input.vis, data_struct)
        }
        syn::Data::Enum(data_enum) => {
            ToTableStruct::from_enum(input.attrs, input.ident, input.vis, data_enum)
        }
        syn::Data::Union(_) => {
            panic!("Unions need a clear representation, either use a struct or an enum.")
        }
    };
    match base {
        Ok(it) => it.into_token_stream().into(),
        Err(it) => it.into_token_stream().into(),
    }
}

#[proc_macro_derive(ToColumns, attributes(silo))]
pub fn derive_to_columns(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input)
        .expect("This is a derive macro and should be used with structs or enums.");

    let base = match input.data {
        syn::Data::Struct(data_struct) => {
            ToColumnsStruct::from_struct(input.attrs, input.ident, input.vis, data_struct)
        }
        syn::Data::Enum(_data_enum) => {
            panic!("Enums are currently not supported.")
        }
        syn::Data::Union(_) => {
            panic!("Unions need a clear representation, either use a struct or an enum.")
        }
    };
    match base {
        Ok(it) => it.into_token_stream().into(),
        Err(it) => it.into_token_stream().into(),
    }
}

// #[macro_export]
// #[proc_macro_derive(ToRows, attributes(silo))]
// pub fn derive_to_rows(input: TokenStream) -> TokenStream {
//     // syn::Data
//     let input: syn::DeriveInput = syn::parse(input)
//         .expect("This is a derive macro and should be used with structs or enums.");

//     let base = match input.data {
//         syn::Data::Struct(data_struct) => {
//             ToRows::from_struct(input.attrs, input.ident, input.vis, data_struct)
//         }
//         syn::Data::Enum(data_enum) => {
//             ToRows::from_enum(input.attrs, input.ident, input.vis, data_enum)
//         }
//         syn::Data::Union(_) => {
//             panic!("Unions need a clear representation, either use a struct or an enum.")
//         }
//     };
//     match base {
//         Ok(it) => it.into_token_stream().into(),
//         Err(it) => it.into_token_stream().into(),
//     }
// }
