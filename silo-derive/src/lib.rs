use heck::{ToSnakeCase, ToSnekCase};
use ident_case_conversions::CaseConversions;
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, Ident, LitInt, Type, Visibility};

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
struct AttributeStructData {
    on_conflict_rollback: bool,
    on_conflict_abort: bool,
    on_conflict_fail: bool,
    on_conflict_ignore: bool,
    on_conflict_replace: bool,
}

impl AttributeStructData {
    fn parse(attrs: &[Attribute]) -> AttributeStructData {
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
                    "rollback" => this.on_conflict_rollback = true,
                    "abort" => this.on_conflict_abort = true,
                    "fail" => this.on_conflict_fail = true,
                    "ignore" => this.on_conflict_ignore = true,
                    "replace" => this.on_conflict_replace = true,
                    _ => {
                        panic!("Invalid attribute");
                    }
                },
            }
        }

        this.validate();
        this
    }

    fn validate(&self) {
        let on_conflict = [
            self.on_conflict_abort,
            self.on_conflict_fail,
            self.on_conflict_ignore,
            self.on_conflict_replace,
            self.on_conflict_rollback,
        ];
        if on_conflict.iter().fold(0, |acc, cur| acc + *cur as usize) > 1 {
            panic!("Only one on conflict attribute can be active at once.");
        }
    }

    fn on_conflict(&self) -> proc_macro2::TokenStream {
        match [
            self.on_conflict_abort,
            self.on_conflict_fail,
            self.on_conflict_ignore,
            self.on_conflict_replace,
            self.on_conflict_rollback,
        ] {
            [false, false, false, false, false] | [true, ..] => {
                quote! {silo::SqlFailureBehavior::Abort}
            }
            [_, true, ..] => quote! {silo::SqlFailureBehavior::Fail},
            [_, _, true, ..] => quote! {silo::SqlFailureBehavior::Ignore},
            [_, _, _, true, ..] => quote! {silo::SqlFailureBehavior::Replace},
            [.., true] => quote! {silo::SqlFailureBehavior::Rollback},
        }
    }
}

#[derive(Debug, Default)]
struct AttributeFieldData {
    is_primary: bool,
    is_unique: bool,
    is_skip: bool,
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
                    "skip" => this.is_skip = true,
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
    is_skipped: bool,
    name_is_generated: bool,
    supports_vec: bool,
}

impl std::fmt::Debug for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Member")
            .field("variant", &self.variant)
            .field("name", &self.name)
            .field("is_primary", &self.is_primary)
            .field("is_unique", &self.is_unique)
            .field("is_optional", &self.is_optional)
            .field("is_skipped", &self.is_skipped)
            .field("name_is_generated", &self.name_is_generated)
            .finish()
    }
}
impl Member {
    fn from_struct_fields<const SUPPORTS_VEC: bool>(
        struct_name: syn::Ident,
        fields: syn::Fields,
    ) -> Vec<Member> {
        let mut field_index = 0;
        match fields {
            syn::Fields::Named(fields_named) => {
                Self::from_named_fields::<SUPPORTS_VEC>(struct_name, fields_named, false)
            }
            syn::Fields::Unnamed(fields_unnamed) => Self::from_unnamed_fields::<SUPPORTS_VEC>(
                &mut field_index,
                struct_name,
                fields_unnamed,
                false,
            ),
            syn::Fields::Unit => Vec::new(),
        }
    }

    fn from_named_fields<const SUPPORTS_VEC: bool>(
        variant: Ident,
        fields: syn::FieldsNamed,
        is_optional: bool,
    ) -> Vec<Member> {
        fields
            .named
            .iter()
            .enumerate()
            .map(|(i, f)| Member::from_field::<SUPPORTS_VEC>(i, variant.clone(), f, is_optional))
            .collect()
    }

    fn from_unnamed_fields<const SUPPORTS_VEC: bool>(
        base: &mut usize,
        variant: Ident,
        fields: syn::FieldsUnnamed,
        is_optional: bool,
    ) -> Vec<Member> {
        fields
            .unnamed
            .iter()
            .map(|f| {
                let result =
                    Member::from_field::<SUPPORTS_VEC>(*base, variant.clone(), f, is_optional);
                *base += 1;
                result
            })
            .collect()
    }

