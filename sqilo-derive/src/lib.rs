use proc_macro::TokenStream;
use quote::ToTokens;

mod to_table;
use syn::ext::IdentExt;
use to_table::ToTableStruct;
mod to_columns;
use to_columns::ToColumnsStruct;

mod attributes;
mod base_struct;
mod error;

#[proc_macro_derive(ToTable, attributes(sqilo))]
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
///     #[sqilo(skip)]
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
/// **#[[sqilo(primary)]]**
///
/// You can designate one field as primary field. If you have multiple fields
/// marked as primary, compilation will fail.
///
/// ```compile_fail
/// # use silo_derive::ToTable;
/// #[derive(Debug, Clone, ToTable)]
/// struct Person {
///     #[sqilo(primary)]
///     id: usize,
///     #[sqilo(primary)]
///     last_name: String
/// }
/// ```
///
/// **#[[sqilo(skip)]]**
///
/// Any field, which can not be represented in a database, or which you do not
/// want to put into the database you can mark with skip.
///
/// ```ignore
/// #[derive(ToTable)]
/// struct Person<T> {
///     age: usize,
///     name: String,
///     #[sqilo(skip)]
///     is_senior: bool,
///     #[sqilo(skip)]
///     employment_history: JsonValue,
///     #[sqilo(skip)]
///     marker: PhantomMarker<T>,
/// }
/// ```

pub fn derive_to_table(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input)
        .expect("This is a derive macro and should be used with structs or enums.");

    let base = match input.data {
        syn::Data::Struct(data_struct) => ToTableStruct::from_struct(
            input.attrs,
            input.ident,
            input.vis,
            input.generics,
            data_struct,
        ),
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

#[proc_macro_derive(ToColumns, attributes(sqilo))]
pub fn derive_to_columns(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input)
        .expect("This is a derive macro and should be used with structs or enums.");

    let base = match input.data {
        syn::Data::Struct(data_struct) => ToColumnsStruct::from_struct(
            input.attrs,
            input.ident,
            input.vis,
            input.generics,
            data_struct,
        ),
        syn::Data::Enum(data_enum) => ToColumnsStruct::from_enum(
            input.attrs,
            input.ident,
            input.vis,
            input.generics,
            data_enum,
        ),
        syn::Data::Union(_) => {
            panic!("Unions need a clear representation, either use a struct or an enum.")
        }
    };
    match base {
        Ok(it) => it.into_token_stream().into(),
        Err(it) => it.into_token_stream().into(),
    }
}

trait ToLitStr {
    fn to_lit_str(&self) -> syn::LitStr;
}

impl ToLitStr for syn::Ident {
    fn to_lit_str(&self) -> syn::LitStr {
        let span = self.span();
        let value = self.unraw().to_string();
        syn::LitStr::new(&value, span)
    }
}
