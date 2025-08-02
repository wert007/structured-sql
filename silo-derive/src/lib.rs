use heck::{ToSnakeCase, ToSnekCase};
use ident_case_conversions::CaseConversions;
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Attribute, Error, Ident, LitInt, Type, TypePath, Visibility};

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
    has_custom_migration_handler: bool,
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
                    "migrate" => this.has_custom_migration_handler = true,
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
        let type_ = Member::try_strip_vec(type_);
        quote! { #name: <#type_ as silo::Filterable>::Filtered}
    }

    fn create_has_filter_field(&self) -> proc_macro2::TokenStream {
        let Member { name, type_, .. } = self;
        let name = format_ident!("has_{name}");
        let type_ = Member::try_strip_vec(type_);
        if Member::as_simple_type(type_, false).is_some() {
            let type_ = Member::try_strip_auxiliary(type_);
            quote! { #name(mut self, expected: #type_) -> Self}
        } else {
            quote! { #name(mut self, expected: <#type_ as silo::Filterable>::Filtered) -> Self}
        }
    }

    fn create_contains_filter_field(&self) -> proc_macro2::TokenStream {
        let Member { name, type_, .. } = self;
        let type_ = Member::try_strip_vec(type_);
        let name = format_ident!("{name}_contains");

        if Member::as_simple_type(type_, false).is_some() {
            let type_ = Member::try_strip_auxiliary(type_);
            quote! { #name(mut self, expected: #type_) -> Self}
        } else {
            quote! { #name(mut self, expected: <#type_ as silo::Filterable>::Filtered) -> Self}
        }
    }

    fn create_field_name(&self) -> proc_macro2::TokenStream {
        let Member { name, .. } = self;
        quote! { #name }
    }

    fn create_column_definition(&self, has_vec_as_members: bool) -> proc_macro2::TokenStream {
        let Member {
            name,
            type_,
            is_primary,
            is_unique,
            is_optional,
            ..
        } = self;
        let is_unique = syn::LitBool::new(!has_vec_as_members && *is_unique, name.span());
        let is_primary = syn::LitBool::new(!has_vec_as_members && *is_primary, name.span());
        let snake_case_name = name.to_string().trim_start_matches("r#").to_snake_case();
        let snake_case_name = syn::LitStr::new(&snake_case_name, name.span());
        let type_ = Member::try_strip_vec(type_);

        if let Some(t) = Member::as_simple_type(type_, *is_optional) {
            quote! { &[silo::SqlColumn {
                name: #snake_case_name,
                r#type: #t,
                is_unique: #is_unique,
                is_primary: #is_primary,
            }] }
        } else {
            let type_name = Member::type_to_name(Member::try_strip_auxiliary(type_));
            let column_macro_name = format_ident!("column_names_with_prefix_for_{type_name}");
            quote! { &#column_macro_name!(#snake_case_name) }
        }
    }

    fn create_column_definition_in_macro(
        &self,
        has_vec_as_members: bool,
    ) -> proc_macro2::TokenStream {
        let Member {
            name,
            type_,
            is_primary,
            is_unique,
            is_optional,
            ..
        } = self;
        let type_ = Member::try_strip_vec(type_);

        let is_unique = syn::LitBool::new(!has_vec_as_members && *is_unique, name.span());
        let is_primary = syn::LitBool::new(!has_vec_as_members && *is_primary, name.span());
        let snake_case_name = name.to_string().trim_start_matches("r#").to_snake_case();
        let snake_case_name = syn::LitStr::new(&format!("_{snake_case_name}"), name.span());

        if let Some(t) = Member::as_simple_type(type_, *is_optional) {
            quote! { &[silo::SqlColumn {
                name: concat!($prefix, #snake_case_name),
                r#type: #t,
                is_unique: #is_unique,
                is_primary: #is_primary,
            }] }
        } else {
            let type_name = Member::type_to_name(Member::try_strip_auxiliary(type_));
            let column_macro_name = format_ident!("column_names_with_prefix_for_{type_name}");
            quote! { &#column_macro_name!(concat!($prefix, #snake_case_name)) }
        }
    }

    fn create_field_type(&self) -> proc_macro2::TokenStream {
        let Member { type_, .. } = self;
        let type_ = Member::try_strip_vec(type_);

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
        let mut seen_path = Vec::with_capacity(path.segments.len());
        for segment in &path.segments {
            if let Some(result) =
                Member::ident_as_simple_type(&segment.ident, is_optional, &seen_path)
            {
                return Some(result);
            }
            if segment.ident.to_string() == "time" {
                seen_path.push(segment);
                continue;
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

    fn ident_as_simple_type(
        ident: &Ident,
        is_optional: bool,
        seen_path: &[&syn::PathSegment],
    ) -> Option<proc_macro2::TokenStream> {
        match ident.to_string().as_str() {
            "bool" | "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
            | "isize" | "String" | "f32" | "f64" | "Time" | "Date" | "OffsetDateTime" => {
                if is_optional {
                    Some(
                        quote! {< Option<#(#seen_path::)*#ident> as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE},
                    )
                } else {
                    Some(
                        quote! {< #(#seen_path::)*#ident as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE},
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

    fn try_strip_auxiliary(type_: &Type) -> &Type {
        Member::try_strip_option(type_)
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
                let Some(segment) = type_path.path.segments.iter().last() else {
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

    fn create_partial_field_definition(&self) -> proc_macro2::TokenStream {
        let Member { name, type_, .. } = self;
        let type_ = Member::try_strip_vec(type_);
        quote! { #name: <#type_ as silo::HasPartialRepresentation>::Partial}
    }

    fn has_vec(&self) -> bool {
        type_is_vec(&self.type_)
    }
}

fn type_is_vec(type_: &syn::Type) -> bool {
    match type_ {
        Type::Path(type_path) => {
            let Some(segment) = type_path.path.segments.iter().next() else {
                return false;
            };
            if segment.ident.to_string() != "Vec" {
                return false;
            } else {
                return true;
            }
        }
        _ => false,
    }
}

fn type_is_option(type_: &syn::Type) -> bool {
    match type_ {
        Type::Path(type_path) => {
            let Some(segment) = type_path.path.segments.iter().next() else {
                return false;
            };
            if segment.ident.to_string() != "Option" {
                return false;
            } else {
                return true;
            }
        }
        _ => false,
    }
}

struct Base {
    name: Ident,
    row_type_name: Ident,
    table_name: Ident,
    filter_name: Ident,
    partial_name: Ident,
    visibility: Visibility,
    variants: Option<Vec<Ident>>,
    members: Vec<Member>,
    on_conflict: proc_macro2::TokenStream,
    migration_handler: proc_macro2::TokenStream,
    has_vec_as_member: bool,
    errors: proc_macro2::TokenStream,
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
        let members = Member::from_struct_fields(name.clone(), data_struct.fields);

        let has_vec_as_member = members.iter().any(|m| !m.is_skipped && m.has_vec());
        let row_type_name = if has_vec_as_member {
            format_ident!("{name}RowType")
        } else {
            name.clone()
        };
        let table_name = format_ident!("{name}Table");
        let filter_name = format_ident!("{name}Filter");
        let partial_name = format_ident!("Partial{name}");
        let migration_handler = if attribute_struct_data.has_custom_migration_handler {
            proc_macro2::TokenStream::new()
        } else {
            quote! { impl silo::MigrationHandler for #row_type_name {}
            }
        };
        // Add Partial types for Migration here!
        Self {
            name,
            row_type_name,
            table_name,
            filter_name,
            partial_name,
            visibility,
            variants: None,
            members,
            on_conflict,
            migration_handler,
            has_vec_as_member,
            errors: proc_macro2::TokenStream::new(),
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
        let partial_name = format_ident!("Partial{name}");
        let members = Member::from_enum_variants(&data_enum.variants);
        let variants = data_enum.variants.iter().map(|v| v.ident.clone()).collect();
        // Add Partial types for Migration here!
        let migration_handler = if attribute_struct_data.has_custom_migration_handler {
            proc_macro2::TokenStream::new()
        } else {
            quote! { impl silo::MigrationHandler for #name {}
            }
        };
        let mut errors = proc_macro2::TokenStream::new();
        if members.iter().any(|m| !m.is_skipped && m.has_vec()) {
            // compile_error!("Only structs can have vectors not enums for now!")
            errors.extend(
                Error::new(name.span(), "Cannot have a vec inside an enum!").into_compile_error(),
            );
        }
        Self {
            row_type_name: name.clone(),
            name,
            table_name,
            filter_name,
            partial_name,
            visibility,
            variants: Some(variants),
            members,
            on_conflict,
            migration_handler,
            has_vec_as_member: false,
            errors,
        }
    }

    fn create_table(&self) -> proc_macro2::TokenStream {
        let Base {
            name,
            row_type_name,
            table_name,
            filter_name,
            partial_name,
            visibility,
            on_conflict,
            members,
            ..
        } = self;
        let iterable_remaining_elements = members
            .iter()
            .filter(|m| !m.is_skipped && !m.is_primary && m.has_vec())
            .map(|m| format_ident!("{}_silo_remaining_elements", m.name));
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
            type RowType = #row_type_name;
            type ValueType = #name;

            const INSERT_FAILURE_BEHAVIOR: silo::SqlFailureBehavior = #on_conflict;


            fn insert(&self, row: impl silo::ToRows<Self::RowType>) -> Result<(), silo::rusqlite::Error> {
                use silo::{AsParams, RowType};
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

                let mut stmt = self.connection.prepare(&sql)?;
                for row in row.to_rows() {
                    row.clone().insert_into_connected_foreign_tables(true, self.connection)?;
                    stmt.execute(row.as_params().as_slice())?;
                }
                Ok(())
            }

            fn filter(&self, filter: #filter_name) -> Result<Vec<#name>, silo::rusqlite::Error> {
                use silo::IntoGenericFilter;
                let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
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

    fn create_filter(&self) -> proc_macro2::TokenStream {
        if let Some(variants) = &self.variants {
            self.create_filter_enum(variants)
        } else {
            self.create_filter_struct()
        }
    }

    fn create_filter_struct(&self) -> proc_macro2::TokenStream {
        let Base {
            row_type_name,
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
        let contains_filter_fields: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_contains_filter_field())
            .collect();

        let must_be_equal_impl = if let Some(primary) = members.iter().find(|m| m.is_primary) {
            let name = &primary.name;
            quote! {
                let mut result = #filter_name::default();
                result.#name = self.#name.must_be_equal();
                result
            }
        } else {
            quote! {
                let mut result = #filter_name::default();
                #(result.#filter_field_names = self.#filter_field_names.must_be_equal();)*
                result
            }
        };

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

                    #visibility fn #contains_filter_fields {
                        use silo::Filterable;
                        self.#filter_field_names = expected.must_contain();
                        self
                    }
                )*
            }

            impl silo::Filterable for #row_type_name {
                type Filtered = #filter_name;

                fn must_be_equal(&self) -> Self::Filtered {
                    use silo::AsParams;
                    let mut string_storage = silo::StaticStringStorage::new();
                    #must_be_equal_impl
                }
                fn must_contain(&self) -> Self::Filtered {
                    let mut result = #filter_name::default();
                    #(result.#filter_field_names = self.#filter_field_names.must_contain();)*
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
                        let column_name = string_storage.store(&[name, "_", stringify!(#filter_field_names)]);
                        result.extend(self.#filter_field_names.into_sql_column_filter(column_name, string_storage));
                    )*
                    result
                }
            }
        }
    }

    fn create_filter_enum(&self, _variants: &[syn::Ident]) -> proc_macro2::TokenStream {
        let Base {
            row_type_name,
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

            impl silo::Filterable for #row_type_name {
                type Filtered = #filter_name;

                fn must_be_equal(&self) -> Self::Filtered {
                    let mut result = #filter_name::default();
                    result.variant = self.variant_name().to_string().must_be_equal();
                    result
                }

                fn must_contain(&self) -> Self::Filtered {
                    let mut result = #filter_name::default();
                    result.variant = self.variant_name().to_string().must_contain();
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
                    let column_name = string_storage.store(&[name, "_", "variant"]);
                    result.extend(self.variant.into_sql_column_filter(column_name, string_storage));
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
            row_type_name,
            table_name,
            partial_name,
            members,
            visibility,
            migration_handler,
            has_vec_as_member,
            filter_name,
            ..
        } = self;
        let mut field_names_with_skips: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|c| c.create_field_name())
            .collect();
        let skipped_field_names: Vec<_> = members
            .iter()
            .filter(|m| m.is_skipped)
            .map(|c| c.create_field_name())
            .collect();
        let field_names_without_skips: Vec<_> =
            members.iter().map(|c| c.create_field_name()).collect();
        let mut columns: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition(*has_vec_as_member))
            .collect();
        let mut columns_in_macro: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition_in_macro(*has_vec_as_member))
            .collect();
        let create_prefixed_columns_macro =
            format_ident!("column_names_with_prefix_for_{row_type_name}");
        let partial_field_definitions: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_partial_field_definition())
            .collect();

        let mut field_names = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.name.clone())
            .collect::<Vec<_>>();
        let mut field_types = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| {
                if *has_vec_as_member && !m.is_primary {
                    let t = Member::try_strip_vec(&m.type_);
                    syn::parse_quote!(<#t as silo::HasPartialRepresentation>::Partial)
                } else {
                    Member::try_strip_vec(&m.type_).clone()
                }
            })
            .collect::<Vec<_>>();

        let mut added_fields = Vec::new();
        let mut removed_fields_from_special_handling = Vec::new();
        let row_type_definition = if *has_vec_as_member {
            create_row_type(
                name,
                row_type_name,
                table_name,
                partial_name,
                members,
                visibility,
                filter_name,
                &mut field_names_with_skips,
                field_names_without_skips,
                &mut columns,
                &mut columns_in_macro,
                partial_field_definitions,
                &mut field_names,
                &mut field_types,
                &mut added_fields,
                &mut removed_fields_from_special_handling,
            )
        } else {
            let partial_row_type_name = format_ident!("Partial{row_type_name}");
            quote!(
                impl silo::ToRows<#partial_row_type_name> for #partial_name {
                    fn to_rows(self) -> Vec<#partial_row_type_name> {
                        vec![self]
                    }
                }

                impl silo::RowType for #row_type_name {
                    fn insert_into_connected_foreign_tables(
                        self,
                        is_top_level: bool,
                        connection: &silo::rusqlite::Connection,
                    ) -> silo::rusqlite::Result<()> {
                        #(
                            self.#field_names_with_skips.insert_into_connected_foreign_tables(false, connection)?;
                        )*
                        Ok(())
                    }
                }
            )
        };

        let partial = create_partial::<true>(
            row_type_name,
            &field_names,
            &field_types,
            &members
                .iter()
                .filter(|m| m.is_skipped)
                .map(|m| m.name.clone())
                .collect::<Vec<_>>(),
            &members
                .iter()
                .filter(|m| {
                    !m.is_skipped
                        && !type_is_option(&m.type_)
                        && !removed_fields_from_special_handling.contains(&m.name)
                })
                .map(|m| m.name.clone())
                .chain(added_fields)
                .collect::<Vec<_>>(),
        );

        quote! {
             #row_type_definition
            impl silo::ToRows<#row_type_name> for #row_type_name {
                 fn to_rows(self) -> Vec<#row_type_name> {
                     vec![self]
                 }
             }


            impl silo::FromRow for #row_type_name {
                fn try_from_row(string_storage: &mut silo::StaticStringStorage, row_name: Option<&'static str>, row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> Option<Self> {
                use silo::rusqlite::OptionalExtension;
                #(
                    let actual_column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#field_names_with_skips)])).unwrap_or(stringify!(#field_names_with_skips));
                    let #field_names_with_skips = <#field_types>::try_from_row(string_storage, Some(actual_column_name), row, connection)?;
                )*
                Some(Self {
                    #( #field_names_with_skips: #field_names_with_skips.into(),)*
                    #(#skipped_field_names: Default::default(),)*
                })
            }
        }

            impl silo::AsParams for #row_type_name {
                const PARAM_COUNT: usize = #(<#field_types as silo::AsParams>::PARAM_COUNT +)* 0;
                fn as_params(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
                    use silo::AsParams;
                    let mut result = Vec::new();
                    #(result.extend(&self.#field_names_with_skips.as_params()));*
                    ;
                    result
                }
            }

            impl<'a> silo::IntoSqlTable<'a> for #row_type_name {
                type Table = #table_name<'a>;
                const COLUMNS: &'static [silo::SqlColumn] = &silo::konst::slice::slice_concat!{silo::SqlColumn ,&[
                    #(#columns,)*
                ]};

                const NAME: &'static str = stringify!(#table_name);
            }


            #partial

            #migration_handler

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
            row_type_name,
            table_name,
            members,
            visibility,
            migration_handler,
            partial_name,
            has_vec_as_member,
            ..
        } = self;
        let field_names_with_skips: Vec<_> =
            members.iter().map(|c| c.create_field_name()).collect();
        let field_types_with_skips: Vec<_> =
            members.iter().map(|c| c.create_field_type()).collect();
        // let param_count = field_names_with_skips.len() + 1;
        // let param_count = LitInt::new(&format!("{param_count}usize"), row_type_name.span());

        let columns: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition(*has_vec_as_member))
            .collect();
        let variant_pattern = Member::create_variant_pattern(variants, &members);
        let variant_empty_columns_before =
            Member::create_variant_empty_columns_before(variants, &members);
        let variant_names = Member::create_variant_names(variants, &members);
        let variant_field_names = Member::create_variant_field_names(variants, &members);

        let columns_in_macro: Vec<_> = members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.create_column_definition_in_macro(*has_vec_as_member))
            .collect();
        let create_prefixed_columns_macro =
            format_ident!("column_names_with_prefix_for_{row_type_name}");

        let partial = create_partial::<false>(
            row_type_name,
            &[format_ident!("variant")],
            &[syn::parse_quote!(String)],
            &[],
            &[],
        );

        let partial_row_type_name = format_ident!("Partial{row_type_name}");
        quote! {
            #partial

            impl silo::ToRows<#partial_row_type_name> for #partial_name {
                fn to_rows(self) -> Vec<#partial_row_type_name> {
                    vec![self]
                }
            }

            impl silo::FromRow for #row_type_name {
                fn try_from_row(string_storage: &mut silo::StaticStringStorage,
                   row_name: Option<&'static str>,
                   row: &silo::rusqlite::Row,
                   connection: &silo::rusqlite::Connection,
               ) -> Option<Self> {
                    use silo::rusqlite::OptionalExtension;
                    let variant_name = row_name.map(|r| string_storage.store(&[r, "_variant"])).unwrap_or("variant");
                    let variant = String::try_from_row(string_storage, Some(variant_name), row, connection)?;
                    #(
                        let column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#field_names_with_skips)])).unwrap_or(stringify!(#field_names_with_skips));
                        let #field_names_with_skips = <#field_types_with_skips>::try_from_row(string_storage, Some(column_name), row, connection);)*
                    Some(match variant.as_str() {
                        #(stringify!(#variants) => {

                            #(let #variant_field_names = #variant_field_names?;)*
                            Self::#variant_pattern
                        })*
                        _ => {return None;}
                    })}
            }

            impl #row_type_name {
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

            impl silo::AsParams for #row_type_name {
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

           impl silo::RowType for #row_type_name {}

            impl<'a> silo::IntoSqlTable<'a> for #row_type_name {
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

            #migration_handler

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