    fn from_field<const SUPPORTS_VEC: bool>(
        index: usize,
        variant: syn::Ident,
        f: &syn::Field,
        is_optional: bool,
    ) -> Member {
        let AttributeFieldData {
            is_primary,
            is_unique,
            is_skip,
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
            is_skipped: is_skip,
            name_is_generated,
            supports_vec: SUPPORTS_VEC,
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
                Self::from_named_fields::<false>(v.ident.clone(), fields_named, true)
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                Self::from_unnamed_fields::<false>(base, v.ident.clone(), fields_unnamed, true)
            }
            syn::Fields::Unit => Vec::new(),
        }
    }

    fn create_filter_field(&self) -> proc_macro2::TokenStream {
        let Member { name, type_, .. } = self;
        quote! { #name: <#type_ as silo::Filterable>::Filtered}
    }

    fn create_has_filter_field(&self) -> proc_macro2::TokenStream {
        let Member { name, type_, .. } = self;
        let type_ = Member::try_strip_auxiliary(self.supports_vec, type_);
        let name = format_ident!("has_{name}");
        quote! { #name(mut self, expected: #type_) -> Self}
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
        let snake_case_name = name.to_string().trim_start_matches("r#").to_snake_case();
        let snake_case_name = syn::LitStr::new(&snake_case_name, name.span());

        if let Some(t) = Member::as_simple_type(type_, *is_optional, self.supports_vec) {
            quote! { &[silo::SqlColumn {
                name: #snake_case_name,
                r#type: #t,
                is_unique: #is_unique,
                is_primary: #is_primary,
            }] }
        } else {
            let type_name =
                Member::type_to_name(Member::try_strip_auxiliary(self.supports_vec, type_));
            let column_macro_name = format_ident!("column_names_with_prefix_for_{type_name}");
            quote! { &#column_macro_name!(#snake_case_name) }
        }
    }

    fn create_column_definition_in_macro(&self) -> proc_macro2::TokenStream {
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
        let snake_case_name = name.to_string().trim_start_matches("r#").to_snake_case();
        let snake_case_name = syn::LitStr::new(&format!("_{snake_case_name}"), name.span());

        if let Some(t) = Member::as_simple_type(type_, *is_optional, self.supports_vec) {
            quote! { &[silo::SqlColumn {
                name: concat!($prefix, #snake_case_name),
                r#type: #t,
                is_unique: #is_unique,
                is_primary: #is_primary,
            }] }
        } else {
            let type_name =
                Member::type_to_name(Member::try_strip_auxiliary(self.supports_vec, type_));
            let column_macro_name = format_ident!("column_names_with_prefix_for_{type_name}");
            quote! { &#column_macro_name!(concat!($prefix, #snake_case_name)) }
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

    fn as_simple_type(
        type_: &Type,
        is_optional: bool,
        supports_vec: bool,
    ) -> Option<proc_macro2::TokenStream> {
        match type_ {
            Type::Path(type_path) => {
                Member::path_as_simple_type(&type_path.path, is_optional, supports_vec)
            }
            _ => None,
        }
    }

    fn path_as_simple_type(
        path: &syn::Path,
        is_optional: bool,
        supports_vec: bool,
    ) -> Option<proc_macro2::TokenStream> {
        for segment in &path.segments {
            if let Some(result) = Member::ident_as_simple_type(&segment.ident, is_optional) {
                return Some(result);
            }
            return match segment.ident.to_string().as_str() {
                "Option" => match &segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        match angle_bracketed_generic_arguments.args.iter().next()? {
                            syn::GenericArgument::Type(t) => {
                                Member::as_simple_type(t, true, supports_vec)
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                },
                "Vec" if supports_vec => match &segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        match angle_bracketed_generic_arguments.args.iter().next()? {
                            syn::GenericArgument::Type(t) => {
                                Member::as_simple_type(t, is_optional, supports_vec)
                            }
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
            "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
            | "isize" | "String" | "f32" | "f64" => {
                if is_optional {
                    Some(quote! {< Option<#ident> as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE})
                } else {
                    Some(quote! {< #ident as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE})
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

    fn try_strip_auxiliary(supports_vec: bool, type_: &Type) -> &Type {
        if supports_vec {
            Member::try_strip_vec_and_option(type_)
        } else {
            Member::try_strip_option(type_)
        }
    }

    fn try_strip_option(type_: &Type) -> &Type {
        match type_ {
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.iter().next() else {
                    return type_;
                };
                if segment.ident.to_string() != "Option" {
                    return type_;
                }
                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        let Some(syn::GenericArgument::Type(arg)) =
                            angle_bracketed_generic_arguments.args.iter().next()
                        else {
                            return type_;
                        };
                        arg
                    }
                    _ => type_,
                }
            }
            _ => type_,
        }
    }

    fn try_strip_vec(type_: &Type) -> &Type {
        match type_ {
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.iter().next() else {
                    return type_;
                };
                if segment.ident.to_string() != "Vec" {
                    return type_;
                }
                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        let Some(syn::GenericArgument::Type(arg)) =
                            angle_bracketed_generic_arguments.args.iter().next()
                        else {
                            return type_;
                        };
                        arg
                    }
                    _ => type_,
                }
            }
            _ => type_,
        }
    }

    fn try_strip_vec_and_option(type_: &Type) -> &Type {
        match type_ {
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.iter().next() else {
                    return type_;
                };
                let ident_str = segment.ident.to_string();
                if &ident_str != "Option" && ident_str != "Vec" {
                    return type_;
                }
                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(angle_bracketed_generic_arguments) => {
                        let Some(syn::GenericArgument::Type(arg)) =
                            angle_bracketed_generic_arguments.args.iter().next()
                        else {
                            return type_;
                        };
                        arg
                    }
                    _ => type_,
                }
            }
            _ => type_,
        }
    }

    fn type_to_name(type_: &Type) -> Ident {
        match type_ {
            Type::Path(type_path) => type_path.path.segments.last().unwrap().ident.clone(),
            _ => todo!(),
        }
    }

    fn create_single_field_type(&self) -> &Type {
        Member::try_strip_vec(&self.type_)
    }
}

struct Base {
    name: Ident,
    table_name: Ident,
    filter_name: Ident,
    visibility: Visibility,
    variants: Option<Vec<Ident>>,
    members: Vec<Member>,
    on_conflict: proc_macro2::TokenStream,
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
        let attribute_struct_data = AttributeStructData::parse(&attrs);
        let on_conflict = attribute_struct_data.on_conflict();
        let table_name = format_ident!("{name}Table");
        let filter_name = format_ident!("{name}Filter");
        let members = Member::from_struct_fields::<false>(name.clone(), data_struct.fields);
        // Add Partial types for Migration here!
        Self {
            name,
            table_name,
            filter_name,
            visibility,
            variants: None,
            members,
            on_conflict,
        }
    }

    fn from_enum(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_enum: syn::DataEnum,
    ) -> Base {
        let attribute_struct_data = AttributeStructData::parse(&attrs);
        let on_conflict = attribute_struct_data.on_conflict();
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
            on_conflict,
        }
    }

    fn create_table(&self) -> proc_macro2::TokenStream {
        let Base {
            name,
            table_name,
            filter_name,
            visibility,
            on_conflict,
            ..
        } = self;
        quote! {
        #visibility struct #table_name<'a> {
            connection: &'a silo::rusqlite::Connection,
            string_storage: std::sync::Arc<std::sync::Mutex<silo::StaticStringStorage>>,
        }


        impl<'a> silo::SqlTable<'a> for #table_name<'a> {
            type RowType = #name;

            const INSERT_FAILURE_BEHAVIOR: silo::SqlFailureBehavior = #on_conflict;


            fn insert(&self, row: Self::RowType) -> Result<(), silo::rusqlite::Error> {
                use silo::AsParams;
                let columns = Self::RowType::COLUMNS.into_iter().map(|c| c.name).fold(
                    String::new(),
                    |mut acc, cur| {
                        if acc.is_empty() {
                            format!("\"{cur}\"")
                        } else {
                            acc.push_str(", ");
                            acc.push('"');
                            acc.push_str(cur);
                            acc.push('"');
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
                        "INSERT OR {} INTO {} ({columns}) VALUES ({values})",
                        Self::INSERT_FAILURE_BEHAVIOR.to_string(),
                        Self::RowType::NAME,
                    );
                self.connection.execute(
                    &sql,
                    row.as_params().as_slice(),
                )?;
                Ok(())
            }

            fn filter(&self, filter: #filter_name) -> Result<Vec<#name>, silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
                silo::query_table_filtered::<Self::RowType>(&self.connection, &mut self.string_storage.lock().unwrap(), generic)
            }

            fn delete(&self, filter: #filter_name) -> Result<usize, silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
                silo::delete_table_filtered::<Self::RowType>(&self.connection, generic)
            }

            fn from_connection(connection: &'a silo::rusqlite::Connection, string_storage: std::sync::Arc<std::sync::Mutex<silo::StaticStringStorage>>) -> Self {
                Self { connection, string_storage }
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

        let filter_field_names: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_field_name())
            .collect();

        let filter_fields: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_filter_field())
            .collect();

        let has_filter_fields: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_has_filter_field())
            .collect();

        quote! {
            #[derive(Default, Clone, Debug)]
            #visibility struct #filter_name {
                #(#filter_fields,)*
            }

            impl #filter_name {
                #(
                    #visibility fn #has_filter_fields {
                        use silo::Filterable;
                        self.#filter_field_names = expected.must_be_equal();
                        self
                    }
                )*
            }

            impl silo::Filterable for #name {
                type Filtered = #filter_name;

                fn must_be_equal(self) -> Self::Filtered {
                    let mut result = #filter_name::default();
                    #(result.#filter_field_names = self.#filter_field_names.must_be_equal();)*
                    result
                }
            }

            impl silo::IntoGenericFilter for #filter_name {
                fn into_generic(self, string_storage: &mut silo::StaticStringStorage, column_name: Option<&'static str>) -> silo::GenericFilter {
                    let mut columns = std::collections::HashMap::new();
                    #(
                        let actual_column_name = column_name.map(|c|
                            string_storage.store(&[c, "_", stringify!(#filter_field_names)])).unwrap_or(stringify!(#filter_field_names));
                        silo::GenericFilter::insert_into_columns(actual_column_name, &mut columns, self.#filter_field_names, string_storage);
                    )*
                    silo::GenericFilter { columns }
                }
            }

            impl silo::IntoSqlColumnFilter for #filter_name {
                fn into_sql_column_filter(
                    self,
                    name: &'static str,
                    string_storage: &mut silo::StaticStringStorage,
                ) -> Vec<(&'static str, silo::SqlColumnFilter<silo::SqlValue>)> {
                    use silo::IntoSqlColumnFilter;
                    let mut result = Vec::new();
                    #(
                        let name = string_storage.store(&[name, "_", stringify!(#filter_field_names)]);
                        result.extend(self.#filter_field_names.into_sql_column_filter(name, string_storage));
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
                variant: silo::SqlColumnFilter<String>,
            }

            impl silo::Filterable for #name {
                type Filtered = #filter_name;

                fn must_be_equal(self) -> Self::Filtered {
                    let mut result = #filter_name::default();
                    result.variant = self.variant_name().to_string().must_be_equal();
                    result
                }
            }

            impl silo::IntoGenericFilter for #filter_name {
                fn into_generic(self, string_storage: &mut silo::StaticStringStorage, column_name: Option<&'static str>) -> silo::GenericFilter {
                    let mut columns = std::collections::HashMap::new();
                    // TODO: Concat with column name!
                    let actual_column_name = column_name.map(|c| string_storage.store(
                            &[c, "_", "variant"]
                        )).unwrap_or("variant");

                    silo::GenericFilter::insert_into_columns(actual_column_name, &mut columns, self.variant, string_storage);
                    silo::GenericFilter { columns }
                }
            }

            impl silo::IntoSqlColumnFilter for #filter_name {
                fn into_sql_column_filter(
                    self,
                    name: &'static str,
                    string_storage: &mut silo::StaticStringStorage,
                ) -> Vec<(&'static str, silo::SqlColumnFilter<silo::SqlValue>)> {
                    use silo::IntoSqlColumnFilter;
                    let mut result = Vec::new();
                    let name = string_storage.store(&[name, "_", "variant"]);
                    result.extend(self.variant.into_sql_column_filter(name, string_storage));
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
        let field_names_with_skips: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|c| c.create_field_name())
            .collect();
        let skipped_field_names: Vec<_> = members
            .iter()
            .filter(|m| m.is_skipped)
            .map(|c| c.create_field_name())
            .collect();
        let field_types_with_skips: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|c| c.create_field_type())
            .collect();
        let param_count = field_names_with_skips.len();
        let field_names_without_skips: Vec<_> =
            members.iter().map(|c| c.create_field_name()).collect();
        let columns: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition())
            .collect();
        let columns_in_macro: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition_in_macro())
            .collect();
        let create_prefixed_columns_macro = format_ident!("column_names_with_prefix_for_{name}");
        quote! {
            impl silo::FromRow for #name {
                fn try_from_row(string_storage: &mut silo::StaticStringStorage, row_name: Option<&'static str>, row: &silo::rusqlite::Row) -> Option<Self> {
                    use silo::rusqlite::OptionalExtension;
                    #(
                        let actual_column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#field_names_with_skips)])).unwrap_or(stringify!(#field_names_with_skips));
                        let #field_names_with_skips = <#field_types_with_skips>::try_from_row(string_storage, Some(actual_column_name), row)?;)*
                    #(let #skipped_field_names = Default::default();)*
                    Some(Self {#( #field_names_without_skips),*})
                }
            }

            impl silo::AsParams for #name {
                const PARAM_COUNT: usize = #(<#field_types_with_skips as silo::AsParams>::PARAM_COUNT +)* 0;
                fn as_params(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
                    use silo::AsParams;
                    let mut result = Vec::new();
                    #(result.extend(&self.#field_names_with_skips.as_params()));*
                    ;
                    result
                }
            }

            impl<'a> silo::IntoSqlTable<'a> for #name {
                type Filter = #filter_name;
                type Table = #table_name<'a>;
                const COLUMNS: &'static [silo::SqlColumn] = &silo::konst::slice::slice_concat!{silo::SqlColumn ,&[
                    #(#columns,)*
                ]};

                const NAME: &'static str = stringify!(#table_name);
            }

            #[allow(unused_macros)]
            macro_rules! #create_prefixed_columns_macro {
                ($prefix:expr) => {
                    silo::konst::slice::slice_concat!{silo::SqlColumn ,&[
                    #(#columns_in_macro,)*
                ]}
            };
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
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition())
            .collect();
        let variant_pattern = Member::create_variant_pattern(variants, &members);
        let variant_empty_columns_before =
            Member::create_variant_empty_columns_before(variants, &members);
        let variant_names = Member::create_variant_names(variants, &members);
        let variant_field_names = Member::create_variant_field_names(variants, &members);

        let columns_in_macro: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition_in_macro())
            .collect();
        let create_prefixed_columns_macro = format_ident!("column_names_with_prefix_for_{name}");

        quote! {
            impl silo::FromRow for #name {
                fn try_from_row(string_storage: &mut silo::StaticStringStorage, row_name: Option<&'static str>, row: &silo::rusqlite::Row) -> Option<Self> {
                    use silo::rusqlite::OptionalExtension;
                    let variant_name = row_name.map(|r| string_storage.store(&[r, "_variant"])).unwrap_or("variant");
                    let variant = String::try_from_row(string_storage, Some(variant_name), row)?;
                    #(
                        let column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#field_names_with_skips)])).unwrap_or(stringify!(#field_names_with_skips));
                        let #field_names_with_skips = <#field_types_with_skips>::try_from_row(string_storage, Some(column_name), row);)*
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

            impl silo::AsParams for #name {
                const PARAM_COUNT: usize = #(<#field_types_with_skips as silo::AsParams>::PARAM_COUNT +)* 1;
                fn as_params(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
                    use silo::AsParams;
                    let mut result: Vec<&dyn silo::rusqlite::ToSql> = vec![&silo::rusqlite::types::Null; self.empty_columns_before()];
                    result[0] = self.variant_name();

                    match self {
                        #(Self::#variant_pattern => {
                            #(result.extend(#variant_field_names.as_params());)*
                        })*
                    }
                    while result.len() < Self::PARAM_COUNT {
                        result.push(&silo::rusqlite::types::Null);
                    }
                    result
                }
            }

            impl<'a> silo::IntoSqlTable<'a> for #name {
                type Filter = #filter_name;
                type Table = #table_name<'a>;
                const COLUMNS: &'static [silo::SqlColumn] = &silo::konst::slice::slice_concat!{silo::SqlColumn ,&[
                    &[silo::SqlColumn {
                        name: "variant",
                        r#type: silo::SqlColumnType::OptionalText,
                        is_primary: false,
                        is_unique: false,
                    }],
                    #(#columns,)*
                ]};

                const NAME: &'static str = stringify!(#table_name);
            }

            #[allow(unused_macros)]
            macro_rules! #create_prefixed_columns_macro {
                ($prefix:expr) => {
                    silo::konst::slice::slice_concat!{silo::SqlColumn ,&[
                        &[silo::SqlColumn {
                        name: concat!($prefix, "_variant"),
                        r#type: silo::SqlColumnType::Text,
                        is_primary: false,
                        is_unique: false,
                    }],
                    #(#columns_in_macro,)*
                ]}


            };
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
    quote! {#base}.into()
}

