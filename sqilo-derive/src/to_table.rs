use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Generics, Ident, Lifetime, LifetimeParam, Visibility};

use crate::{
    attributes::{self, ToTableAttributesStruct},
    base_struct,
};

pub mod as_params;
pub mod filter;
pub mod from_row;
mod from_row_type;
mod into_sql_table;
pub mod partial;
mod row_type;

pub struct ToTableStruct {
    visibility: Visibility,
    variants: Option<Vec<Ident>>,
    base_struct: base_struct::StructData,
    on_conflict: proc_macro2::TokenStream,
    attr: ToTableAttributesStruct,
}

impl std::fmt::Debug for ToTableStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Base")
            .field("variants", &self.variants)
            // .field("members", &self.members)
            .finish()
    }
}
impl ToTableStruct {
    pub fn from_struct(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        generics: Generics,
        data_struct: syn::DataStruct,
    ) -> Result<Self, crate::error::Error> {
        let attribute_struct_data = attributes::ToTableAttributesStruct::parse(&attrs)?;
        let on_conflict = attribute_struct_data.on_conflict();
        let base_struct: base_struct::StructData = base_struct::StructData::from_struct_data(
            visibility.clone(),
            name.clone(),
            generics,
            data_struct.fields,
        )?;
        Ok(Self {
            attr: attribute_struct_data,
            visibility,
            variants: None,
            base_struct,
            on_conflict,
        })
    }

    pub fn from_enum(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_enum: syn::DataEnum,
    ) -> Result<ToTableStruct, crate::error::Error> {
        todo!()
        // let attribute_struct_data = attributes::ToTableAttributesStruct::parse(&attrs)?;
        // let on_conflict = attribute_struct_data.on_conflict();
        // let variants = data_enum.variants.iter().map(|v| v.ident.clone()).collect();
        // let base_struct: base_struct::StructData = base_struct::StructData::from_enum_data(
        //     visibility.clone(),
        //     name.clone(),
        //     data_enum.variants,
        //     a
        // )?;

        // Ok(Self {
        //     visibility,
        //     variants: Some(variants),
        //     on_conflict,
        //     base_struct,
        // })
    }

    fn create_table(&self) -> proc_macro2::TokenStream {
        let ToTableStruct {
            visibility,
            base_struct,
            ..
        } = self;
        let name = &base_struct.name;
        let table_name = base_struct.table_name();
        let value_type_name = &base_struct.name;
        let filter_name = base_struct.filter_name();
        let partial_name = base_struct.partial_name();
        let b = base_struct.generics_names_only(TokenStream::new());
        let b_tupled = base_struct.generics_names_only_tupled(TokenStream::new());
        let ab = base_struct.generics_names_only(quote! {'__silo__a,});
        let mut a = base_struct.generics.clone();
        a.params.insert(
            0,
            syn::GenericParam::Lifetime(LifetimeParam::new(Lifetime::new(
                "'__silo__a",
                Span::call_site(),
            ))),
        );
        let c = &base_struct.where_clause;
        let marker = if base_struct.generics.params.is_empty() {
            quote! {()}
        } else {
            quote! { std::marker::PhantomData #b_tupled }
        };

        quote! {

            #visibility struct #table_name #a #c {
                connection: &'__silo__a sqilo::rusqlite::Connection,
                _marker: #marker,
            }

            impl #a sqilo::SqlTable<'__silo__a> for #table_name #ab #c {
                type RowType = #value_type_name #b;
                type ValueType = #value_type_name #b;
                type FilterType = #filter_name #b;

                fn connection(&self) -> &'__silo__a sqilo::rusqlite::Connection {
                    self.connection
                }

                fn update(&self, filter: impl Into<Self::FilterType>, updated: #partial_name #b) -> std::result::Result<usize, sqilo::rusqlite::Error> {
                    use sqilo::partial::PartialType;
                    if updated.is_empty() {return Ok(0)}
                    sqilo::update::<#value_type_name #b, #partial_name #b, Self::FilterType>(&self.connection, filter, updated)
                }

                fn from_connection(connection: &'__silo__a sqilo::rusqlite::Connection) -> Self {
                    Self { connection, _marker: Default::default(), }
                }
            }
        }
    }

    fn create_conversions(&self, tokens: &mut proc_macro2::TokenStream) {
        from_row::create_from_row_for(&self.base_struct, tokens);
        partial::create_partial_for(&self.base_struct, tokens);
        as_params::create_as_params_for_struct(&self.base_struct, tokens, true);
    }

    fn create_into_sql_table(&self) -> proc_macro2::TokenStream {
        into_sql_table::create_into_sql_table(&self.base_struct, &self.attr)
    }

    fn create_filter(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(filter::create_filter_for(&self.base_struct));
    }
}

impl ToTokens for ToTableStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // self.create_filter(tokens);
        let table = self.create_table();
        tokens.extend(table);
        tokens.extend(self.create_into_sql_table());
        // tokens.extend(self.create_row_type());
        // self.migration_handler.to_tokens(tokens);
        self.create_conversions(tokens);
        self.create_filter(tokens);
        // let path = format!("dbg/to-table-for-{}.rs", self.base_struct.name);
        // std::fs::write(&path, tokens.to_string()).unwrap();
        // std::process::Command::new("rustfmt")
        //     .args([
        //         "--emit",
        //         "files",
        //         "--edition",
        //         "2024",
        //         "--style-edition",
        //         "2024",
        //         &path,
        //     ])
        //     .spawn()
        //     .unwrap()
        //     .wait()
        //     .unwrap();
    }
}