// TODO: Make this more sensible!
fn create_row_type(
    name: &Ident,
    row_type_name: &Ident,
    table_name: &Ident,
    partial_name: &Ident,
    members: &Vec<Member>,
    visibility: &Visibility,
    filter_name: &Ident,
    field_names_with_skips: &mut Vec<proc_macro2::TokenStream>,
    mut field_names_without_skips: Vec<proc_macro2::TokenStream>,
    columns: &mut Vec<proc_macro2::TokenStream>,
    columns_in_macro: &mut Vec<proc_macro2::TokenStream>,
    mut partial_field_definitions: Vec<proc_macro2::TokenStream>,
    field_names: &mut Vec<Ident>,
    field_types: &mut Vec<Type>,
    added_fields: &mut Vec<Ident>,
    removed_fields_from_special_handling: &mut Vec<Ident>,
) -> proc_macro2::TokenStream {
    let primary_key_field = &members
        .iter()
        .find(|m| !m.is_skipped && m.is_primary)
        .expect("Checked, that this exists")
        .name;
    let cur_primary_key_field = format_ident!("cur_{primary_key_field}");
    let primary_key_type = &members
        .iter()
        .find(|m| !m.is_skipped && m.is_primary)
        .expect("Checked, that this exists")
        .type_;
    let has_primary_key_field = format_ident!("has_{primary_key_field}");

    removed_fields_from_special_handling.extend(
        members
            .iter()
            .filter(|m| !m.is_skipped && !m.is_primary)
            .map(|m| m.name.clone()),
    );
    columns_in_macro.clear();
    columns_in_macro.push(quote!(&[silo::SqlColumn {
        name: stringify!(#primary_key_field),
        r#type: <#primary_key_type as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE,
        is_unique: false,
        is_primary: true,
    }]));
    for vec_able_member in members.iter().filter(|m| !m.is_skipped && m.has_vec()) {
        let n = &vec_able_member.name;
        let remaining = format_ident!("{n}_silo_remaining_elements");
        field_names.push(remaining.clone());
        added_fields.push(remaining.clone());
        field_types.push(syn::parse_quote!(usize));
        field_names_with_skips.push(quote!(#remaining));
        field_names_without_skips.push(quote!(#remaining));
        partial_field_definitions.push(quote!(#remaining: Option<usize>));
        columns.push(quote!(&[silo::SqlColumn {
            name: stringify!(#remaining),
            r#type: <usize as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE,
            is_unique: false,
            is_primary: false,
        }]));
    }
    let row_type_fields = members.iter().filter(|m| !m.is_skipped).map(|m| {
        let n = &m.name;
        let is_vec = m.has_vec();
        let is_primary = m.is_primary;
        let t = m.create_field_type();
        if !is_vec && !is_primary {
            quote! {#n: Option<#t>,}
            // quote! {#n: <#t as silo::HasPartialRepresentation>::Partial,}
        } else if !is_vec {
            quote! {#n: #t,}
        } else {
            let remaining = format_ident!("{n}_silo_remaining_elements");
            quote! {#n: Option<#t>,
            #remaining: usize,}
        }
    });

    let iterable_field_names: Vec<_> = members
        .iter()
        .filter(|m| !m.is_skipped && m.has_vec())
        .map(|m| m.name.clone())
        .collect();
    let cur_iterable_fields: Vec<_> = iterable_field_names
        .iter()
        .map(|n| format_ident!("cur_{n}"))
        .collect();

    let iterable_remaining_names: Vec<_> = iterable_field_names
        .iter()
        .map(|n| format_ident!("{n}_silo_remaining_elements"))
        .collect();
    let iterable_lens: Vec<_> = iterable_field_names
        .iter()
        .map(|n| format_ident!("{n}_len"))
        .collect();
    let remaining_fields: Vec<_> = members
        .iter()
        .filter(|m| !m.is_skipped && !m.has_vec() && !m.is_primary)
        .map(|m| m.name.clone())
        .collect();
    let cur_remaining_fields: Vec<_> = remaining_fields
        .iter()
        .map(|f| format_ident!("cur_{f}"))
        .collect();

    let iterable_fields_as_iterator = members
        .iter()
        .filter(|m| !m.is_skipped && m.has_vec())
        .fold(proc_macro2::TokenStream::new(), |acc, cur| {
            let name = cur.create_field_name();
            if acc.is_empty() {
                quote!(self.#name.into_iter().map(Some).enumerate().chain(std::iter::repeat((usize::MAX, None))))
            } else {
                quote!(#acc.zip(self.#name.into_iter().map(Some).enumerate().chain(std::iter::repeat((usize::MAX, None)))))
            }
        });
    let iterable_fields_as_pattern_match = members
        .iter()
        .filter(|m| !m.is_skipped && m.has_vec())
        .fold(proc_macro2::TokenStream::new(), |acc, cur| {
            let name = cur.create_field_name();
            let remaining = format_ident!("{}_silo_remaining_elements", cur.name);
            if acc.is_empty() {
                quote!((#remaining, #name))
            } else {
                quote!((#acc, (#remaining, #name)))
            }
        });

    let column_macro_name = format_ident!("column_names_with_prefix_for_{name}");

    let partial = create_partial::<true>(
        name,
        &members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| m.name.clone())
            .collect::<Vec<_>>(),
        &members
            .iter()
            .filter(|m| !m.is_skipped)
            .map(|m| {
                // if m.has_vec() || m.is_primary {
                m.type_.clone()
                // } else {
                //     let t = &m.type_;
                //     syn::parse_quote!(Option<#t>)
                // }
            })
            .collect::<Vec<_>>(),
        &members
            .iter()
            .filter(|m| m.is_skipped)
            .map(|m| m.name.clone())
            .collect::<Vec<_>>(),
        &members
            .iter()
            .filter(|m| !m.is_skipped && !type_is_option(&m.type_))
            .map(|m| m.name.clone())
            .collect::<Vec<_>>(),
    );

    // let partial_normal_name = format_ident!("Partial{name}");

    let partial_row_type_name = format_ident!("Partial{row_type_name}");
    quote! {
        #partial

        impl silo::ToRows<#partial_row_type_name> for #partial_name {
            fn to_rows(self) -> Vec<#partial_row_type_name> {
                let mut result = Vec::new();
                #(let mut #remaining_fields = self.#remaining_fields;)*
                #(let #iterable_lens = self.#iterable_field_names.len();)*
                let len = #((#iterable_lens).max)*(0);
                for #iterable_fields_as_pattern_match in #iterable_fields_as_iterator.take(len) {
                    result.push(#partial_row_type_name {
                        #primary_key_field: self.#primary_key_field.clone(),
                        #(#remaining_fields: #remaining_fields.clone(),)*
                        #(#iterable_field_names,)*
                        #(#iterable_remaining_names: Some(#iterable_lens.saturating_sub(#iterable_remaining_names)),)*
                    });
                }
                result
            }
        }

        #[allow(unused_macros)]
        macro_rules! #column_macro_name {
            ($prefix:expr) => {
                [silo::SqlColumn {
                    name: concat!($prefix, "_", stringify!(#primary_key_field)),
                    r#type: <#primary_key_type as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE, is_unique: false, is_primary: false,
                }]
            }
        }

        #[derive(Clone, Debug)]
        #visibility struct #row_type_name {
            #(#row_type_fields)*
        }

        impl silo::Filterable for #name {
            type Filtered = #filter_name;
            fn must_be_equal(&self) -> #filter_name {
                <#filter_name as Default>::default().#has_primary_key_field(self.#primary_key_field.clone())
            }

            fn must_contain(&self) -> #filter_name {
                <#filter_name as Default>::default()
            }
        }

        impl Into<Option<#primary_key_type>> for #name {
            fn into(self) -> <#primary_key_type as silo::HasPartialRepresentation>::Partial {
                self.#primary_key_field.into()
            }
        }

        impl silo::AsParams for #name {
            const PARAM_COUNT: usize = 1;

            fn as_params<'b>(&'b self) -> Vec<&'b dyn silo::rusqlite::ToSql> {
                self.#primary_key_field.as_params()
            }
        }

        impl silo::FromRow for #name {
            fn try_from_row(
                string_storage: &mut silo::StaticStringStorage,
                row_name: Option<&'static str>,
                row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> Option<Self> {
                use silo::SqlTable;
                let actual_column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#primary_key_field)])).unwrap_or(stringify!(#primary_key_field));
                let #primary_key_field = <<#primary_key_type as silo::HasPartialRepresentation>::Partial>::try_from_row(string_storage, Some(actual_column_name), row, connection);
                dbg!(&#primary_key_field);
                let #primary_key_field = #primary_key_field??;
                let db = unsafe { silo::Database::from_connection(connection) }.ok()?;
                let table = db.load::<#name>().ok()?;

                let elements = table.filter(#filter_name::default().#has_primary_key_field(#primary_key_field.clone()));
                dbg!(&elements);
                elements.ok()?.into_iter().next()
            }
        }

        impl silo::FromRow for #partial_name {
            fn try_from_row(
                string_storage: &mut silo::StaticStringStorage,
                row_name: Option<&'static str>,
                row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> Option<Self> {
                use silo::SqlTable;
                let actual_column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#primary_key_field)])).unwrap_or(stringify!(#primary_key_field));
                let #primary_key_field = <<#primary_key_type as silo::HasPartialRepresentation>::Partial>::try_from_row(string_storage, Some(actual_column_name), row, connection);
                dbg!(&#primary_key_field);
                let #primary_key_field = #primary_key_field??;
                let db = unsafe { silo::Database::from_connection(connection) }.ok()?;
                let table = db.load::<#name>().ok()?;

                let elements = table.filter(#filter_name::default().#has_primary_key_field(#primary_key_field.clone()));
                dbg!(&elements);
                elements.ok()?.into_iter().next().map(Into::into)
            }
        }

        impl silo::FromRowType<#row_type_name> for #name {
            fn from_row_type(mut values: Vec<#row_type_name>) -> Vec<#name> {
                if values.is_empty() {
                    return Vec::new();
                }
                let mut result = Vec::new();
                let mut #cur_primary_key_field = values[0].#primary_key_field.clone();
                #(let mut #cur_remaining_fields = values[0]
                    .#remaining_fields
                    .clone()
                    .expect("First value of a vec should be set!");)*
                #(let mut #cur_iterable_fields = Vec::new();)*
                while let Some(value) = values.pop() {
                    if #cur_primary_key_field == value.#primary_key_field {
                        #(
                            if let Some(value) = value.#iterable_field_names {
                                #cur_iterable_fields.push(value);
                            }
                        )*
                        continue;
                    }
                    #(#cur_iterable_fields.reverse();)*
                    result.push(#name {
                        #primary_key_field: #cur_primary_key_field,
                        #(#iterable_field_names: #cur_iterable_fields,)*
                        #(#remaining_fields: #cur_remaining_fields.clone(),)*
                    });
                    #cur_primary_key_field = value.#primary_key_field.clone();
                    #(#cur_remaining_fields = value
                        .#remaining_fields
                        .clone()
                        .expect("First value of a vec should be set!");)*
                    #(#cur_iterable_fields = Vec::new();)*
                }
                if #(!#cur_iterable_fields.is_empty() ||)* false {
                    #(#cur_iterable_fields.reverse();)*
                    result.push(#name {
                        #primary_key_field: #cur_primary_key_field,
                        #(#iterable_field_names: #cur_iterable_fields,)*
                        #(#remaining_fields: #cur_remaining_fields.clone(),)*
                    });

                }
                result
            }
        }

        impl silo::ToRows<#row_type_name> for #name {
            fn to_rows(self) -> Vec<#row_type_name> {
                let mut result = Vec::new();
                #(let mut #remaining_fields = Some(self.#remaining_fields);)*
                #(let #iterable_lens = self.#iterable_field_names.len();)*
                let len = #((#iterable_lens).max)*(0);
                for #iterable_fields_as_pattern_match in #iterable_fields_as_iterator.take(len) {
                    result.push(#row_type_name {
                        #primary_key_field: self.#primary_key_field.clone(),
                        #(#remaining_fields: #remaining_fields.take(),)*
                        #(#iterable_field_names,)*
                        #(#iterable_remaining_names: #iterable_lens.saturating_sub(#iterable_remaining_names),)*
                    });
                }
                result
            }
        }


        impl<'a> silo::IntoSqlTable<'a> for #name {
            type Table = #table_name<'a>;
            const COLUMNS: &'static [silo::SqlColumn] =
                &[silo::SqlColumn {
                    name: stringify!(#primary_key_field),
                    r#type: <#primary_key_type as silo::RelatedSqlColumnType>::SQL_COLUMN_TYPE,
                    is_primary: true,
                    is_unique: false,
                }];

            const NAME: &'static str = stringify!(#table_name);
        }

        impl silo::RowType for #row_type_name {
            fn insert_into_connected_foreign_tables(
                self,
                is_top_level: bool,
                connection: &silo::rusqlite::Connection,
            ) -> silo::rusqlite::Result<()> {
                use silo::SqlTable;
                if is_top_level {
                    return Ok(())
                }
                let db = unsafe { silo::Database::from_connection(connection) }?;
                let table = db.load::<#name>()?;
                table.insert(self)?;
                Ok(())
            }
        }

        impl silo::RowType for #name {
            fn insert_into_connected_foreign_tables(
                self,
                is_top_level: bool,
                connection: &silo::rusqlite::Connection,
            ) -> silo::rusqlite::Result<()> {
                use silo::SqlTable;
                if is_top_level {
                    return Ok(());
                }
                let db = unsafe { silo::Database::from_connection(connection) }?;
                let table = db.load::<#name>()?;
                table.insert(self)?;
                Ok(())
            }
        }
    }
}