struct BaseVec {
    name: Ident,
    table_name: Ident,
    filter_name: Ident,
    single_row_name: Ident,
    visibility: Visibility,
    members: Vec<Member>,
    on_conflict: proc_macro2::TokenStream,
}

impl std::fmt::Debug for BaseVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BaseVec")
            .field("name", &self.name)
            .field("table_name", &self.table_name)
            .field("filter_name", &self.filter_name)
            .field("single_row_name", &self.single_row_name)
            .field("members", &self.members)
            .finish()
    }
}
impl BaseVec {
    fn from_struct(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_struct: syn::DataStruct,
    ) -> Self {
        let attribute_struct_data = AttributeStructData::parse(&attrs);
        let on_conflict = attribute_struct_data.on_conflict();
        let table_name = format_ident!("{name}Table");
        let filter_name = format_ident!("{name}Filter");
        let single_row_name = format_ident!("{name}SingleRow");
        let members = Member::from_struct_fields::<true>(name.clone(), data_struct.fields);
        if !members.iter().any(|m| m.is_primary) {
            panic!("Vector Tables need to have a primary key! Use attribute #[silo(primary)]");
        }
        // Add Partial types for Migration here!
        Self {
            name,
            table_name,
            filter_name,
            single_row_name,
            visibility,
            members,
            on_conflict,
        }
    }

