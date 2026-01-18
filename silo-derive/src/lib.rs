use heck::{ToSnakeCase, ToSnekCase};
use ident_case_conversions::CaseConversions;
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, Error, Ident, LitInt, Type, TypePath, Visibility};

use crate::base_struct::Member;

mod as_params;
mod attributes;
mod base_struct;
mod enum_helper;
mod error;
mod filter;
mod from_row;
mod from_row_type;
mod into_sql_table;
mod partial;
mod row_type;
mod type_checker;

struct Base {
    visibility: Visibility,
    variants: Option<Vec<Ident>>,
    base_struct: base_struct::StructData,
    on_conflict: proc_macro2::TokenStream,
    migration_handler: proc_macro2::TokenStream,
}

impl std::fmt::Debug for Base {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Base")
            .field("variants", &self.variants)
            // .field("members", &self.members)
            .finish()
    }
}
impl Base {
    fn from_struct(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_struct: syn::DataStruct,
    ) -> Result<Self, crate::error::Error> {
        let attribute_struct_data = attributes::AttributeStructData::parse(&attrs);
        let on_conflict = attribute_struct_data.on_conflict();

        let base_struct: base_struct::StructData = base_struct::StructData::from_struct_data(
            visibility.clone(),
            name.clone(),
            data_struct.fields,
        )?;
        let migration_handler = if attribute_struct_data.has_custom_migration_handler {
            proc_macro2::TokenStream::new()
        } else {
            let row_type_name = &base_struct.name;
            quote! { impl silo::MigrationHandler for #row_type_name {}
            }
        };
        Ok(Self {
            visibility,
            variants: None,
            base_struct,
            on_conflict,
            migration_handler,
        })
    }

    fn from_enum(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_enum: syn::DataEnum,
    ) -> Result<Base, error::Error> {
        let attribute_struct_data = attributes::AttributeStructData::parse(&attrs);
        let on_conflict = attribute_struct_data.on_conflict();
        let variants = data_enum.variants.iter().map(|v| v.ident.clone()).collect();
        let base_struct: base_struct::StructData = base_struct::StructData::from_enum_data(
            visibility.clone(),
            name.clone(),
            data_enum.variants,
        )?;

        // Add Partial types for Migration here!
        let migration_handler = if attribute_struct_data.has_custom_migration_handler {
            proc_macro2::TokenStream::new()
        } else {
            let name = &base_struct.name;
            quote! { impl silo::MigrationHandler for #name {}
            }
        };
        Ok(Self {
            visibility,
            variants: Some(variants),
            on_conflict,
            migration_handler,
            base_struct,
        })
    }

    fn create_table(&self) -> proc_macro2::TokenStream {
        let Base {
            visibility,
            on_conflict,
            base_struct,
            ..
        } = self;
        let table_name = base_struct.table_name();
        let iterable_remaining_elements = base_struct.remaining_elements();
        // let iterable_remaining_elements: Vec<_> = members
        //     .iter()
        //     .filter(|m| !m.is_skipped && !m.is_primary && m.has_vec())
        //     .map(|m| format_ident!("{}_silo_remaining_elements", m.name))
        //     .collect();
        let value_type_name = &base_struct.name;
        let filter_name = base_struct.filter_name();
        let partial_name = base_struct.partial_name();

        quote! {
        #visibility struct #table_name<'a> {
            connection: &'a silo::rusqlite::Connection,
            string_storage: std::sync::Arc<std::sync::Mutex<silo::StaticStringStorage>>,
        }

        impl<'a> #table_name<'a> {
            fn default_order() -> silo::GenericOrder {
                let mut result = silo::GenericOrder::default();
                #(result.add(stringify!(#iterable_remaining_elements), silo::Ordering {
                    asc_desc: Some(silo::OrderingAscDesc::Descending),
                    nulls: Some(silo::OrderingNulls::NullsLast),
                });)*
                result
            }
        }


        impl<'a> silo::SqlTable<'a> for #table_name<'a> {
            type RowType = #value_type_name;
            type ValueType = #value_type_name;

            const INSERT_FAILURE_BEHAVIOR: silo::SqlFailureBehavior = #on_conflict;


            fn insert(&self, row: impl silo::ToRows<Self::RowType>) -> Result<(), silo::rusqlite::Error> {
                silo::insert_into_table(&self.connection, row, Self::INSERT_FAILURE_BEHAVIOR)?;
                Ok(())
            }

            fn filter(&self, filter: #filter_name) -> Result<Vec<#value_type_name>, silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let mut generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
                silo::query_table_filtered::<Self::RowType, Self::ValueType>(&self.connection, &mut self.string_storage.lock().unwrap(), generic, Self::default_order())
            }

            fn delete(&self, filter: #filter_name) -> Result<usize, silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
                silo::delete_table_filtered::<Self::RowType>(&self.connection, generic)
            }


            fn update(&self, filter: #filter_name, updated: #partial_name) -> Result<(), silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
                silo::update_rows::<Self::RowType>(&self.connection, generic, updated)?;
                Ok(())
            }

            fn migrate(&self, actual_columns: &[silo::SqlColumn]) -> Result<(), silo::rusqlite::Error> {
                silo::handle_migration::<Self::RowType>(
                    self.connection,
                    &mut self.string_storage.lock().unwrap(),
                    actual_columns,
                )
            }

            fn from_connection(connection: &'a silo::rusqlite::Connection, string_storage: std::sync::Arc<std::sync::Mutex<silo::StaticStringStorage>>) -> Self {
                Self { connection, string_storage }
            }
        }
        }
    }

    fn create_filter(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(filter::create_filter_for(&self.base_struct));
    }

    fn create_conversions(&self, tokens: &mut proc_macro2::TokenStream) {
        if self.base_struct.variant_field().is_some() {
            enum_helper::create_enum_helper_for(&self.base_struct, tokens);
        }
        from_row::create_from_row_for(&self.base_struct, tokens);
        partial::create_partial_for(&self.base_struct, false, tokens);
        as_params::create_as_params(&self.base_struct, tokens);
    }

    fn create_into_sql_table(&self) -> proc_macro2::TokenStream {
        let mut tokens = into_sql_table::create_into_sql_table(&self.base_struct);
        tokens
    }

    fn create_row_type(&self) -> proc_macro2::TokenStream {
        row_type::create_row_type(&self.base_struct)
    }
}

impl ToTokens for Base {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.create_filter(tokens);
        let table = self.create_table();
        tokens.extend(table);
        tokens.extend(self.create_into_sql_table());
        tokens.extend(self.create_row_type());
        self.migration_handler.to_tokens(tokens);
        self.create_conversions(tokens);
    }
}

// #[macro_export]
#[proc_macro_derive(IntoSqlTable, attributes(silo))]
pub fn derive_into_sql_table(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input)
        .expect("This is a derive macro and should be used with structs or enums.");

    let base = match input.data {
        syn::Data::Struct(data_struct) => {
            Base::from_struct(input.attrs, input.ident, input.vis, data_struct)
        }
        syn::Data::Enum(data_enum) => {
            Base::from_enum(input.attrs, input.ident, input.vis, data_enum)
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