impl ToTokens for Base {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        if !self.errors.is_empty() {
            tokens.extend(self.errors.clone());
            return;
        }
        let filter = self.create_filter();
        tokens.extend(filter);
        let table = self.create_table();
        tokens.extend(table);
        let conversions = self.create_conversions();
        tokens.extend(conversions);
    }
}

fn create_partial<const IS_STRUCT: bool>(
    name: &syn::Ident,
    field_names: &[syn::Ident],
    field_types: &[syn::Type],
    unrepresented_fields: &[syn::Ident],
    special_handling_fields_for_transpose: &[syn::Ident],
) -> proc_macro2::TokenStream {
    let partial_name = format_ident!("Partial{name}");
    let partial_fields = field_names
        .iter()
        .zip(field_types.iter())
        .map(|(n, t)| quote!(#n: <#t as silo::HasPartialRepresentation>::Partial,));
    let from_field_conversion = field_names.iter().zip(field_types.iter()).map(|(n, t)| {
        if type_is_vec(t) {
            quote!(#n.into_iter().map(Into::into).collect())
        } else if type_is_option(t) {
            quote!(#n.map(Into::into))
        } else {
            quote!(#n.into())
        }
    });

    let from_impl = if IS_STRUCT {
        quote! {
            impl From<#name> for #partial_name {
                fn from(value: #name) -> #partial_name {
                    #partial_name {
                        #(#field_names: value.#from_field_conversion,)*
                    }
                }
            }
        }
    } else {
        quote! {
            impl From<#name> for #partial_name {
                fn from(value: #name) -> #partial_name {
                    let variant = *value.variant_name();
                    #partial_name {
                        variant: Some(variant.into()),
                    }
                }
            }
        }
    };

    let partial_type_impl = if IS_STRUCT {
        quote! {
            impl silo::PartialType<#name> for #partial_name {
                fn transpose(self) -> Option<#name> {
                    #(let #field_names = self.#field_names.transpose();)*
                    #(let #special_handling_fields_for_transpose = #special_handling_fields_for_transpose?;)*
                    #(let #unrepresented_fields = Default::default();)*
                    Some(#name {
                        #(#field_names,)*
                        #(#unrepresented_fields,)*
                    })
                }
            }
        }
    } else {
        quote! {
            impl silo::PartialType<#name> for #partial_name {
                fn transpose(self) -> Option<#name> {
                    let variant = self.variant?;
                    // TODO: Select right variant here!
                    None
                }
            }
        }
    };

    let from_row_impl = if field_types.iter().any(|t| type_is_vec(t)) {
        quote! {}
    } else {
        quote! {
            impl silo::FromRow for #partial_name {
            fn try_from_row(
                string_storage: &mut silo::StaticStringStorage,
                row_name: Option<&'static str>,
                row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> Option<Self> {
                use silo::rusqlite::OptionalExtension;
                #(
                    let actual_column_name = row_name.map(|r| string_storage.store(&[r, "_", stringify!(#field_names)])).unwrap_or(stringify!(#field_names));
                    let #field_names = <<#field_types as silo::HasPartialRepresentation>::Partial>::try_from_row(string_storage, Some(actual_column_name), row, connection)?;
                )*
                Some(Self {
                    #( #field_names),*
                })
            }
        }

        }
    };

    let as_params_impl = if !IS_STRUCT {
        quote! {

            impl silo::AsParams for #partial_name {
                const PARAM_COUNT: usize = #(<#field_types as silo::AsParams>::PARAM_COUNT +)* 0;
                fn as_params(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
                    use silo::AsParams;
                    let mut result = Vec::new();
                    #(result.extend(&self.#special_handling_fields_for_transpose.as_params()));*
                    ;
                    result
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #[derive(Default, Debug, Clone)]
        pub struct #partial_name {
            #(pub #partial_fields)*
        }

        impl silo::HasPartialRepresentation for #name {
            type Partial = #partial_name;
        }

        #from_impl
        #partial_type_impl
        #from_row_impl
        #as_params_impl

        impl From<Option<#name>> for #partial_name {
            fn from(value: Option<#name>) -> #partial_name {
                match value {
                    Some(value) => value.into(),
                    None => Default::default(),
                }
            }
        }

        impl From<#partial_name> for Option<#name> {
            fn from(value: #partial_name) -> Option<#name> {
                use silo::PartialType;
                value.transpose()
            }
        }

        impl silo::HasValue for #partial_name {
            fn has_values(&self) -> bool {
                #(self.#field_names.has_values() ||)* false
            }
        }

        impl silo::PartialRow for #partial_name {
            fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
                use silo::HasValue;
                let mut result = Vec::new();
                #(if self.#field_names.has_values() {
                    result.append(
                        &mut self.#field_names.used_column_names(
                            Some(column_name.as_ref().map(|c| format!("{c}_{}", stringify!(#field_names)))
                                .unwrap_or_else(|| stringify!(#field_names).to_string()))));
                })*
                result
            }

            fn used_values(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
                let mut result = Vec::new();
                #(result.append(&mut self.#field_names.used_values());)*
                result
            }
        }
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
