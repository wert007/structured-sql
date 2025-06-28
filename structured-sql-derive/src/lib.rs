use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, Ident, Lit, LitInt, Type, TypePath, Visibility, spanned::Spanned};

enum StructuredAttributeArguments {
    Identifier(String),
}
impl StructuredAttributeArguments {
    fn new(argument: syn::Expr) -> Option<Self> {
        match argument {
            syn::Expr::Path(path) => Some(Self::Identifier(path.path.get_ident()?.to_string())),
            _ => None,
        }
    }
}

struct StructuredAttribute {
    path: String,
    arguments: StructuredAttributeArguments,
}
impl StructuredAttribute {
    fn new(attribute: &Attribute) -> Option<Self> {
        let path = attribute.path().get_ident()?.to_string();
        let arguments = StructuredAttributeArguments::new(attribute.parse_args().ok()?)?;
        Some(Self { path, arguments })
    }
}

#[derive(Debug, Default)]
struct AttributeFieldData {
    is_primary: bool,
    is_unique: bool,
}
impl AttributeFieldData {
    fn parse(attrs: &[Attribute]) -> AttributeFieldData {
        let mut this = Self::default();
        for attribute in attrs {
            let Some(attribute) = StructuredAttribute::new(attribute) else {
                panic!("Invalid attribute");
            };
            if attribute.path != "silo" {
                panic!("Invalid attribute");
            }
            match attribute.arguments {
                StructuredAttributeArguments::Identifier(name) => match name.as_str() {
                    "primary" => this.is_primary = true,
                    "unique" => this.is_unique = true,
                    _ => {
                        panic!("Invalid attribute");
                    }
                },
            }
        }
        this
    }
}

struct Member {
    variant: Ident,
    name: Ident,
    visibility: Visibility,
    type_: Type,
    is_primary: bool,
    is_unique: bool,
    is_optional: bool,
    name_is_generated: bool,
}

impl std::fmt::Debug for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Member")
            .field("variant", &self.variant)
            .field("name", &self.name)
            .field("is_primary", &self.is_primary)
            .field("is_unique", &self.is_unique)
            .field("is_optional", &self.is_optional)
            .field("name_is_generated", &self.name_is_generated)
            .finish()
    }
}
impl Member {
    fn from_struct_fields(struct_name: syn::Ident, fields: syn::Fields) -> Vec<Member> {
        let mut field_index = 0;
        match fields {
            syn::Fields::Named(fields_named) => {
                Self::from_named_fields(struct_name, fields_named, false)
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                Self::from_unnamed_fields(&mut field_index, struct_name, fields_unnamed, false)
            }
            syn::Fields::Unit => Vec::new(),
        }
    }

    fn from_named_fields(
        variant: Ident,
        fields: syn::FieldsNamed,
        is_optional: bool,
    ) -> Vec<Member> {
        fields
            .named
            .iter()
            .enumerate()
            .map(|(i, f)| Member::from_field(i, variant.clone(), f, is_optional))
            .collect()
    }

    fn from_unnamed_fields(
        base: &mut usize,
        variant: Ident,
        fields: syn::FieldsUnnamed,
        is_optional: bool,
    ) -> Vec<Member> {
        fields
            .unnamed
            .iter()
            .map(|f| {
                let result = Member::from_field(*base, variant.clone(), f, is_optional);
                *base += 1;
                result
            })
            .collect()
    }

    fn from_field(index: usize, variant: syn::Ident, f: &syn::Field, is_optional: bool) -> Member {
        let AttributeFieldData {
            is_primary,
            is_unique,
        } = AttributeFieldData::parse(&f.attrs);
        let name_is_generated = f.ident.is_none();
        let name = f
            .ident
            .clone()
            .unwrap_or_else(|| format_ident!("unnamed{index}"));
        Member {
            variant,
            name,
            visibility: f.vis.clone(),
            type_: f.ty.clone(),
            is_primary,
            is_unique,
            is_optional,
            name_is_generated,
        }
    }

    fn from_enum_variants(
        variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    ) -> Vec<Member> {
        let mut base = 0;
        variants
            .iter()
            .flat_map(|v| Member::from_variant(&mut base, v))
            .collect()
    }

    fn from_variant(base: &mut usize, v: &syn::Variant) -> Vec<Member> {
        match v.fields.clone() {
            syn::Fields::Named(fields_named) => {
                Self::from_named_fields(v.ident.clone(), fields_named, true)
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                Self::from_unnamed_fields(base, v.ident.clone(), fields_unnamed, true)
            }
            syn::Fields::Unit => Vec::new(),
        }
    }

