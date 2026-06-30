use proc_macro::TokenStream;
use quote::ToTokens;

mod to_table;
use to_table::ToTable;
mod to_columns;
use to_columns::ToColumns;

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
///
/// #[derive(Debug, Clone, silo_derive::ToTable)]
/// struct EmptyTable {}
///
/// #[derive(Debug, Clone, silo_derive::ToColumns)]
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
/// #[derive(Debug, Clone, silo_derive::ToTable)]
/// struct Person {
///     #[silo(primary)]
///     id: usize,
///     #[silo(primary)]
///     last_name: String
/// }
/// ```
pub fn derive_to_table(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input)
        .expect("This is a derive macro and should be used with structs or enums.");

    let base = match input.data {
        syn::Data::Struct(data_struct) => {
            ToTable::from_struct(input.attrs, input.ident, input.vis, data_struct)
        }
        syn::Data::Enum(data_enum) => {
            ToTable::from_enum(input.attrs, input.ident, input.vis, data_enum)
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
            ToColumns::from_struct(input.attrs, input.ident, input.vis, data_struct)
        }
        syn::Data::Enum(data_enum) => {
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