    fn create_table(&self) -> proc_macro2::TokenStream {
        let BaseVec {
            name,
            table_name,
            filter_name,
            members,
            visibility,
            on_conflict,
            ..
        } = self;
        quote! {
        #visibility struct #table_name<'a> {
            connection: &'a silo::rusqlite::Connection,
            string_storage: std::sync::Arc<std::sync::Mutex<silo::StaticStringStorage>>,
        }

        impl<'a> silo::SqlVecTable<'a> for #table_name<'a> {
            const INSERT_FAILURE_BEHAVIOR: silo::SqlFailureBehavior = #on_conflict;

            type GroupedRowType = #name;

            fn insert(&self, row: Self::GroupedRowType) -> Result<(), silo::rusqlite::Error> {
                use silo::AsRepeatedParams;
                let columns = Self::GroupedRowType::COLUMNS.into_iter().map(|c| c.name).fold(
                    String::new(),
                    |mut acc, cur| {
                        if acc.is_empty() {
                            format!("\"{cur}\"")
                        } else {
                            acc.push_str(", ");
                            acc.push('"');
                            acc.push_str(cur);
                            acc.push('"');
                            acc
                        }
                    },
                );
                let values = (0..Self::GroupedRowType::COLUMNS.len()).map(|v| v + 1).fold(
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
                        "INSERT OR {} INTO {} ({columns}) VALUES ({values})",
                        Self::INSERT_FAILURE_BEHAVIOR.to_string(),
                        Self::GroupedRowType::NAME,
                    );
                for entry in &row.as_params() {
                    self.connection.execute(
                        &sql,
                        entry.as_slice(),
                    )?;
                }
                Ok(())
            }

            fn filter(&self, filter: #filter_name) -> Result<Vec<#name>, silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
                silo::query_vec_table_filtered::<Self::GroupedRowType>(&self.connection, &mut self.string_storage.lock().unwrap(), generic)
            }

            fn delete(&self, filter: #filter_name) -> Result<usize, silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
                silo::delete_vec_table_filtered::<Self::GroupedRowType>(&self.connection, generic)
            }

            fn from_connection(connection: &'a silo::rusqlite::Connection, string_storage: std::sync::Arc<std::sync::Mutex<silo::StaticStringStorage>>) -> Self {
                Self { connection, string_storage }
            }
        }
        }
    }

    fn create_filter(&self) -> proc_macro2::TokenStream {
        self.create_filter_struct()
    }

    fn create_filter_struct(&self) -> proc_macro2::TokenStream {
        let BaseVec {
            name,
            filter_name,
            visibility,
            members,
            ..
        } = self;

        let filter_field_names: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_field_name())
            .collect();

        let filter_fields: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_filter_field())
            .collect();

        let has_filter_fields: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_has_filter_field())
            .collect();

        quote! {
            #[derive(Default, Clone, Debug)]
            #visibility struct #filter_name {
                #(#filter_fields,)*
            }

            impl #filter_name {
                #(
                    #visibility fn #has_filter_fields {
                        use silo::Filterable;
                        self.#filter_field_names = expected.must_be_equal();
                        self
                    }
                )*
            }

            impl silo::Filterable for #name {
                type Filtered = #filter_name;

                fn must_be_equal(self) -> Self::Filtered {
                    let mut result = #filter_name::default();
                    #(result.#filter_field_names = self.#filter_field_names.must_be_equal();)*
                    result
                }
            }

            impl silo::IntoGenericFilter for #filter_name {
                fn into_generic(self, string_storage: &mut silo::StaticStringStorage, column_name: Option<&'static str>) -> silo::GenericFilter {
                    let mut columns = std::collections::HashMap::new();
                    #(
                        let actual_column_name = column_name.map(|c|
                            string_storage.store(&[c, "_", stringify!(#filter_field_names)])).unwrap_or(stringify!(#filter_field_names));
                        silo::GenericFilter::insert_into_columns(actual_column_name, &mut columns, self.#filter_field_names, string_storage);
                    )*
                    silo::GenericFilter { columns }
                }
            }

            impl silo::IntoSqlColumnFilter for #filter_name {
                fn into_sql_column_filter(
                    self,
                    name: &'static str,
                    string_storage: &mut silo::StaticStringStorage,
                ) -> Vec<(&'static str, silo::SqlColumnFilter<silo::SqlValue>)> {
                    use silo::IntoSqlColumnFilter;
                    let mut result = Vec::new();
                    #(
                        let name = string_storage.store(&[name, "_", stringify!(#filter_field_names)]);
                        result.extend(self.#filter_field_names.into_sql_column_filter(name, string_storage));
                    )*
                    result
                }
            }
        }
    }

    fn create_conversions(&self) -> proc_macro2::TokenStream {
        self.create_conversions_struct()
    }

    fn create_conversions_struct(&self) -> proc_macro2::TokenStream {
        let BaseVec {
            name,
            table_name,
            filter_name,
            members,
            single_row_name,
            visibility,
            ..
        } = self;
        let field_names_with_skips: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|c| c.create_field_name())
            .collect();
        let skipped_field_names: Vec<_> = members
            .iter()
            .filter(|m| m.is_skipped)
            .map(|c| c.create_field_name())
            .collect();
        let field_types_with_skips: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|c| c.create_field_type())
            .collect();
        let field_names_without_skips: Vec<_> =
            members.iter().map(|c| c.create_field_name()).collect();
        let columns: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition())
            .collect();
        let columns_in_macro: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition_in_macro())
            .collect();
        let create_prefixed_columns_macro = format_ident!("column_names_with_prefix_for_{name}");
        let primary = members
            .iter()
            .find(|m| m.is_primary)
            .expect("Needs to have attribute #[silo(primary)]");
        let primary_name = &primary.name;
        let primary = {
            let name = &primary.name;
            quote! {
                let #name = rows.get(0)?.#name;
            }
        };
        let remaining_names: Vec<_> = members
            .iter()
            .filter(|m| !m.is_primary && !m.is_skipped)
            .map(|m| &m.name)
            .collect();
        let remaining_types: Vec<_> = members
            .iter()
            .filter(|m| !m.is_primary && !m.is_skipped)
            .map(|m| &m.type_)
            .collect();
        let single_field_names: Vec<_> = members.iter().map(|m| m.create_field_name()).collect();
        let single_field_types: Vec<_> = members
            .iter()
            .map(|m| m.create_single_field_type())
            .collect();

        quote! {
            #visibility struct #single_row_name {
                #(#single_field_names: #single_field_types,)*
            }

            impl silo::FromRow for #single_row_name {
                fn try_from_row(string_storage: &mut silo::StaticStringStorage, row_name: Option<&'static str>, row: &silo::rusqlite::Row) -> Option<Self> {
                    #(
                        let actual_column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#single_field_names)])).unwrap_or(stringify!(#single_field_names));
                        let #single_field_names = <#single_field_types>::try_from_row(string_storage, Some(actual_column_name), row)?;)*
                    Some(Self {
                        #(#single_field_names,)*
                    })
                }
            }

            impl silo::FromRowWithPrimary for #single_row_name {
                fn primary(&self) -> usize {
                    self.#primary_name
                }
            }

            impl silo::FromGroupedRows for #name {
                type RowType = #single_row_name;
                fn try_from_rows(string_storage: &mut silo::StaticStringStorage, row_name: Option<&'static str>, rows: Vec<#single_row_name>) -> Option<Self> {
                    use silo::rusqlite::OptionalExtension;
                    #primary
                    #(
                        let mut #remaining_names = <#remaining_types>::default();
                    )*
                    for row in rows {
                        #(
                            #remaining_names.push(row.#remaining_names);
                        )*
                    }
                    #(let #skipped_field_names = Default::default();)*
                    Some(Self {#( #field_names_without_skips),*})
                }
            }

            impl silo::AsRepeatedParams for #name {
                const PARAM_COUNT: usize = #(<#field_types_with_skips as silo::AsRepeatedParams>::PARAM_COUNT +)* 0;
                fn as_params(&self) -> Vec<Vec<&dyn silo::rusqlite::ToSql>> {
                    use silo::AsRepeatedParams;
                    let mut result = Vec::new();
                    #(
                        let values = <#field_types_with_skips as silo::AsRepeatedParams>::as_params(&self.#field_names_with_skips);
                        // dbg!(&self.#field_names_with_skips, values.len());
                        result.extend(values));*
                    ;
                    result
                }
            }

            impl<'a> silo::IntoSqlVecTable<'a> for #name {
                type Filter = #filter_name;
                type Table = #table_name<'a>;
                const COLUMNS: &'static [silo::SqlColumn] = &silo::konst::slice::slice_concat!{silo::SqlColumn ,&[
                    #(#columns,)*
                ]};

                const NAME: &'static str = stringify!(#table_name);
            }

            #[allow(unused_macros)]
            macro_rules! #create_prefixed_columns_macro {
                ($prefix:expr) => {
                    silo::konst::slice::slice_concat!{silo::SqlColumn ,&[
                    #(#columns_in_macro,)*
                ]}
            };
            }
        }
    }
}

impl ToTokens for BaseVec {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let filter = self.create_filter();
        tokens.extend(filter);
        let table = self.create_table();
        tokens.extend(table);
        let conversions = self.create_conversions();
        tokens.extend(conversions);
    }
}

// #[macro_export]
#[proc_macro_derive(IntoSqlVecTable, attributes(silo))]
pub fn derive_into_sql_vec_table(input: TokenStream) -> TokenStream {
    // syn::Data
    let input: syn::DeriveInput = syn::parse(input)
        .expect("This is a derive macro and should be used with structs or enums.");

    let base = match input.data {
        syn::Data::Struct(data_struct) => {
            BaseVec::from_struct(input.attrs, input.ident, input.vis, data_struct)
        }
        syn::Data::Enum(data_enum) => {
            panic!(
                "The right behavior of an enum is not obvious here. Either use IntoSqlTable or derive this on a struct instead."
            )
        }
        syn::Data::Union(_) => {
            panic!("Unions need a clear representation, either use a struct or an enum.")
        }
    };
    quote! {#base}.into()
}