    fn create_filter_field(&self) -> proc_macro2::TokenStream {
        let Member { name, type_, .. } = self;
        quote! { #name: <#type_ as structured_sql::Filterable>::Filtered}
    }

    fn create_field_name(&self) -> proc_macro2::TokenStream {
        let Member { name, .. } = self;
        quote! { #name }
    }

    fn create_column_definition(&self) -> proc_macro2::TokenStream {
        let Member {
            name,
            type_,
            is_primary,
            is_unique,
            is_optional,
            ..
        } = self;
        let is_unique = syn::LitBool::new(*is_unique, name.span());
        let is_primary = syn::LitBool::new(*is_primary, name.span());
        if let Some(t) = Member::as_simple_type(type_, *is_optional) {
            quote! { &[structured_sql::SqlColumn {
                name: stringify!(#name),
                r#type: #t,
                is_unique: #is_unique,
                is_primary: #is_primary,
            }] }
        } else {
            quote! { < #type_ as structured_sql::IntoSqlTable>::COLUMNS }
        }
    }

    fn create_field_type(&self) -> proc_macro2::TokenStream {
        let Member { type_, .. } = self;
        quote! { #type_ }
    }

    fn create_variant_pattern(
        variants: &[Ident],
        members: &[Member],
    ) -> Vec<proc_macro2::TokenStream> {
        variants
            .iter()
            .map(|v| {
                let members = Member::get_relevant_members_for_variant(v, members);
                let member_names = members.iter().copied().map(Member::create_field_name);
                if members.is_empty() {
                    quote!(#v)
                } else if members[0].name_is_generated {
                    quote!(#v(#(#member_names,)*))
                } else {
                    quote!(#v{#(#member_names,)*})
                }
            })
            .collect()
    }

    fn create_variant_field_names(
        variants: &[Ident],
        members: &[Member],
    ) -> Vec<Vec<proc_macro2::TokenStream>> {
        variants
            .iter()
            .map(|v| {
                let members = Member::get_relevant_members_for_variant(v, members);
                members
                    .iter()
                    .copied()
                    .map(Member::create_field_name)
                    .collect()
            })
            .collect()
    }
    fn create_variant_field_indices(
        variants: &[Ident],
        members: &[Member],
    ) -> Vec<Vec<proc_macro2::TokenStream>> {
        let mut index = 0;
        variants
            .iter()
            .map(|v| {
                let base = index;
                let members = Member::get_relevant_members_for_variant(v, members);
                index += members.len();
                (0..members.len())
                    .map(|i| {
                        let value = i + base;
                        let value = syn::LitInt::new(&value.to_string(), v.span());
                        quote! { #value}
                    })
                    .collect()
            })
            .collect()
    }

    // fn create_variant_creation(
    //     variants: &[Ident],
    //     members: &[Member],
    // ) -> Vec<proc_macro2::TokenStream> {
    //     let mut result = Vec::with_capacity(variants.len());
    //     result
    // }

    fn get_relevant_members_for_variant<'a>(v: &Ident, members: &'a [Member]) -> Vec<&'a Member> {
        members.iter().filter(|m| &m.variant == v).collect()
    }

    fn as_simple_type(type_: &Type, is_optional: bool) -> Option<proc_macro2::TokenStream> {
        match type_ {
            Type::Path(type_path) => Member::path_as_simple_type(&type_path.path, is_optional),
            _ => None,
        }
    }

    fn path_as_simple_type(
        path: &syn::Path,
        is_optional: bool,
    ) -> Option<proc_macro2::TokenStream> {
        for segment in &path.segments {
            if let Some(result) = Member::ident_as_simple_type(&segment.ident, is_optional) {
                return Some(result);
            }
            return match segment.ident.to_string().as_str() {
                "Option" => match &segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        match angle_bracketed_generic_arguments.args.iter().next()? {
                            syn::GenericArgument::Type(t) => Member::as_simple_type(t, true),
                            _ => None,
                        }
                    }
                    _ => None,
                },
                _ => None,
            };
        }
        None
    }

    fn ident_as_simple_type(ident: &Ident, is_optional: bool) -> Option<proc_macro2::TokenStream> {
        match ident.to_string().as_str() {
            "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "String"
            | "f32" | "f64" => {
                if is_optional {
                    Some(
                        quote! {< Option<#ident> as structured_sql::RelatedSqlColumnType>::SQL_COLUMN_TYPE},
                    )
                } else {
                    Some(
                        quote! {< #ident as structured_sql::RelatedSqlColumnType>::SQL_COLUMN_TYPE},
                    )
                }
            }
            _ => None,
        }
    }

    fn create_variant_empty_columns_before(
        variants: &[Ident],
        members: &[Member],
    ) -> Vec<proc_macro2::TokenStream> {
        let mut empty_columns = 1;
        variants
            .iter()
            .map(|v| {
                let result = empty_columns;
                empty_columns += Member::get_relevant_members_for_variant(v, members).len();
                LitInt::new(&result.to_string(), v.span()).into_token_stream()
            })
            .collect()
    }

    fn create_variant_names(
        variants: &[Ident],
        members: &[Member],
    ) -> Vec<proc_macro2::TokenStream> {
        variants.iter().map(|v| quote! {stringify!(#v)}).collect()
    }
}

struct Base {
    name: Ident,
    table_name: Ident,
    filter_name: Ident,
    visibility: Visibility,
    variants: Option<Vec<Ident>>,
    members: Vec<Member>,
}

impl std::fmt::Debug for Base {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Base")
            .field("name", &self.name)
            .field("table_name", &self.table_name)
            .field("filter_name", &self.filter_name)
            .field("variants", &self.variants)
            .field("members", &self.members)
            .finish()
    }
}
impl Base {
    fn from_struct(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_struct: syn::DataStruct,
    ) -> Self {
        let table_name = format_ident!("{name}Table");
        let filter_name = format_ident!("{name}Filter");
        let members = Member::from_struct_fields(name.clone(), data_struct.fields);
        // Add Partial types for Migration here!
        Self {
            name,
            table_name,
            filter_name,
            visibility,
            variants: None,
            members,
        }
    }

    fn from_enum(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_enum: syn::DataEnum,
    ) -> Base {
        let table_name = format_ident!("{name}Table");
        let filter_name = format_ident!("{name}Filter");
        let members = Member::from_enum_variants(&data_enum.variants);
        let variants = data_enum.variants.iter().map(|v| v.ident.clone()).collect();
        // Add Partial types for Migration here!
        Self {
            name,
            table_name,
            filter_name,
            visibility,
            variants: Some(variants),
            members,
        }
    }

    fn create_table(&self) -> proc_macro2::TokenStream {
        let Base {
            name,
            table_name,
            filter_name,
            visibility,
            ..
        } = self;
        quote! {
        #visibility struct #table_name<'a> {
            connection: &'a structured_sql::rusqlite::Connection,
        }


        impl<'a> structured_sql::SqlTable<'a> for #table_name<'a> {
            type RowType = #name;

            fn insert(&self, row: Self::RowType) -> Result<(), structured_sql::rusqlite::Error> {
                use structured_sql::AsParams;
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

            fn filter(&self, filter: #filter_name) -> Result<Vec<#name>, structured_sql::rusqlite::Error> {
                use structured_sql::IntoGenericFilter;
                let generic = filter.into_generic(None);
                structured_sql::query_table_filtered::<Self::RowType>(&self.connection, generic)
            }

            fn from_connection(connection: &'a structured_sql::rusqlite::Connection) -> Self {
                Self { connection }
            }
        }
        }
    }

    fn create_filter(&self) -> proc_macro2::TokenStream {
        if let Some(variants) = &self.variants {
            self.create_filter_enum(variants)
        } else {
            self.create_filter_struct()
        }
    }

    fn create_filter_struct(&self) -> proc_macro2::TokenStream {
        let Base {
            name,
            filter_name,
            visibility,
            members,
            ..
        } = self;

        let filter_field_names: Vec<_> = members.iter().map(|m| m.create_field_name()).collect();

        let filter_fields = members.iter().map(|m| m.create_filter_field());

        quote! {
            #[derive(Default, Clone, Debug)]
            #visibility struct #filter_name {
                #(#filter_fields,)*
            }

            impl structured_sql::Filterable for #name {
                type Filtered = #filter_name;
            }

            impl structured_sql::IntoGenericFilter for #filter_name {
                fn into_generic(self, column_name: Option<&'static str>) -> structured_sql::GenericFilter {
                    let mut columns = std::collections::HashMap::new();
                    #(
                        structured_sql::GenericFilter::insert_into_columns(stringify!(#filter_field_names), &mut columns, self.#filter_field_names);
                    )*
                    structured_sql::GenericFilter { columns }
                }
            }

            impl structured_sql::IntoSqlColumnFilter for #filter_name {
                fn into_sql_column_filter(
                    self,
                    name: &'static str,
                ) -> Vec<(&'static str, structured_sql::SqlColumnFilter<structured_sql::SqlValue>)> {
                    use structured_sql::IntoSqlColumnFilter;
                    let mut result = Vec::new();
                    #(
                        result.extend(self.#filter_field_names.into_sql_column_filter(stringify!(#filter_field_names)));
                    )*
                    result
                }
            }
        }
    }

    fn create_filter_enum(&self, _variants: &[syn::Ident]) -> proc_macro2::TokenStream {
        let Base {
            name,
            filter_name,
            visibility,
            ..
        } = self;

        // let filter_field_names: Vec<_> = members.iter().map(|m| m.create_field_name()).collect();

        // let filter_fields = members.iter().map(|m| m.create_filter_field());

        quote! {
            #[derive(Default, Clone, Debug)]
            #visibility struct #filter_name {
                variant: structured_sql::SqlColumnFilter<String>,
            }

            impl structured_sql::Filterable for #name {
                type Filtered = #filter_name;
            }

            impl structured_sql::IntoGenericFilter for #filter_name {
                fn into_generic(self, column_name: Option<&'static str>) -> structured_sql::GenericFilter {
                    let mut columns = std::collections::HashMap::new();
                    // TODO: Concat with column name!
                    structured_sql::GenericFilter::insert_into_columns("variant", &mut columns, self.variant);
                    structured_sql::GenericFilter { columns }
                }
            }

            impl structured_sql::IntoSqlColumnFilter for #filter_name {
                fn into_sql_column_filter(
                    self,
                    name: &'static str,
                ) -> Vec<(&'static str, structured_sql::SqlColumnFilter<structured_sql::SqlValue>)> {
                    use structured_sql::IntoSqlColumnFilter;
                    let mut result = Vec::new();
                    result.extend(self.variant.into_sql_column_filter("variant"));
                    result
                }
            }
        }
    }

    fn create_conversions(&self) -> proc_macro2::TokenStream {
        if let Some(variants) = &self.variants {
            self.create_conversions_enum(variants)
        } else {
            self.create_conversions_struct()
        }
    }

    fn create_conversions_struct(&self) -> proc_macro2::TokenStream {
        let Base {
            name,
            table_name,
            filter_name,
            members,
            ..
        } = self;
        let field_names_with_skips: Vec<_> =
            members.iter().map(|c| c.create_field_name()).collect();
        let field_types_with_skips: Vec<_> =
            members.iter().map(|c| c.create_field_type()).collect();
        let param_count = field_names_with_skips.len();
        let param_count = LitInt::new(&format!("{param_count}usize"), name.span());
        let field_names_without_skips: Vec<_> =
            members.iter().map(|c| c.create_field_name()).collect();
        let columns: Vec<_> = members
            .iter()
            .map(|m| m.create_column_definition())
            .collect();
        quote! {
            impl structured_sql::FromRow for #name {
                fn from_row(row_name: Option<&'static str>, row: &structured_sql::rusqlite::Row) -> Self {
                    use structured_sql::rusqlite::OptionalExtension;
                    #(let #field_names_with_skips = <#field_types_with_skips>::from_row(Some(stringify!(#field_names_with_skips)), row);)*
                    Self {#( #field_names_without_skips),*}
                }

                fn try_from_row(row_name: Option<&'static str>, row: &structured_sql::rusqlite::Row) -> Option<Self> {
                    use structured_sql::rusqlite::OptionalExtension;
                    #(let #field_names_with_skips = <#field_types_with_skips>::try_from_row(Some(stringify!(#field_names_with_skips)), row)?;)*
                    Some(Self {#( #field_names_without_skips),*})
                }
            }

            impl structured_sql::AsParams for #name {
                const PARAM_COUNT: usize = #param_count;
                fn as_params(&self) -> Vec<&dyn structured_sql::rusqlite::ToSql> {
                    use structured_sql::AsParams;
                    let mut result = Vec::new();
                    #(result.extend(&self.#field_names_with_skips.as_params()));*
                    ;
                    result
                }
            }

            impl<'a> structured_sql::IntoSqlTable<'a> for #name {
                type Filter = #filter_name;
                type Table = #table_name<'a>;
                const COLUMNS: &'static [structured_sql::SqlColumn] = &structured_sql::konst::slice::slice_concat!{structured_sql::SqlColumn ,&[
                    #(#columns,)*
                ]};

                const NAME: &'static str = stringify!(#table_name);
            }
        }
    }

    fn create_conversions_enum(&self, variants: &[syn::Ident]) -> proc_macro2::TokenStream {
        let Base {
            name,
            table_name,
            filter_name,
            members,
            ..
        } = self;
        let field_names_with_skips: Vec<_> =
            members.iter().map(|c| c.create_field_name()).collect();
        let field_types_with_skips: Vec<_> =
            members.iter().map(|c| c.create_field_type()).collect();
        let param_count = field_names_with_skips.len() + 1;
        let param_count = LitInt::new(&format!("{param_count}usize"), name.span());

        let columns: Vec<_> = members
            .iter()
            .map(|m| m.create_column_definition())
            .collect();
        let variant_pattern = Member::create_variant_pattern(variants, &members);
        let variant_empty_columns_before =
            Member::create_variant_empty_columns_before(variants, &members);
        let variant_names = Member::create_variant_names(variants, &members);
        let variant_field_names = Member::create_variant_field_names(variants, &members);
        let variant_field_indices = Member::create_variant_field_indices(variants, &members);
        // let variant_creation = Member::create_variant_creation(variants, &members);
        quote! {
            impl structured_sql::FromRow for #name {
                fn from_row(row_name: Option<&'static str>, row: &structured_sql::rusqlite::Row) -> Self {
                    use structured_sql::rusqlite::OptionalExtension;
                    let variant = String::from_row(Some("variant"), row);
                    #(let #field_names_with_skips = <#field_types_with_skips>::try_from_row(Some(stringify!(#field_names_with_skips)), row);)*
                    match variant.as_str() {
                        #(stringify!(#variants) => {
                            #(let #variant_field_names = #variant_field_names.expect("Column belongs to variant and should have value");)*
                            Self::#variant_pattern})*
                        _ => unreachable!("Unknown variant!")
                    }
                }

                fn try_from_row(row_name: Option<&'static str>, row: &structured_sql::rusqlite::Row) -> Option<Self> {
                    use structured_sql::rusqlite::OptionalExtension;
                    let variant = String::from_row(Some("variant"), row);
                    #(let #field_names_with_skips = <#field_types_with_skips>::try_from_row(Some(stringify!(#field_names_with_skips)), row);)*
                    Some(match variant.as_str() {
                        #(stringify!(#variants) => {

                            #(let #variant_field_names = #variant_field_names?;)*
                            Self::#variant_pattern
                        })*
                        _ => {return None;}
                    })}
            }

            impl #name {
                #[allow(unused_variables)]
                pub fn empty_columns_before(&self) -> usize {
                    match self {
                        #(Self::#variant_pattern => {
                            #variant_empty_columns_before
                        })*
                    }
                }

                #[allow(unused_variables)]
                pub fn variant_name(&self) -> &'static &'static str {
                    match self {
                        #(Self::#variant_pattern => {
                            &#variant_names
                        })*
                    }
                }
            }

            impl structured_sql::AsParams for #name {
                const PARAM_COUNT: usize = #param_count;
                fn as_params(&self) -> Vec<&dyn structured_sql::rusqlite::ToSql> {
                    use structured_sql::AsParams;
                    let mut result: Vec<&dyn structured_sql::rusqlite::ToSql> = vec![&structured_sql::rusqlite::types::Null; self.empty_columns_before()];
                    result[0] = self.variant_name();

                    match self {
                        #(Self::#variant_pattern => {
                            #(result.extend(#variant_field_names.as_params());)*
                        })*
                    }
                    while result.len() < Self::PARAM_COUNT {
                        result.push(&structured_sql::rusqlite::types::Null);
                    }
                    result
                }
            }

            impl<'a> structured_sql::IntoSqlTable<'a> for #name {
                type Filter = #filter_name;
                type Table = #table_name<'a>;
                const COLUMNS: &'static [structured_sql::SqlColumn] = &structured_sql::konst::slice::slice_concat!{structured_sql::SqlColumn ,&[
                    &[structured_sql::SqlColumn {
                        name: "variant",
                        r#type: structured_sql::SqlColumnType::Text,
                        is_primary: false,
                        is_unique: false,
                    }],
                    #(#columns,)*
                ]};

                const NAME: &'static str = stringify!(#table_name);
            }
        }
    }
}

impl ToTokens for Base {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let filter = self.create_filter();
        tokens.extend(filter);
        let table = self.create_table();
        tokens.extend(table);
        let conversions = self.create_conversions();
        tokens.extend(conversions);
    }
}

#[macro_export]
#[proc_macro_derive(IntoSqlTable, attributes(silo))]
pub fn derive_into_sql_table(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input).unwrap();

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
    quote! {#base}.into()
}
