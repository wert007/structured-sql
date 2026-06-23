use quote::{ToTokens, quote};
use syn::{Ident, Visibility};

mod as_params;
mod attributes;
mod base_struct;
mod enum_helper;
mod filter;
mod from_row;
mod from_row_type;
mod into_sql_table;
mod partial;
mod row_type;
mod to_columns;

pub struct ToTable {
    visibility: Visibility,
    variants: Option<Vec<Ident>>,
    base_struct: base_struct::StructData,
    on_conflict: proc_macro2::TokenStream,
}

impl std::fmt::Debug for ToTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Base")
            .field("variants", &self.variants)
            // .field("members", &self.members)
            .finish()
    }
}
impl ToTable {
    pub fn from_struct(
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
        Ok(Self {
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
    ) -> Result<ToTable, crate::error::Error> {
        let attribute_struct_data = attributes::AttributeStructData::parse(&attrs);
        let on_conflict = attribute_struct_data.on_conflict();
        let variants = data_enum.variants.iter().map(|v| v.ident.clone()).collect();
        let base_struct: base_struct::StructData = base_struct::StructData::from_enum_data(
            visibility.clone(),
            name.clone(),
            data_enum.variants,
        )?;

        Ok(Self {
            visibility,
            variants: Some(variants),
            on_conflict,
            base_struct,
        })
    }

    fn create_table(&self) -> proc_macro2::TokenStream {
        let ToTable {
            visibility,
            base_struct,
            ..
        } = self;
        let table_name = base_struct.table_name();
        let value_type_name = &base_struct.name;
        let filter_name = base_struct.filter_name();
        let partial_name = base_struct.partial_name();

        quote! {
            #visibility struct #table_name<'a> {
                connection: &'a silo::rusqlite::Connection,
            }

            impl<'a> silo::SqlTable<'a> for #table_name<'a> {
                type RowType = #value_type_name;
                type ValueType = #value_type_name;
                type FilterType = #filter_name;

                fn connection(&self) -> &'a silo::rusqlite::Connection {
                    self.connection
                }

                fn insert(&self, row: Self::RowType) -> std::result::Result<bool, silo::rusqlite::Error> {
                    silo::insert_into_table(&self.connection, row)
                }

                fn load_where(&self, filter: impl Into<Self::FilterType>) -> std::result::Result<Vec<Self::RowType>, silo::rusqlite::Error> {
                    silo::load_where(&self.connection, filter)
                }
                fn update(&self, filter: impl Into<Self::FilterType>, updated: #partial_name) -> std::result::Result<usize, silo::rusqlite::Error> {
                    silo::update::<#value_type_name, #partial_name, Self::FilterType>(&self.connection, filter, updated)
                }

                fn from_connection(connection: &'a silo::rusqlite::Connection) -> Self {
                    Self { connection }
                }
            }
        }
    }

    // fn create_filter(&self, tokens: &mut proc_macro2::TokenStream) {
    //     tokens.extend(filter::create_filter_for(&self.base_struct));
    // }

    fn create_conversions(&self, tokens: &mut proc_macro2::TokenStream) {
        // if self.base_struct.variant_field().is_some() {
        //     enum_helper::create_enum_helper_for(&self.base_struct, tokens);
        // }
        from_row::create_from_row_for(&self.base_struct, tokens);
        partial::create_partial_for(&self.base_struct, tokens);
        // TODO: ToColumns would use false here!
        as_params::create_as_params(&self.base_struct, tokens, true);
        // as_params::create_as_params_for_pk(&self.base_struct, tokens);
        // if let Some(pk) = self.base_struct.primary_key_field() {
        //     to_columns::create_to_columns_for_pk(&self.base_struct, pk, tokens)
        // }
    }

    fn create_into_sql_table(&self) -> proc_macro2::TokenStream {
        into_sql_table::create_into_sql_table(&self.base_struct)
    }

    fn create_filter(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(filter::create_filter_for(&self.base_struct));
    }
}

impl ToTokens for ToTable {
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
