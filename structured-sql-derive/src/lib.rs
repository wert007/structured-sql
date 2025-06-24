use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::{Type, spanned::Spanned};

#[macro_export]
#[proc_macro_derive(IntoSqlTable)]
pub fn derive_into_sql_table(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input).unwrap();

    match input.data {
        syn::Data::Struct(data_struct) => derive_onto_struct(input.ident, data_struct),
        syn::Data::Enum(data_enum) => derive_onto_enum(input.ident, data_enum),
        syn::Data::Union(data_union) => todo!(),
    }
}

fn derive_onto_enum(enum_name: syn::Ident, data_enum: syn::DataEnum) -> TokenStream {
    let filter_name = syn::Ident::new(&format!("{enum_name}Filter"), enum_name.span());
    let table_name = syn::Ident::new(&format!("{enum_name}Table"), enum_name.span());

    let variants: Vec<_> = data_enum.variants.pairs().map(|p| p.into_value()).collect();
    let variant_names: Vec<_> = variants.iter().map(|v| v.ident.clone()).collect();
    let variant_matches: Vec<_> = variants
        .iter()
        .map(|v| {
            let name = &v.ident;
            match &v.fields {
                syn::Fields::Named(_fields_named) => quote! { #enum_name::#name { .. }},
                syn::Fields::Unnamed(_fields_unnamed) => quote! { #enum_name::#name(..)},
                syn::Fields::Unit => quote! { #enum_name::#name},
            }
        })
        .collect();
    let match_field_names = variants.iter().flat_map(|v| match &v.fields {
        syn::Fields::Named(fields_named) => fields_named
            .named
            .pairs()
            .map(|p| {
                let field_name = p.value().ident.as_ref().expect("Should be named");
                let variant_name = &v.ident;
                let name = syn::Ident::new(
                    &format!("{}_{}", v.ident.to_string().to_lowercase(), field_name),
                    p.value().span(),
                );
                quote! {#enum_name::#variant_name { #field_name: #name, .. }}
            })
            .collect::<Vec<_>>(),
        syn::Fields::Unnamed(fields_unnamed) => fields_unnamed
            .unnamed
            .pairs()
            .enumerate()
            .map(|(i, p)| {
                let variant_name = &v.ident;
                let mut match_pattern = Vec::new();
                for _ in 0..i {
                    match_pattern.push(quote! {_,});
                }
                let name = syn::Ident::new(
                    &format!("{}_{}", v.ident.to_string().to_lowercase(), i),
                    p.value().span(),
                );
                match_pattern.push(quote! {#name, ..});
                quote! {#enum_name::#variant_name(#(#match_pattern)*)}
            })
            .collect::<Vec<_>>(),
        syn::Fields::Unit => vec![],
    });
    let construct_variants = variants.iter().map(|v| {
        let variant_name = &v.ident;
        match &v.fields {
            syn::Fields::Named(fields_named) => {
                let field_names = fields_named
                    .named
                    .pairs()
                    .map(|p| p.value().ident.as_ref().unwrap())
                    .map(|i| {
                        let name = syn::Ident::new(&format!("{}_{i}", variant_name.to_string().to_lowercase()), i.span());
                        quote! { #name }
                    });

                let actual_field_names = fields_named
                    .named
                    .pairs()
                    .map(|p| p.value().ident.as_ref().unwrap());
                quote! {Self::#variant_name { #(#actual_field_names: #field_names.expect("Correct Variant"),)*}}
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let field_names = v.fields.members().enumerate().map(|(i, m)| {
                    let name = syn::Ident::new(&format!("{}_{i}", variant_name.to_string().to_lowercase()), m.span());
                    quote! { #name }
                });
                quote! {Self::#variant_name(#(#field_names.expect("Correct Variant"),)*)}
            }
            syn::Fields::Unit => quote! {Self::#variant_name},
        }
    });
    let (field_names, field_types): (Vec<_>, Vec<_>) = variants
        .iter()
        .flat_map(|v| match &v.fields {
            syn::Fields::Named(fields_named) => fields_named
                .named
                .pairs()
                .map(|p| {
                    (
                        syn::Ident::new(
                            &format!(
                                "{}_{}",
                                v.ident.to_string().to_lowercase(),
                                p.value().ident.as_ref().expect("Should be named")
                            ),
                            p.value().span(),
                        ),
                        p.value().ty.clone(),
                    )
                })
                .collect::<Vec<_>>(),
            syn::Fields::Unnamed(fields_unnamed) => fields_unnamed
                .unnamed
                .pairs()
                .enumerate()
                .map(|(i, p)| {
                    (
                        syn::Ident::new(
                            &format!("{}_{i}", v.ident.to_string().to_lowercase(),),
                            p.value().span(),
                        ),
                        p.value().ty.clone(),
                    )
                })
                .collect(),
            syn::Fields::Unit => Vec::new(),
        })
        .collect();
    let field_types = &field_types;
    let param_counts = field_types.iter().map(|t| {
        if convert_type_to_sql_column_type(t).is_some() {
            quote! { 1 }
        } else {
            quote! {#t::COLUMNS.len()}
        }
    });
    let try_get_field_names: Vec<_> = field_names
        .iter()
        .map(|n| syn::Ident::new(&format!("try_get_{}", n), n.span()))
        .collect();
    let columns: Vec<_> = field_names
        .iter()
        .zip(field_types)
        .map(|(n, t)| {
            let type_ = convert_type_to_sql_column_type(&t);
            match type_ {
                Some(type_) => {
                    quote! { &[structured_sql::SqlColumn {
                        name: stringify!(#n),
                        r#type: #type_,
                    }], }
                }
                None => {
                    quote! {
                        #t::COLUMNS,
                    }
                }
            }
        })
        .collect();
    quote! {
        impl #enum_name {
            pub const VARIANTS: &'static [&'static str] = &[#(stringify!(#variant_names),)*];

            pub fn as_variant(&self) -> &'static &'static str {
                match self {
                    #(#variant_matches => &stringify!(#variant_names),)*
                }
            }

            #(pub fn #try_get_field_names(&self) -> Option<&#field_types> {
                match self {
                    #match_field_names => Some(&#field_names),
                    _ => None
                }
            })*
        }

        #[derive(Default, Clone, Debug)]
        struct #filter_name {
            filter: structured_sql::SqlColumnFilter<&'static str>,
        }

        impl structured_sql::IntoSqlColumnFilter for #filter_name {
            fn into_sql_column_filter(
                self,
                name: &'static str,
            ) -> Vec<(&'static str, structured_sql::SqlColumnFilter<structured_sql::SqlValue>)> {
                self.filter.into_sql_column_filter("variant")
            }
        }


        impl Into<structured_sql::GenericFilter> for #filter_name {
            fn into(self) -> structured_sql::GenericFilter {
                structured_sql::GenericFilter {columns: [("variant",  self.filter.into_generic())].into_iter().collect()}
            }
        }

        impl<'a> structured_sql::IntoSqlTable<'a> for #enum_name {
            type Filter = #filter_name;
            type Table = #table_name<'a>;
            const COLUMNS: &'static [structured_sql::SqlColumn] =

            &structured_sql::konst::slice::slice_concat!{structured_sql::SqlColumn ,&[
                &[structured_sql::SqlColumn {name: "variant", r#type: structured_sql::SqlColumnType::Text}],
                #(#columns)*
            ]};

            const NAME: &'static str = stringify!(#enum_name);

            fn as_params(&self) -> Vec<&dyn structured_sql::rusqlite::ToSql> {
                use structured_sql::AsParams;
                let mut result: Vec<&dyn structured_sql::rusqlite::ToSql> = vec![self.as_variant()];
                #(if let Some(value) = self.#try_get_field_names() {
                    result.extend(value.as_params());
                } else {
                    for _ in 0..#param_counts {
                        result.push(&None::<&dyn structured_sql::rusqlite::ToSql>);

                    }
                })*

                result
            }
        }

        struct #table_name<'a> {
            connection: &'a structured_sql::rusqlite::Connection,
        }


        impl structured_sql::FromRow for #enum_name {
            fn from_row(column_name: Option<&'static str>, row: &structured_sql::rusqlite::Row) -> Self {
                let variant: String = row.get("variant").unwrap();
                #(let #field_names: Option<#field_types> = row.get(stringify!(#field_names)).unwrap();)*
                match variant.as_str() {
                    #(stringify!(#variant_names) => #construct_variants,)*
                    _ => unreachable!("Unknown variant found!"),
                }
            }
        }

        impl<'a> structured_sql::SqlTable<'a> for #table_name<'a> {
            type RowType = #enum_name;

            fn insert(&self, row: Self::RowType) -> Result<(), structured_sql::rusqlite::Error> {
                let columns = Self::RowType::COLUMNS.into_iter().map(|c| c.name).fold(
                    String::new(),
                    |mut acc, cur| {
                        if acc.is_empty() {
                            cur.into()
                        } else {
                            acc.push_str(", ");
                            acc.push_str(cur);
                            acc
                        }
                    },
                );
                let values = (0..Self::RowType::COLUMNS.len()).map(|v| v + 1).fold(
                    String::new(),
                    |mut acc, cur| {
                        if acc.is_empty() {
                            format!("?{cur}")
                        } else {
                            acc.push_str(", ?");
                            acc.push_str(&cur.to_string());
                            acc
                        }
                    },
                );
                let sql = format!(
                        "INSERT INTO {} ({columns}) VALUES ({values})",
                        Self::RowType::NAME
                    );
                self.connection.execute(
                    &sql,
                    row.as_params().as_slice(),
                )?;
                Ok(())
            }

            fn filter(&self, filter: #filter_name) -> Result<Vec<#enum_name>, structured_sql::rusqlite::Error> {
                let generic: structured_sql::GenericFilter = filter.into();
                structured_sql::query_table_filtered::<Self::RowType>(&self.connection, generic)
            }

            fn from_connection(connection: &'a structured_sql::rusqlite::Connection) -> Self {
                Self { connection }
            }
        }
    }
    .into()
}

fn derive_onto_struct(struct_name: syn::Ident, members: syn::DataStruct) -> TokenStream {
    let filter_name = syn::Ident::new(&format!("{struct_name}Filter"), struct_name.span());
    let table_name = syn::Ident::new(&format!("{struct_name}Table"), struct_name.span());

    let field_names: Vec<_> = members
        .fields
        .iter()
        .map(|f| {
            f.ident
                .as_ref()
                .expect("Can only be used on structs with named fields")
        })
        .collect();
    let field_names = &field_names;
    let field_types: Vec<_> = members.fields.iter().map(|f| &f.ty).collect();
    let field_types = &field_types;
    let field_types_filter = field_types.iter().map(|f| {
        let name = f.to_token_stream().to_string();
        syn::Ident::new(&format!("{name}Filter"), f.span())
    });
    let columns: Vec<proc_macro2::TokenStream> = field_types
        .iter()
        .zip(field_names)
        .map(|(t, n)| {
            let type_ = convert_type_to_sql_column_type(t);
            match type_ {
                Some(type_) => {
                    quote! { &[structured_sql::SqlColumn {
                        name: stringify!(#n),
                        r#type: #type_,
                    }], }
                }
                None => {
                    quote! {
                        #t::COLUMNS,
                    }
                }
            }
        })
        .collect();
    let columns = proc_macro2::TokenStream::from_iter(columns);
    quote! {
            use structured_sql::filters::*;
        #[derive(Default, Clone, Debug)]
        struct #filter_name {
            #(#field_names: Option<structured_sql::SqlColumnFilter<#field_types_filter>>,)*
        }
        impl #filter_name {
            pub fn into_generic(self) -> structured_sql::GenericFilter {
                self.into()
            }
        }


    impl Into<structured_sql::GenericFilter> for #filter_name {
        fn into(self) -> structured_sql::GenericFilter {
            let mut columns = std::collections::HashMap::new();
            #(if let Some(#field_names) = self.#field_names {
                structured_sql::GenericFilter::insert_into_columns(stringify!(#field_names), &mut columns, #field_names);
            })*
            structured_sql::GenericFilter { columns }
        }
    }


        struct #table_name<'a> {
            connection: &'a structured_sql::rusqlite::Connection,
        }


    impl<'a> structured_sql::SqlTable<'a> for #table_name<'a> {
        type RowType = #struct_name;

        fn insert(&self, row: Self::RowType) -> Result<(), structured_sql::rusqlite::Error> {
            let columns = Self::RowType::COLUMNS.into_iter().map(|c| c.name).fold(
                String::new(),
                |mut acc, cur| {
                    if acc.is_empty() {
                        cur.into()
                    } else {
                        acc.push_str(", ");
                        acc.push_str(cur);
                        acc
                    }
                },
            );
            let values = (0..Self::RowType::COLUMNS.len()).map(|v| v + 1).fold(
                String::new(),
                |mut acc, cur| {
                    if acc.is_empty() {
                        format!("?{cur}")
                    } else {
                        acc.push_str(", ?");
                        acc.push_str(&cur.to_string());
                        acc
                    }
                },
            );

            let sql = format!(
                    "INSERT INTO {} ({columns}) VALUES ({values})",
                    Self::RowType::NAME
                );
            self.connection.execute(
                &sql,
                row.as_params().as_slice(),
            )?;
            Ok(())
        }

        fn filter(&self, filter: #filter_name) -> Result<Vec<#struct_name>, structured_sql::rusqlite::Error> {
            let generic: structured_sql::GenericFilter = filter.into();
            structured_sql::query_table_filtered::<Self::RowType>(&self.connection, generic)
        }

        fn from_connection(connection: &'a structured_sql::rusqlite::Connection) -> Self {
            Self { connection }
        }
    }


    impl structured_sql::IntoSqlColumnFilter for #filter_name {
        fn into_sql_column_filter(
            self,
            name: &'static str,
        ) -> Vec<(&'static str, structured_sql::SqlColumnFilter<structured_sql::SqlValue>)> {
            use structured_sql::IntoSqlColumnFilter;
            let mut result = Vec::new();
            #(if let Some(#field_names) = self.#field_names {
                result.extend(#field_names.into_sql_column_filter(stringify!(#field_names)));
        })*
            result
        }
    }

    impl structured_sql::FromRow for #struct_name {
        fn from_row(row_name: Option<&'static str>, row: &structured_sql::rusqlite::Row) -> Self {
            use structured_sql::rusqlite::OptionalExtension;
            #(let #field_names: #field_types = #field_types::from_row(Some(stringify!(#field_names)), row);)*
            Self {#( #field_names),*}
        }
    }

    impl<'a> structured_sql::IntoSqlTable<'a> for #struct_name {
        type Filter = #filter_name;
        type Table = #table_name<'a>;
        const COLUMNS: &'static [structured_sql::SqlColumn] = &structured_sql::konst::slice::slice_concat!{structured_sql::SqlColumn ,&[
            #columns
        ]};

        const NAME: &'static str = stringify!(#table_name);

        fn as_params(&self) -> Vec<&dyn structured_sql::rusqlite::ToSql> {
            use structured_sql::AsParams;
            let mut result = Vec::new();
            #(result.extend(&self.#field_names.as_params()));*
            ;
            result
        }
    }
    }
    .into()
}

fn convert_type_to_sql_column_type(t: &Type) -> Option<proc_macro2::TokenStream> {
    match t {
        Type::Path(type_path) => {
            let type_ident = &type_path.path.segments.last().unwrap().ident;
            Some(match type_ident.to_string().as_str() {
                "f64" | "f32" => {
                    quote! { structured_sql::SqlColumnType::Float }
                }
                "bool" | "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64"
                | "u128" => {
                    quote! { structured_sql::SqlColumnType::Integer }
                }
                "String" => {
                    quote! { structured_sql::SqlColumnType::Text }
                }
                _ => return None,
            })
        }
        _ => unreachable!("Only simple types can be converted into columns"),
    }
}
